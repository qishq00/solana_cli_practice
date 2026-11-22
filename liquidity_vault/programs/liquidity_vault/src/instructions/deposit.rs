use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::errors::VaultError;
use crate::math::shares_for_deposit;
use crate::state::{UserPosition, VaultConfig};

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        has_one = share_mint,
        has_one = token_vault,
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    /// CHECK: PDA-authority vault, подписывает mint share-токенов.
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

    /// Откуда пользователь вносит токены.
    #[account(
        mut,
        token::mint = config.token_mint,
        token::authority = owner,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Куда начисляются share-токены.
    #[account(
        mut,
        token::mint = share_mint,
        token::authority = owner,
    )]
    pub user_share_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let config = &ctx.accounts.config;
    require!(!config.paused, VaultError::VaultPaused);
    require!(amount > 0, VaultError::ZeroAmount);

    // Сколько активов уже лежит в vault (до депозита).
    let total_assets = ctx.accounts.token_vault.amount;
    if config.deposit_cap > 0 {
        let after = total_assets
            .checked_add(amount)
            .ok_or(VaultError::MathOverflow)?;
        require!(after <= config.deposit_cap, VaultError::DepositCapExceeded);
    }

    let total_shares = ctx.accounts.share_mint.supply;
    let shares = shares_for_deposit(amount, total_assets, total_shares)
        .ok_or(VaultError::MathOverflow)?;
    require!(shares > 0, VaultError::ZeroShares);

    // 1) пользователь -> vault (подписывает пользователь)
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.token_vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
    )?;

    // 2) mint share-токенов пользователю (подписывает PDA vault_authority)
    let config_key = config.key();
    let bump = config.authority_bump;
    let signer_seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, config_key.as_ref(), &[bump]]];
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.share_mint.to_account_info(),
                to: ctx.accounts.user_share_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        shares,
    )?;

    // 3) учёт
    let config = &mut ctx.accounts.config;
    config.total_deposited = config
        .total_deposited
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;

    let position = &mut ctx.accounts.user_position;
    position.deposited = position
        .deposited
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;
    position.shares = position
        .shares
        .checked_add(shares)
        .ok_or(VaultError::MathOverflow)?;

    emit!(DepositEvent {
        owner: position.owner,
        amount,
        shares,
    });
    Ok(())
}

#[event]
pub struct DepositEvent {
    pub owner: Pubkey,
    pub amount: u64,
    pub shares: u64,
}
