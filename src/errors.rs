use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("user has already claimed drop")]
    ClaimedDrop,
    #[msg("mint limit reached.")]
    MintLimitReached,
    
}