use anchor_lang::prelude::*;

#[account]
pub struct Drop {
    /// claim id
    pub claim_id: String, 

    /// drop name
    pub name: String,

    /// drop description
    pub description: String,

    /// creator 
    pub creator: Pubkey,

    /// collection
    pub collection: Pubkey,

    pub infinite: bool,

    /// maxiumum number of drop mints
    pub left_mint: u64,

    pub ticker: String,
    pub uri: String, 

    /// nonce
    pub drop_nonce: u8,

    pub signer_nonce: u8,
}

#[account]
pub struct Claimed {
    pub minted: bool,

    pub master_mint: Pubkey,
    pub master_metadata: Pubkey,
    pub master_edition: Pubkey,

    pub nonce: u8,
}