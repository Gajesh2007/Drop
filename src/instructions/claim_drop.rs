use crate::{state::*};
use anchor_lang::prelude::*;
use anchor_spl::metadata::{
    self, CreateMasterEditionV3, CreateMetadataAccountsV3, Metadata, VerifyCollection 
};
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use anchor_spl::associated_token::{AssociatedToken};
use mpl_token_metadata::state::{Collection, DataV2, Creator};
use crate::errors::ErrorCode;

#[derive(Accounts)]
#[instruction(claim_id: String)]
pub struct ClaimDrop<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        constraint = drop.collection == collection_mint.key(),
        constraint = drop.creator == creator.key(),
        seeds=[
            claim_id.as_ref(),
        ],
        bump = drop.drop_nonce
    )]
    pub drop: Box<Account<'info, Drop>>,

    pub creator: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [
            claim_id.as_ref(),
            signer.to_account_info().key().as_ref(),
        ],
        space = 105 + 8,
        payer = signer,
        bump
    )]
    pub claimed: Box<Account<'info, Claimed>>,

    #[account(
        mut,
        seeds = [
            b"collection",
            drop.key().as_ref(),
        ],
        bump
    )]
    pub collection_mint: Box<Account<'info, Mint>>,

    /// CHECK: already initalized
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
    pub collection_metadata: UncheckedAccount<'info>,


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
    pub collection_edition: UncheckedAccount<'info>,


    #[account(
        init,
        payer = signer,
        seeds = [
            b"mint",
            drop.key().as_ref(),
            signer.key().as_ref(),
        ],
        bump,
        mint::authority = head_signer,
        mint::freeze_authority = head_signer,
        mint::decimals = 0,
    )]
    pub master_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = signer,
        associated_token::authority = signer,
        associated_token::mint = master_mint,
    )]
    pub master_token: Box<Account<'info, TokenAccount>>,

    /// CHECK: Account allocation and initialization is done via CPI to the metadata program.
    #[account(
        mut,
        seeds = [
            "metadata".as_bytes(),
            metadata_program.key().as_ref(),
            master_mint.key().as_ref(),
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
            master_mint.key().as_ref(),
            "edition".as_bytes(),
        ],
        seeds::program = metadata_program.key(),
        bump,
    )]
    pub master_edition: UncheckedAccount<'info>,

    /// CHECK: Head Signer
    #[account(
        mut,
        seeds = [
            drop.key().as_ref()
        ],
        bump = drop.signer_nonce
    )]
    pub head_signer: UncheckedAccount<'info>,

    pub system_program: Program<'info,System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub metadata_program: Program<'info, Metadata>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, ClaimDrop<'info>>,
    claim_id: String
) -> Result<()> {
    let drop = &mut ctx.accounts.drop;
    let drop_pda = drop.key();
    let claimed = &mut ctx.accounts.claimed;

    if claimed.minted == true {
        return err!(ErrorCode::ClaimedDrop)
    } if drop.left_mint == 0 && drop.infinite == false {
        return err!(ErrorCode::MintLimitReached)
    }

    let seeds = &[drop_pda.as_ref(), &[drop.signer_nonce]];
    let signer = &[&seeds[..]];

    //
    // Mint the master token.
    //
    {
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.master_mint.to_account_info(),
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
    msg!("Minted Token");

    //
    // Create metadata.
    //
    {
        let is_mutable = true;
        let update_authority_is_signer = true;

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.master_metadata.to_account_info(),
                mint: ctx.accounts.master_mint.to_account_info(),
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
                name: drop.name.clone(),
                symbol: drop.ticker.clone(),
                uri: drop.uri.clone(),
                seller_fee_basis_points: 1000,
                creators: Some(vec![
                    Creator {
                        address: ctx.accounts.head_signer.key(),
                        share: 0,
                        verified: true
                    },
                    Creator {
                        address: ctx.accounts.creator.key(),
                        verified: false,
                        share: 100
                    }
                ]),
                collection: Some(Collection {
                    key: ctx.accounts.collection_mint.key(),
                    verified: false
                }),
                uses: None,
            },
            is_mutable,
            update_authority_is_signer,
            None,
        )?;
    }
    msg!("Created metadata");

    //
    // Create master edition.
    //
    {
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.metadata_program.to_account_info(),
            CreateMasterEditionV3 {
                edition: ctx.accounts.master_edition.to_account_info(),
                mint: ctx.accounts.master_mint.to_account_info(),
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
    msg!("Created master edition");

    // 
    // Verify Collection
    // 
    {
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.metadata_program.to_account_info(),
            VerifyCollection {
                payer: ctx.accounts.signer.to_account_info(),
                metadata: ctx.accounts.master_metadata.to_account_info(),
                collection_authority: ctx.accounts.head_signer.to_account_info(),
                collection_metadata: ctx.accounts.collection_metadata.to_account_info(),
                collection_master_edition: ctx.accounts.collection_edition.to_account_info(),
                collection_mint: ctx.accounts.collection_mint.to_account_info()
            },
            signer
        );

        metadata::verify_collection(cpi_ctx, None)?;
    }
    msg!("Verified Collection");

    if drop.left_mint > 0 && drop.infinite == true {
        drop.left_mint = drop.left_mint - 1;
    }

    claimed.minted = true;
    claimed.master_mint = ctx.accounts.master_mint.key();
    claimed.master_edition = ctx.accounts.master_edition.key();
    claimed.master_metadata = ctx.accounts.master_metadata.key();

    claimed.nonce = *ctx.bumps.get("claimed").unwrap();

    Ok(())
}