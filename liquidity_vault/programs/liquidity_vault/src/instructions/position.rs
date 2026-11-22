//! Неделя 13. Позиция vault в CLMM-пуле Raydium:
//! открыть в диапазоне цен → собирать комиссии → закрыть → переставить,
//! когда цена вышла из диапазона.

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Token, TokenAccount};

use crate::constants::*;
use crate::errors::VaultError;
use crate::raydium::{self, read_pool, PoolSnapshot};
use crate::state::VaultConfig;

/// Проверяет, что переданные хранилища/mint-ы пула совпадают с PoolState
/// и что токен-аккаунты 0/1 — это хранилища vault в правильном порядке.
#[allow(clippy::too_many_arguments)]
fn check_pool_accounts(
    config: &VaultConfig,
    pool: &PoolSnapshot,
    token_account_0: &TokenAccount,
    token_account_1: &TokenAccount,
    token_vault_0: Pubkey,
    token_vault_1: Pubkey,
    vault_0_mint: Pubkey,
    vault_1_mint: Pubkey,
) -> Result<()> {
    require_keys_eq!(token_vault_0, pool.token_vault_0, VaultError::InvalidPool);
    require_keys_eq!(token_vault_1, pool.token_vault_1, VaultError::InvalidPool);
    require_keys_eq!(vault_0_mint, pool.token_mint_0, VaultError::InvalidPool);
    require_keys_eq!(vault_1_mint, pool.token_mint_1, VaultError::InvalidPool);
    require_keys_eq!(token_account_0.mint, pool.token_mint_0, VaultError::InvalidPool);
    require_keys_eq!(token_account_1.mint, pool.token_mint_1, VaultError::InvalidPool);
    // token_account_0/1 — разные хранилища vault (проверено constraint-ами)
    require_keys_neq!(
        token_account_0.mint,
        token_account_1.mint,
        VaultError::InvalidPool
    );
    let _ = config;
    Ok(())
}

fn is_vault_account(config: &VaultConfig, key: &Pubkey) -> bool {
    *key == config.token_vault || *key == config.quote_vault
}

// =====================================================================
// open_position
// =====================================================================

#[derive(Accounts)]
pub struct OpenPosition<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        has_one = admin @ VaultError::Unauthorized,
        constraint = config.clmm_pool != Pubkey::default() @ VaultError::PoolNotSet,
        constraint = config.position_nft_mint == Pubkey::default() @ VaultError::PositionAlreadyOpen,
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    /// CHECK: PDA-authority vault: платит ренту за аккаунты позиции и владеет NFT.
    /// Админ заранее пополняет его SOL (System transfer).
    #[account(mut, seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// CHECK: программа Raydium CLMM из config.
    #[account(address = config.clmm_program)]
    pub clmm_program: UncheckedAccount<'info>,

    /// Новый keypair — mint NFT позиции (создаёт Raydium).
    #[account(mut)]
    pub position_nft_mint: Signer<'info>,
    /// CHECK: ATA(vault_authority, position_nft_mint), создаёт Raydium.
    #[account(mut)]
    pub position_nft_account: UncheckedAccount<'info>,
    /// CHECK: PDA метаданных NFT (with_metadata = false, но аккаунт обязателен).
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    /// CHECK: пул vault.
    #[account(mut, address = config.clmm_pool)]
    pub pool_state: UncheckedAccount<'info>,
    /// CHECK: PDA Raydium ["position", pool, lower_start, upper_start].
    #[account(mut)]
    pub protocol_position: UncheckedAccount<'info>,
    /// CHECK: tick array нижней границы.
    #[account(mut)]
    pub tick_array_lower: UncheckedAccount<'info>,
    /// CHECK: tick array верхней границы.
    #[account(mut)]
    pub tick_array_upper: UncheckedAccount<'info>,
    /// CHECK: PDA Raydium ["position", nft_mint].
    #[account(mut)]
    pub personal_position: UncheckedAccount<'info>,

    /// Хранилище vault с token_mint_0 пула.
    #[account(mut, constraint = is_vault_account(&config, &token_account_0.key()) @ VaultError::InvalidPool)]
    pub token_account_0: Box<Account<'info, TokenAccount>>,
    /// Хранилище vault с token_mint_1 пула.
    #[account(mut, constraint = is_vault_account(&config, &token_account_1.key()) @ VaultError::InvalidPool)]
    pub token_account_1: Box<Account<'info, TokenAccount>>,
    /// CHECK: хранилище пула 0, сверяется с PoolState.
    #[account(mut)]
    pub token_vault_0: UncheckedAccount<'info>,
    /// CHECK: хранилище пула 1, сверяется с PoolState.
    #[account(mut)]
    pub token_vault_1: UncheckedAccount<'info>,
    /// CHECK: mint 0, сверяется с PoolState.
    pub vault_0_mint: UncheckedAccount<'info>,
    /// CHECK: mint 1, сверяется с PoolState.
    pub vault_1_mint: UncheckedAccount<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: Metaplex Token Metadata.
    #[account(address = raydium::METADATA_PROGRAM)]
    pub metadata_program: UncheckedAccount<'info>,
    /// CHECK: Token-2022.
    #[account(address = anchor_spl::token_2022::ID)]
    pub token_program_2022: UncheckedAccount<'info>,
    // remaining_accounts: [tick_array_bitmap_extension?]
}

#[allow(clippy::too_many_arguments)]
pub fn open_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, OpenPosition<'info>>,
    tick_lower: i32,
    tick_upper: i32,
    tick_array_lower_start: i32,
    tick_array_upper_start: i32,
    liquidity: u128,
    amount_0_max: u64,
    amount_1_max: u64,
) -> Result<()> {
    let a = &ctx.accounts;
    let pool = read_pool(&a.pool_state.to_account_info())?;

    // Диапазон: lower < upper, оба кратны tick_spacing.
    let spacing = pool.tick_spacing as i32;
    require!(tick_lower < tick_upper, VaultError::InvalidRange);
    require!(
        tick_lower % spacing == 0 && tick_upper % spacing == 0,
        VaultError::InvalidRange
    );
    check_pool_accounts(
        &a.config,
        &pool,
        &a.token_account_0,
        &a.token_account_1,
        a.token_vault_0.key(),
        a.token_vault_1.key(),
        a.vault_0_mint.key(),
        a.vault_1_mint.key(),
    )?;

    let bal0 = a.token_account_0.amount;
    let bal1 = a.token_account_1.amount;

    // Порядок — как в Raydium `OpenPositionV2`.
    let mut accounts = vec![
        a.vault_authority.to_account_info(), // payer
        a.vault_authority.to_account_info(), // position_nft_owner
        a.position_nft_mint.to_account_info(),
        a.position_nft_account.to_account_info(),
        a.metadata_account.to_account_info(),
        a.pool_state.to_account_info(),
        a.protocol_position.to_account_info(),
        a.tick_array_lower.to_account_info(),
        a.tick_array_upper.to_account_info(),
        a.personal_position.to_account_info(),
        a.token_account_0.to_account_info(),
        a.token_account_1.to_account_info(),
        a.token_vault_0.to_account_info(),
        a.token_vault_1.to_account_info(),
        a.rent.to_account_info(),
        a.system_program.to_account_info(),
        a.token_program.to_account_info(),
        a.associated_token_program.to_account_info(),
        a.metadata_program.to_account_info(),
        a.token_program_2022.to_account_info(),
        a.vault_0_mint.to_account_info(),
        a.vault_1_mint.to_account_info(),
    ];
    accounts.extend(ctx.remaining_accounts.iter().cloned());

    let config_key = a.config.key();
    let bump = a.config.authority_bump;
    let seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, config_key.as_ref(), &[bump]]];

    // liquidity = 0 → Raydium сам посчитает ликвидность от amount_0_max (base_flag = true)
    let base_flag = if liquidity == 0 { Some(true) } else { None };
    raydium::invoke_clmm(
        &a.clmm_program.to_account_info(),
        &accounts,
        &[a.vault_authority.key(), a.position_nft_mint.key()],
        raydium::open_position_v2_data(
            tick_lower,
            tick_upper,
            tick_array_lower_start,
            tick_array_upper_start,
            liquidity,
            amount_0_max,
            amount_1_max,
            false,
            base_flag,
        ),
        seeds,
    )?;

    let nft_mint = a.position_nft_mint.key();
    ctx.accounts.token_account_0.reload()?;
    ctx.accounts.token_account_1.reload()?;
    let used0 = bal0.saturating_sub(ctx.accounts.token_account_0.amount);
    let used1 = bal1.saturating_sub(ctx.accounts.token_account_1.amount);

    let config = &mut ctx.accounts.config;
    config.position_nft_mint = nft_mint;
    config.position_tick_lower = tick_lower;
    config.position_tick_upper = tick_upper;

    emit!(PositionOpened {
        nft_mint,
        tick_lower,
        tick_upper,
        tick_current: pool.tick_current,
        amount_0: used0,
        amount_1: used1,
    });
    Ok(())
}

// =====================================================================
// decrease_liquidity / collect_fees
// =====================================================================

#[derive(Accounts)]
pub struct ModifyPosition<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        has_one = admin @ VaultError::Unauthorized,
        constraint = config.position_nft_mint != Pubkey::default() @ VaultError::NoPosition,
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    /// CHECK: PDA-authority vault — владелец NFT позиции.
    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// CHECK: программа Raydium CLMM из config.
    #[account(address = config.clmm_program)]
    pub clmm_program: UncheckedAccount<'info>,

    #[account(
        token::mint = config.position_nft_mint,
        token::authority = vault_authority,
        constraint = position_nft_account.amount == 1 @ VaultError::NoPosition,
    )]
    pub position_nft_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: позиция Raydium (Raydium проверяет связь с NFT).
    #[account(mut)]
    pub personal_position: UncheckedAccount<'info>,
    /// CHECK: пул vault.
    #[account(mut, address = config.clmm_pool)]
    pub pool_state: UncheckedAccount<'info>,
    /// CHECK: protocol position Raydium.
    #[account(mut)]
    pub protocol_position: UncheckedAccount<'info>,
    /// CHECK: хранилище пула 0.
    #[account(mut)]
    pub token_vault_0: UncheckedAccount<'info>,
    /// CHECK: хранилище пула 1.
    #[account(mut)]
    pub token_vault_1: UncheckedAccount<'info>,
    /// CHECK: tick array нижней границы.
    #[account(mut)]
    pub tick_array_lower: UncheckedAccount<'info>,
    /// CHECK: tick array верхней границы.
    #[account(mut)]
    pub tick_array_upper: UncheckedAccount<'info>,
    /// Хранилище vault для токена 0 (получатель).
    #[account(mut, constraint = is_vault_account(&config, &recipient_token_account_0.key()) @ VaultError::InvalidPool)]
    pub recipient_token_account_0: Box<Account<'info, TokenAccount>>,
    /// Хранилище vault для токена 1 (получатель).
    #[account(mut, constraint = is_vault_account(&config, &recipient_token_account_1.key()) @ VaultError::InvalidPool)]
    pub recipient_token_account_1: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    /// CHECK: Token-2022.
    #[account(address = anchor_spl::token_2022::ID)]
    pub token_program_2022: UncheckedAccount<'info>,
    /// CHECK: SPL Memo.
    #[account(address = raydium::MEMO_PROGRAM)]
    pub memo_program: UncheckedAccount<'info>,
    /// CHECK: mint 0.
    pub vault_0_mint: UncheckedAccount<'info>,
    /// CHECK: mint 1.
    pub vault_1_mint: UncheckedAccount<'info>,
    // remaining_accounts: [tick_array_bitmap_extension?, reward accounts...]
}

pub fn decrease_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, ModifyPosition<'info>>,
    liquidity: u128,
    amount_0_min: u64,
    amount_1_min: u64,
) -> Result<()> {
    let a = &ctx.accounts;
    let pool = read_pool(&a.pool_state.to_account_info())?;
    check_pool_accounts(
        &a.config,
        &pool,
        &a.recipient_token_account_0,
        &a.recipient_token_account_1,
        a.token_vault_0.key(),
        a.token_vault_1.key(),
        a.vault_0_mint.key(),
        a.vault_1_mint.key(),
    )?;

    let bal0 = a.recipient_token_account_0.amount;
    let bal1 = a.recipient_token_account_1.amount;

    // Порядок — как в Raydium `DecreaseLiquidityV2`.
    let mut accounts = vec![
        a.vault_authority.to_account_info(), // nft_owner
        a.position_nft_account.to_account_info(),
        a.personal_position.to_account_info(),
        a.pool_state.to_account_info(),
        a.protocol_position.to_account_info(),
        a.token_vault_0.to_account_info(),
        a.token_vault_1.to_account_info(),
        a.tick_array_lower.to_account_info(),
        a.tick_array_upper.to_account_info(),
        a.recipient_token_account_0.to_account_info(),
        a.recipient_token_account_1.to_account_info(),
        a.token_program.to_account_info(),
        a.token_program_2022.to_account_info(),
        a.memo_program.to_account_info(),
        a.vault_0_mint.to_account_info(),
        a.vault_1_mint.to_account_info(),
    ];
    accounts.extend(ctx.remaining_accounts.iter().cloned());

    let config_key = a.config.key();
    let bump = a.config.authority_bump;
    let seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, config_key.as_ref(), &[bump]]];

    raydium::invoke_clmm(
        &a.clmm_program.to_account_info(),
        &accounts,
        &[a.vault_authority.key()],
        raydium::decrease_liquidity_v2_data(liquidity, amount_0_min, amount_1_min),
        seeds,
    )?;

    ctx.accounts.recipient_token_account_0.reload()?;
    ctx.accounts.recipient_token_account_1.reload()?;
    emit!(LiquidityDecreased {
        liquidity,
        amount_0: ctx.accounts.recipient_token_account_0.amount.saturating_sub(bal0),
        amount_1: ctx.accounts.recipient_token_account_1.amount.saturating_sub(bal1),
        fees_only: liquidity == 0,
    });
    Ok(())
}

// =====================================================================
// close_position
// =====================================================================

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        has_one = admin @ VaultError::Unauthorized,
        constraint = config.position_nft_mint != Pubkey::default() @ VaultError::NoPosition,
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    /// CHECK: PDA-authority vault — владелец NFT, получает обратно ренту.
    #[account(mut, seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// CHECK: программа Raydium CLMM из config.
    #[account(address = config.clmm_program)]
    pub clmm_program: UncheckedAccount<'info>,

    /// CHECK: mint NFT текущей позиции.
    #[account(mut, address = config.position_nft_mint)]
    pub position_nft_mint: UncheckedAccount<'info>,
    #[account(
        mut,
        token::mint = config.position_nft_mint,
        token::authority = vault_authority,
    )]
    pub position_nft_account: Box<Account<'info, TokenAccount>>,
    /// CHECK: позиция Raydium (ликвидность и комиссии должны быть уже выведены).
    #[account(mut)]
    pub personal_position: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn close_handler(ctx: Context<ClosePosition>) -> Result<()> {
    let a = &ctx.accounts;
    // Порядок — как в Raydium `ClosePosition`.
    let accounts = vec![
        a.vault_authority.to_account_info(),
        a.position_nft_mint.to_account_info(),
        a.position_nft_account.to_account_info(),
        a.personal_position.to_account_info(),
        a.system_program.to_account_info(),
        a.token_program.to_account_info(),
    ];
    let config_key = a.config.key();
    let bump = a.config.authority_bump;
    let seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, config_key.as_ref(), &[bump]]];

    raydium::invoke_clmm(
        &a.clmm_program.to_account_info(),
        &accounts,
        &[a.vault_authority.key()],
        raydium::ix::CLOSE_POSITION.to_vec(),
        seeds,
    )?;

    let nft_mint = a.position_nft_mint.key();
    let config = &mut ctx.accounts.config;
    config.position_nft_mint = Pubkey::default();
    config.position_tick_lower = 0;
    config.position_tick_upper = 0;
    emit!(PositionClosed { nft_mint });
    Ok(())
}

// =====================================================================
// check_rebalance — вышла ли цена из диапазона позиции
// =====================================================================

#[derive(Accounts)]
pub struct CheckRebalance<'info> {
    #[account(
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        constraint = config.position_nft_mint != Pubkey::default() @ VaultError::NoPosition,
    )]
    pub config: Box<Account<'info, VaultConfig>>,
    /// CHECK: пул vault.
    #[account(address = config.clmm_pool)]
    pub pool_state: UncheckedAccount<'info>,
}

/// Ok — цена вне диапазона, позицию нужно переставить;
/// ошибка `PositionInRange` — позиция работает, ничего делать не надо.
pub fn check_rebalance_handler(ctx: Context<CheckRebalance>) -> Result<()> {
    let pool = read_pool(&ctx.accounts.pool_state.to_account_info())?;
    let cfg = &ctx.accounts.config;
    // В Raydium позиция активна при tick_lower <= tick_current < tick_upper.
    let in_range =
        pool.tick_current >= cfg.position_tick_lower && pool.tick_current < cfg.position_tick_upper;
    require!(!in_range, VaultError::PositionInRange);
    emit!(RebalanceNeeded {
        tick_current: pool.tick_current,
        tick_lower: cfg.position_tick_lower,
        tick_upper: cfg.position_tick_upper,
    });
    Ok(())
}

#[event]
pub struct PositionOpened {
    pub nft_mint: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub tick_current: i32,
    pub amount_0: u64,
    pub amount_1: u64,
}

#[event]
pub struct LiquidityDecreased {
    pub liquidity: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub fees_only: bool,
}

#[event]
pub struct PositionClosed {
    pub nft_mint: Pubkey,
}

#[event]
pub struct RebalanceNeeded {
    pub tick_current: i32,
    pub tick_lower: i32,
    pub tick_upper: i32,
}
