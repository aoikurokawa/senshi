#![cfg_attr(not(test), no_std)]

use quasar_lang::prelude::*;

pub mod instructions;
pub mod state;

declare_id!("7FrNtPLQ1hSpWzBWzNtQiuDhrsp27oVkwo63o8emKFx7");

#[program]
mod my_program {
    use crate::instructions::initialize_config::InitializeConfig;

    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize(ctx: Ctx<InitializeConfig>) -> Result<(), ProgramError> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    // #[instruction(discriminator = 1)]
    // pub fn initialize_season(ctx: Ctx<InitializeSeason>) -> Result<(), ProgramError> {
    //     ctx.accounts.initialize_season(&ctx.bumps)
    // }
}

#[cfg(test)]
mod tests;
