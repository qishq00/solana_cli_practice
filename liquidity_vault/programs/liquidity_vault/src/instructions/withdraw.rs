use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::errors::VaultError;
use crate::math::assets_for_shares;
use crate::state::{UserPosition, VaultConfig};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        has_one = share_mint,
        has_one = token_vault,
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    /// CHECK: PDA-authority vault, владелец token_vault — подписывает перевод.
    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [USER_SEED, config.key().as_ref(), owner.key().as_ref()],
        bump = user_position.bump,
        has_one = owner,
    )]
    pub user_position: Account<'info, UserPosition>,

    #[account(mut)]
    pub share_mint: Account<'info, Mint>,

    #[account(mut)]
    pub token_vault: Account<'info, TokenAccount>,

    /// Куда вернуть токены.
    #[account(
        mut,
        token::mint = config.token_mint,
        token::authority = owner,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Откуда сжигаются share-токены.
    #[account(
        mut,
        token::mint = share_mint,
        token::authority = owner,
    )]
    pub user_share_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Withdraw>, shares: u64, min_amount_out: u64) -> Result<()> {
    let config = &ctx.accounts.config;
    require!(!config.paused, VaultError::VaultPaused);
    require!(shares > 0, VaultError::ZeroAmount);
    require!(
        ctx.accounts.user_share_account.amount >= shares,
        VaultError::InsufficientShares
    );

    let total_assets = ctx.accounts.token_vault.amount;
    let total_shares = ctx.accounts.share_mint.supply;
    let amount =
        assets_for_shares(shares, total_assets, total_shares).ok_or(VaultError::MathOverflow)?;
    require!(amount > 0, VaultError::ZeroAmount);
    require!(amount >= min_amount_out, VaultError::SlippageExceeded);
    require!(amount <= total_assets, VaultError::InsufficientLiquidity);

    // 1) сжигаем share-токены (подписывает владелец)
    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.share_mint.to_account_info(),
                from: ctx.accounts.user_share_account.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        shares,
    )?;

    // 2) CPI в Token program: vault -> пользователь, подписывает PDA
    let config_key = config.key();
    let bump = config.authority_bump;
    let signer_seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, config_key.as_ref(), &[bump]]];
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_vault.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    // 3) учёт
    let config = &mut ctx.accounts.config;
    config.total_deposited = config.total_deposited.saturating_sub(amount);

    let position = &mut ctx.accounts.user_position;
    position.withdrawn = position
        .withdrawn
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;
    position.shares = position.shares.saturating_sub(shares);

    emit!(WithdrawEvent {
        owner: position.owner,
        shares,
        amount,
    });
    Ok(())
}

#[event]
pub struct WithdrawEvent {
    pub owner: Pubkey,
    pub shares: u64,
    pub amount: u64,
}
