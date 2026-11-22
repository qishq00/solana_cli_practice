use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::*;
use crate::state::VaultConfig;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [VAULT_SEED, token_mint.key().as_ref()],
        bump
    )]
    pub config: Account<'info, VaultConfig>,

    /// CHECK: PDA без данных; владеет хранилищем и является mint authority share-токена.
    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// Share-токен: те же decimals, что у базового токена.
    #[account(
        init,
        payer = admin,
        seeds = [SHARE_MINT_SEED, config.key().as_ref()],
        bump,
        mint::decimals = token_mint.decimals,
        mint::authority = vault_authority,
    )]
    pub share_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        seeds = [TOKEN_VAULT_SEED, config.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = vault_authority,
    )]
    pub token_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>, deposit_cap: u64) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.token_mint = ctx.accounts.token_mint.key();
    config.share_mint = ctx.accounts.share_mint.key();
    config.token_vault = ctx.accounts.token_vault.key();
    config.total_deposited = 0;
    config.deposit_cap = deposit_cap;
    config.paused = false;
    config.bump = ctx.bumps.config;
    config.authority_bump = ctx.bumps.vault_authority;
    config.share_mint_bump = ctx.bumps.share_mint;
    config.token_vault_bump = ctx.bumps.token_vault;
    config.quote_mint = Pubkey::default();
    config.quote_vault = Pubkey::default();
    config.clmm_pool = Pubkey::default();
    config.clmm_program = Pubkey::default();

    msg!(
        "Vault создан: mint={}, share_mint={}",
        config.token_mint,
        config.share_mint
    );
    Ok(())
}
