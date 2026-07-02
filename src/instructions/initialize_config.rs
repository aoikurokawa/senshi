use quasar_lang::prelude::*;

use crate::state::config::Config;

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Config::SPACE,
        seeds = [b"config"],
        bump,
    )]
    pub config: &'info mut Account<Config>,

    #[account(mut)]
    pub payer: &'info mut Signer,

    pub system_program: &'info Program<System>,
}

impl<'info> InitializeConfig<'info> {
    #[inline(always)]
    pub fn initialize(&mut self, bumps: &InitializeConfigBumps) -> Result<(), ProgramError> {
        self.config.authority = *self.payer.to_account_view().address();
        self.config.season_count = PodU64::from(0);
        self.config.bump = bumps.config;
        Ok(())
    }
}
