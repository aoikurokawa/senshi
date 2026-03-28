## Senshi

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("VLEAGUExxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");

#[program]
pub mod validator_league {
    use super::*;

    /// Initialize a new competition season
    pub fn create_season(
        ctx: Context<CreateSeason>,
        season_id: u64,
        entry_fee: u64,
        roster_size: u8,
        target_epoch: u64,
    ) -> Result<()> {
        require!(roster_size > 0 && roster_size <= 10, VLError::InvalidRosterSize);
        require!(entry_fee > 0, VLError::InvalidEntryFee);

        let season = &mut ctx.accounts.season;
        season.season_id = season_id;
        season.entry_fee = entry_fee;
        season.roster_size = roster_size;
        season.status = SeasonStatus::Open;
        season.epoch_start = target_epoch;
        season.epoch_end = target_epoch; // single-epoch for POC
        season.total_entries = 0;
        season.vault = ctx.accounts.vault.key();
        season.prize_pool = 0;
        season.authority = ctx.accounts.authority.key();
        season.bump = ctx.bumps.season;

        emit!(SeasonCreated {
            season_id,
            entry_fee,
            roster_size,
            target_epoch,
        });

        Ok(())
    }

    /// Player enters a season with their validator roster
    pub fn enter_season(
        ctx: Context<EnterSeason>,
        season_id: u64,
        validators: Vec<Pubkey>,
    ) -> Result<()> {
        let season = &mut ctx.accounts.season;

        require!(season.status == SeasonStatus::Open, VLError::SeasonNotOpen);
        require!(
            validators.len() == season.roster_size as usize,
            VLError::InvalidRosterSize
        );

        // Check no duplicate validators
        let mut sorted = validators.clone();
        sorted.sort();
        for i in 1..sorted.len() {
            require!(sorted[i] != sorted[i - 1], VLError::DuplicateValidator);
        }

        // Transfer JitoSOL entry fee to vault
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.player_jitosol.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.player.to_account_info(),
                },
            ),
            season.entry_fee,
        )?;

        // Initialize entry
        let entry = &mut ctx.accounts.entry;
        entry.season_id = season_id;
        entry.player = ctx.accounts.player.key();
        entry.validators = pad_validators(validators, season.roster_size);
        entry.score = None;
        entry.reward = None;
        entry.claimed = false;
        entry.bump = ctx.bumps.entry;

        season.total_entries += 1;
        season.prize_pool += season.entry_fee;

        emit!(EntrySubmitted {
            season_id,
            player: ctx.accounts.player.key(),
            validators: entry.validators.to_vec(),
        });

        Ok(())
    }

    /// Keeper locks the season when the target epoch begins
    pub fn lock_season(ctx: Context<KeeperAction>, _season_id: u64) -> Result<()> {
        let season = &mut ctx.accounts.season;
        require!(season.status == SeasonStatus::Open, VLError::InvalidTransition);

        let clock = Clock::get()?;
        require!(clock.epoch >= season.epoch_start, VLError::EpochNotReached);

        season.status = SeasonStatus::Locked;

        emit!(SeasonLocked {
            season_id: season.season_id,
            epoch: clock.epoch,
            total_entries: season.total_entries,
        });

        Ok(())
    }

    /// Keeper submits scores for a batch of entries
    pub fn submit_scores(
        ctx: Context<SubmitScores>,
        _season_id: u64,
        entries: Vec<Pubkey>,
        scores: Vec<u64>,
    ) -> Result<()> {
        let season = &ctx.accounts.season;
        require!(
            season.status == SeasonStatus::Locked || season.status == SeasonStatus::Scoring,
            VLError::InvalidTransition
        );
        require!(entries.len() == scores.len(), VLError::LengthMismatch);

        // Note: In practice, remaining_accounts would contain the entry PDAs
        // and we'd iterate + set scores. Simplified here for sketch purposes.
        // The keeper would call this in batches of ~20 entries per tx.

        emit!(ScoresSubmitted {
            season_id: season.season_id,
            count: entries.len() as u32,
        });

        Ok(())
    }

    /// Keeper settles the season — computes reward tiers and enables claims
    pub fn settle_season(
        ctx: Context<KeeperAction>,
        _season_id: u64,
    ) -> Result<()> {
        let season = &mut ctx.accounts.season;
        require!(
            season.status == SeasonStatus::Scoring,
            VLError::InvalidTransition
        );

        let clock = Clock::get()?;
        require!(clock.epoch > season.epoch_end, VLError::EpochNotEnded);

        // Prize pool = vault balance (includes accrued JitoSOL yield)
        season.prize_pool = ctx.accounts.vault.amount;
        season.status = SeasonStatus::Settled;

        emit!(SeasonSettled {
            season_id: season.season_id,
            prize_pool: season.prize_pool,
            total_entries: season.total_entries,
        });

        Ok(())
    }

    /// Player claims their reward after settlement
    pub fn claim_reward(ctx: Context<ClaimReward>, season_id: u64) -> Result<()> {
        let entry = &mut ctx.accounts.entry;
        let season = &ctx.accounts.season;

        require!(season.status == SeasonStatus::Settled, VLError::NotSettled);
        require!(!entry.claimed, VLError::AlreadyClaimed);
        require!(entry.reward.is_some(), VLError::NoReward);

        let reward = entry.reward.unwrap();
        entry.claimed = true;

        // Transfer from vault PDA to player
        let seeds = &[
            b"vault",
            season_id.to_le_bytes().as_ref(),
            &[ctx.accounts.season.bump],
        ];
        let signer = &[&seeds[..]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.player_jitosol.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer,
            ),
            reward,
        )?;

        emit!(RewardClaimed {
            season_id,
            player: ctx.accounts.player.key(),
            reward,
        });

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Account Structs
// ---------------------------------------------------------------------------

#[account]
#[derive(Debug)]
pub struct Season {
    pub season_id: u64,
    pub entry_fee: u64,
    pub roster_size: u8,
    pub status: SeasonStatus,
    pub epoch_start: u64,
    pub epoch_end: u64,
    pub total_entries: u32,
    pub vault: Pubkey,
    pub prize_pool: u64,
    pub authority: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct Entry {
    pub season_id: u64,
    pub player: Pubkey,
    pub validators: [Pubkey; 10], // max roster size, unused slots = Pubkey::default()
    pub score: Option<u64>,
    pub reward: Option<u64>,
    pub claimed: bool,
    pub bump: u8,
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeasonStatus {
    Open,
    Locked,
    Scoring,
    Settled,
}

// ---------------------------------------------------------------------------
// Contexts
// ---------------------------------------------------------------------------

#[derive(Accounts)]
#[instruction(season_id: u64)]
pub struct CreateSeason<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<Season>(),
        seeds = [b"season", season_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub season: Account<'info, Season>,

    #[account(
        init,
        payer = authority,
        token::mint = jitosol_mint,
        token::authority = vault, // self-authority for PDA
        seeds = [b"vault", season_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub jitosol_mint: Account<'info, token::Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(season_id: u64)]
pub struct EnterSeason<'info> {
    #[account(
        mut,
        seeds = [b"season", season_id.to_le_bytes().as_ref()],
        bump = season.bump,
    )]
    pub season: Account<'info, Season>,

    #[account(
        init,
        payer = player,
        space = 8 + std::mem::size_of::<Entry>(),
        seeds = [b"entry", season_id.to_le_bytes().as_ref(), player.key().as_ref()],
        bump,
    )]
    pub entry: Account<'info, Entry>,

    #[account(
        mut,
        constraint = vault.key() == season.vault,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = player_jitosol.owner == player.key(),
        constraint = player_jitosol.mint == jitosol_mint.key(),
    )]
    pub player_jitosol: Account<'info, TokenAccount>,

    pub jitosol_mint: Account<'info, token::Mint>,

    #[account(mut)]
    pub player: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(season_id: u64)]
pub struct KeeperAction<'info> {
    #[account(
        mut,
        seeds = [b"season", season_id.to_le_bytes().as_ref()],
        bump = season.bump,
        has_one = authority,
    )]
    pub season: Account<'info, Season>,

    #[account(
        mut,
        constraint = vault.key() == season.vault,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(season_id: u64)]
pub struct SubmitScores<'info> {
    #[account(
        mut,
        seeds = [b"season", season_id.to_le_bytes().as_ref()],
        bump = season.bump,
        has_one = authority,
    )]
    pub season: Account<'info, Season>,

    pub authority: Signer<'info>,
    // remaining_accounts: Entry PDAs to score
}

#[derive(Accounts)]
#[instruction(season_id: u64)]
pub struct ClaimReward<'info> {
    #[account(
        seeds = [b"season", season_id.to_le_bytes().as_ref()],
        bump = season.bump,
    )]
    pub season: Account<'info, Season>,

    #[account(
        mut,
        seeds = [b"entry", season_id.to_le_bytes().as_ref(), player.key().as_ref()],
        bump = entry.bump,
        has_one = player,
    )]
    pub entry: Account<'info, Entry>,

    #[account(
        mut,
        constraint = vault.key() == season.vault,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// CHECK: PDA authority for vault
    #[account(
        seeds = [b"vault", season_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = player_jitosol.owner == player.key(),
    )]
    pub player_jitosol: Account<'info, TokenAccount>,

    pub player: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[event]
pub struct SeasonCreated {
    pub season_id: u64,
    pub entry_fee: u64,
    pub roster_size: u8,
    pub target_epoch: u64,
}

#[event]
pub struct EntrySubmitted {
    pub season_id: u64,
    pub player: Pubkey,
    pub validators: Vec<Pubkey>,
}

#[event]
pub struct SeasonLocked {
    pub season_id: u64,
    pub epoch: u64,
    pub total_entries: u32,
}

#[event]
pub struct ScoresSubmitted {
    pub season_id: u64,
    pub count: u32,
}

#[event]
pub struct SeasonSettled {
    pub season_id: u64,
    pub prize_pool: u64,
    pub total_entries: u32,
}

#[event]
pub struct RewardClaimed {
    pub season_id: u64,
    pub player: Pubkey,
    pub reward: u64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[error_code]
pub enum VLError {
    #[msg("Season is not open for entries")]
    SeasonNotOpen,
    #[msg("Invalid roster size")]
    InvalidRosterSize,
    #[msg("Invalid entry fee")]
    InvalidEntryFee,
    #[msg("Duplicate validator in roster")]
    DuplicateValidator,
    #[msg("Invalid status transition")]
    InvalidTransition,
    #[msg("Target epoch not yet reached")]
    EpochNotReached,
    #[msg("Competition epoch has not ended")]
    EpochNotEnded,
    #[msg("Season not yet settled")]
    NotSettled,
    #[msg("Reward already claimed")]
    AlreadyClaimed,
    #[msg("No reward to claim")]
    NoReward,
    #[msg("Array length mismatch")]
    LengthMismatch,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pad_validators(validators: Vec<Pubkey>, roster_size: u8) -> [Pubkey; 10] {
    let mut padded = [Pubkey::default(); 10];
    for (i, v) in validators.iter().enumerate().take(roster_size as usize) {
        padded[i] = *v;
    }
    padded
}
```
