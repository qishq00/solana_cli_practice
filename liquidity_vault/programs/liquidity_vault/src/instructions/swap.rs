use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::constants::*;
use crate::errors::VaultError;
use crate::raydium::{self, read_pool};
use crate::state::VaultConfig;

/// Неделя 12. Обмен токенов vault через CLMM-пул Raydium (`swap_v2`).
/// Обменивать средства vault может только админ (стратегия).
#[derive(Accounts)]
pub struct Swap<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        has_one = admin @ VaultError::Unauthorized,
        has_one = token_vault,
        has_one = quote_vault,
        constraint = config.clmm_pool != Pubkey::default() @ VaultError::PoolNotSet,
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    /// CHECK: PDA-authority vault — `payer` (владелец входного токен-аккаунта) в Raydium.
    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub quote_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: программа Raydium CLMM, совпадает с сохранённой в config.
    #[account(address = config.clmm_program)]
    pub clmm_program: UncheckedAccount<'info>,
    /// CHECK: сверяется с PoolState.
    pub amm_config: UncheckedAccount<'info>,
    /// CHECK: пул vault.
    #[account(mut, address = config.clmm_pool)]
    pub pool_state: UncheckedAccount<'info>,
    /// CHECK: хранилище пула для входного токена, сверяется с PoolState.
    #[account(mut)]
    pub pool_input_vault: UncheckedAccount<'info>,
    /// CHECK: хранилище пула для выходного токена, сверяется с PoolState.
    #[account(mut)]
    pub pool_output_vault: UncheckedAccount<'info>,
    /// CHECK: сверяется с PoolState.
    #[account(mut)]
    pub observation_state: UncheckedAccount<'info>,
    /// CHECK: mint входного токена.
    pub input_mint: UncheckedAccount<'info>,
    /// CHECK: mint выходного токена.
    pub output_mint: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    /// CHECK: Token-2022 (требуется сигнатурой swap_v2).
    #[account(address = anchor_spl::token_2022::ID)]
    pub token_program_2022: UncheckedAccount<'info>,
    /// CHECK: SPL Memo.
    #[account(address = raydium::MEMO_PROGRAM)]
    pub memo_program: UncheckedAccount<'info>,
    // remaining_accounts: [tick_array_bitmap_extension?, tick_array_0, tick_array_1, ...]
}

pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, Swap<'info>>,
    amount_in: u64,
    min_amount_out: u64,
    sqrt_price_limit_x64: u128,
    base_to_quote: bool,
) -> Result<()> {
    require!(amount_in > 0, VaultError::ZeroAmount);
    require!(min_amount_out > 0, VaultError::SlippageNotSet);

    let config = &ctx.accounts.config;
    let a = &ctx.accounts;

    // Направление обмена: base (token_mint) -> quote или наоборот.
    let (input_acc, output_acc, input_mint, output_mint) = if base_to_quote {
        (&a.token_vault, &a.quote_vault, config.token_mint, config.quote_mint)
    } else {
        (&a.quote_vault, &a.token_vault, config.quote_mint, config.token_mint)
    };
    require!(input_acc.amount >= amount_in, VaultError::InsufficientLiquidity);

    // Сверяем переданные аккаунты пула с данными PoolState.
    let pool = read_pool(&a.pool_state.to_account_info())?;
    require_keys_eq!(a.amm_config.key(), pool.amm_config, VaultError::InvalidPool);
    require_keys_eq!(a.observation_state.key(), pool.observation, VaultError::InvalidPool);
    require_keys_eq!(a.input_mint.key(), input_mint, VaultError::InvalidPool);
    require_keys_eq!(a.output_mint.key(), output_mint, VaultError::InvalidPool);
    let (exp_in_vault, exp_out_vault) = if pool.token_mint_0 == input_mint {
        (pool.token_vault_0, pool.token_vault_1)
    } else {
        (pool.token_vault_1, pool.token_vault_0)
    };
    require_keys_eq!(a.pool_input_vault.key(), exp_in_vault, VaultError::InvalidPool);
    require_keys_eq!(a.pool_output_vault.key(), exp_out_vault, VaultError::InvalidPool);

    let in_before = input_acc.amount;
    let out_before = output_acc.amount;

    // Порядок аккаунтов — как в Raydium `SwapSingleV2`.
    let mut accounts = vec![
        a.vault_authority.to_account_info(),
        a.amm_config.to_account_info(),
        a.pool_state.to_account_info(),
        input_acc.to_account_info(),
        output_acc.to_account_info(),
        a.pool_input_vault.to_account_info(),
        a.pool_output_vault.to_account_info(),
        a.observation_state.to_account_info(),
        a.token_program.to_account_info(),
        a.token_program_2022.to_account_info(),
        a.memo_program.to_account_info(),
        a.input_mint.to_account_info(),
        a.output_mint.to_account_info(),
    ];
    accounts.extend(ctx.remaining_accounts.iter().cloned());

    let config_key = config.key();
    let bump = config.authority_bump;
    let signer_seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, config_key.as_ref(), &[bump]]];

    raydium::invoke_clmm(
        &a.clmm_program.to_account_info(),
        &accounts,
        &[a.vault_authority.key()],
        // is_base_input = true: точный вход, other_amount_threshold = минимум на выходе
        raydium::swap_v2_data(amount_in, min_amount_out, sqrt_price_limit_x64, true),
        signer_seeds,
    )?;

    // Собственная проверка slippage по фактическим балансам vault.
    ctx.accounts.token_vault.reload()?;
    ctx.accounts.quote_vault.reload()?;
    let (in_after, out_after) = if base_to_quote {
        (ctx.accounts.token_vault.amount, ctx.accounts.quote_vault.amount)
    } else {
        (ctx.accounts.quote_vault.amount, ctx.accounts.token_vault.amount)
    };
    let spent = in_before.checked_sub(in_after).ok_or(VaultError::MathOverflow)?;
    let received = out_after.checked_sub(out_before).ok_or(VaultError::MathOverflow)?;
    require!(spent <= amount_in, VaultError::SlippageExceeded);
    require!(received >= min_amount_out, VaultError::SlippageExceeded);

    emit!(SwapEvent {
        base_to_quote,
        amount_in: spent,
        amount_out: received,
        tick_before: pool.tick_current,
    });
    Ok(())
}

#[event]
pub struct SwapEvent {
    pub base_to_quote: bool,
    pub amount_in: u64,
    pub amount_out: u64,
    pub tick_before: i32,
}
