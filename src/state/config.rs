use quasar_lang::prelude::*;

#[account(discriminator = 2)]
pub struct Config {
    /// Authority
    pub authority: Address,

    /// Season count
    pub season_count: PodU64,

    /// Bump
    pub bump: u8,

    /// Reserved for future use
    pub reserved: [u8; 128],
}

impl Config {
    pub const SPACE: usize = 32 + 8 + 1 + 128;
}
