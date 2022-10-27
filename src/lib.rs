use anchor_lang::prelude::*;
pub mod errors;
pub mod instructions;
pub mod state;
// pub use errors::ErrorCode;

pub use instructions::*;
pub use state::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod drop {
    use super::*;

    pub fn initialize_drop<'info>(ctx: Context<'_, '_, '_, 'info, Initialize<'info>>, claim_id: String, name: String, ticker: String, uri: String, description: String, maximum_mint: u64) -> Result<()> {
        initialize_drop::handler(ctx, claim_id, name, ticker, uri, description, maximum_mint)
    }

    pub fn claim_drop<'info>(ctx: Context<'_, '_, '_, 'info, ClaimDrop<'info>>, claim_id: String) -> Result<()> {
        claim_drop::handler(ctx, claim_id)
    }
}