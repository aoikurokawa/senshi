#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SeasonStatus {
    Open,
    Locked,
    Scoring,
    Settled,
}
