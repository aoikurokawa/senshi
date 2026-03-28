#![cfg_attr(not(test), no_std)]

use quasar_lang::prelude::*;

pub mod instructions;

declare_id!("7FrNtPLQ1hSpWzBWzNtQiuDhrsp27oVkwo63o8emKFx7");

#[program]
mod my_program {
    use crate::instructions::initialize::Initialize;

    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize(ctx: Ctx<Initialize>) -> Result<(), ProgramError> {
        ctx.accounts.initialize()
    }
}

#[cfg(test)]
mod tests;
