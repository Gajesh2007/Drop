use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use anchor_spl::metadata::{
    self, CreateMasterEditionV3, CreateMetadataAccountsV3, Metadata, 
};
use anchor_spl::associated_token::{AssociatedToken};
use mpl_token_metadata::state::{DataV2, Creator};

#[derive(Accounts)]
#[instruction(claim_id: String)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init,
        seeds=[
            claim_id.as_ref(),
        ],
        payer=signer,
        space = 272 + 8,
        bump
    )]
    pub drop: Box<Account<'info, Drop>>,

    #[account(
        seeds = [
            drop.key().as_ref()
        ],
        bump
    )]
    pub head_signer: UncheckedAccount<'info>,

    #[account(
        init,
        payer = signer,
        seeds = [
            b"collection",
            drop.key().as_ref(),
        ],
        bump,
        mint::authority = head_signer,
        mint::freeze_authority = head_signer,
        mint::decimals = 0,
    )]
    pub collection_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = signer,
        associated_token::authority = head_signer,
        associated_token::mint = collection_mint,
    )]
    pub master_token: Box<Account<'info, TokenAccount>>,

    /// CHECK: Account allocation and initialization is done via CPI to the metadata program.
    #[account(
        mut,
        seeds = [
            "metadata".as_bytes(),
            metadata_program.key().as_ref(),
            collection_mint.key().as_ref(),
        ],
        seeds::program = metadata_program.key(),
        bump,
    )]
    pub master_metadata: UncheckedAccount<'info>,

    /// CHECK: Account allocation and initialization is done via CPI to the metadata program.
    #[account(
        mut,
        seeds = [
            "metadata".as_bytes(),
            metadata_program.key().as_ref(),
            collection_mint.key().as_ref(),
            "edition".as_bytes(),
        ],
        seeds::program = metadata_program.key(),
        bump,
    )]
    pub master_edition: UncheckedAccount<'info>,

    pub system_program: Program<'info,System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub metadata_program: Program<'info, Metadata>,
    pub rent: Sysvar<'info, Rent>
}

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, Initialize<'info>>,
    claim_id: String, 
    name: String, 
    ticker: String,
    uri: String,
    description: String, 
    maximum_mint: u64
) -> Result<()> {
    let drop = &mut ctx.accounts.drop;
    let drop_pda = drop.key();

    let signer_nonce = *ctx.bumps.get("head_signer").unwrap();

    //
    // Mint the master token.
    //
    {
        let seeds = &[drop_pda.as_ref(), &[signer_nonce]];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.collection_mint.to_account_info(),
                to: ctx.accounts.master_token.to_account_info(),
                authority: ctx.accounts.head_signer.to_account_info(), 
            },
            signer
        );

        token::mint_to(
            cpi_ctx,
            1,
        )?;
    }

    {
        let seeds = &[drop_pda.as_ref(), &[signer_nonce]];
        let signer = &[&seeds[..]];
        let is_mutable = true;
        let update_authority_is_signer = true;

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.master_metadata.to_account_info(),
                mint: ctx.accounts.collection_mint.to_account_info(),
                mint_authority: ctx.accounts.head_signer.to_account_info(),
                payer: ctx.accounts.signer.to_account_info(),
                update_authority: ctx.accounts.head_signer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            signer
        );

        metadata::create_metadata_accounts_v3(
            cpi_ctx, 
            DataV2 {
                name: name.to_string(),
                symbol: ticker.to_string(),
                uri: uri.to_string(),
                seller_fee_basis_points: 1000,
                creators: Some(vec![
                    Creator {
                        address: ctx.accounts.head_signer.key(),
                        share: 0,
                        verified: true
                    },
                    Creator {
                        address: ctx.accounts.signer.key(),
                        verified: false,
                        share: 100
                    }
                ]),
                collection: None,
                uses: None,
            },
            is_mutable,
            update_authority_is_signer,
            None,
        )?;
    }

    //
    // Create master edition.
    //
    {
        let seeds = &[drop_pda.as_ref(), &[signer_nonce]];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.metadata_program.to_account_info(),
            CreateMasterEditionV3 {
                edition: ctx.accounts.master_edition.to_account_info(),
                mint: ctx.accounts.collection_mint.to_account_info(),
                update_authority: ctx.accounts.head_signer.to_account_info(),
                mint_authority: ctx.accounts.head_signer.to_account_info(),
                payer: ctx.accounts.signer.to_account_info(),
                metadata: ctx.accounts.master_metadata.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            signer
        );

        metadata::create_master_edition_v3(
            cpi_ctx, 
            Some(0)
        )?;
    }

    drop.claim_id = claim_id;
    drop.name = name;
    drop.description = description;
    drop.creator = ctx.accounts.signer.key();
    drop.collection = ctx.accounts.collection_mint.key();
    drop.left_mint = maximum_mint;

    if maximum_mint == 0 {
        drop.infinite = true;
        msg!("infinite mint");
    }

    drop.drop_nonce = *ctx.bumps.get("drop").unwrap();
    drop.signer_nonce = signer_nonce;
    drop.uri = uri;
    drop.ticker = ticker;

    Ok(())
}