use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::VaultError;
use crate::state::VaultConfig;

/// Неделя 11. Создаёт хранилище второго токена пары (quote) —
/// оно нужно для swap и для позиции в CLMM-пуле (там всегда два токена).
#[derive(Accounts)]
pub struct InitQuoteVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        has_one = admin @ VaultError::Unauthorized,
    )]
    pub config: Account<'info, VaultConfig>,

    /// CHECK: PDA-authority vault.
    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(constraint = quote_mint.key() != config.token_mint @ VaultError::InvalidPool)]
    pub quote_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        seeds = [QUOTE_VAULT_SEED, config.key().as_ref()],
        bump,
        token::mint = quote_mint,
        token::authority = vault_authority,
    )]
    pub quote_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitQuoteVault>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.quote_mint = ctx.accounts.quote_mint.key();
    config.quote_vault = ctx.accounts.quote_vault.key();
    Ok(())
}
