use quasar_lang::prelude::*;

use crate::state::season_status::SeasonStatus;

#[account(discriminator = 1)]
pub struct Season {
    /// Authority
    pub authority: Address,

    /// Vault
    pub vault: Address,

    /// Epoch start
    pub epoch_start: PodU64,

    /// Epoch end
    pub epoch_end: PodU64,

    /// Entry fee
    pub entry_fee: PodU64,

    /// Prize pool
    pub prize_pool: PodU64,

    /// Total entries
    pub total_entries: PodU32,

    /// Roster size
    pub roster_size: u8,

    /// Season status
    pub season_status: SeasonStatus,

    /// Bump
    pub bump: u8,

    /// Reserved for future use
    pub reserved: [u8; 128],
}

impl Season {
    pub const SPACE: usize = 32 + 32 + 8 + 8 + 8 + 8 + 4 + 1 + 1 + 1 + 128;
}
