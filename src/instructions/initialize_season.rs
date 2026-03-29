// use quasar_lang::prelude::*;
// use quasar_spl::{Mint, Token};
//
// use crate::state::{config::Config, season::Season};
//
// #[derive(Accounts)]
// #[instruction(epoch: u64)]
// pub struct InitializeSeason<'info> {
//     #[account(
//         mut,
//         seeds = [b"config"],
//         bump,
//     )]
//     pub config: &'info mut Account<Config>,
//
//     #[account(
//         init,
//         payer = authority,
//         space = 8 + Season::SPACE,
//         seeds = [b"season", epoch.to_le_bytes().as_ref()],
//         bump,
//     )]
//     pub season: &'info mut Account<Season>,
//
//     #[account(
//         init,
//         payer = authority,
//         token::mint = jitosol_mint,
//         token::authority = vault,
//         seeds = [b"vault", epoch.to_le_bytes().as_ref()],
//         bump,
//     )]
//     pub vault: &'info mut Account<Token>,
//
//     pub jitosol_mint: &'info Account<Mint>,
//
//     #[account(mut)]
//     pub authority: &'info Signer,
//
//     /// System program
//     pub system_program: &'info Program<System>,
//
//     /// Token program
//     pub token_program: &'info Program<Token>,
// }
//
// impl<'info> InitializeSeason<'info> {
//     #[inline(always)]
//     pub fn initialize_season(&mut self, bumps: &InitializeSeasonBumps) -> Result<(), ProgramError> {
//         self.season.authority = *self.authority.to_account_view().address();
//         self.season.bump = bumps.season;
//
//         // Increment season count
//         let count = u64::from(self.config.season_count);
//         self.config.season_count = PodU64::from(
//             count
//                 .checked_add(1)
//                 .ok_or(ProgramError::ArithmeticOverflow)?,
//         );
//
//         Ok(())
//     }
// }
//
