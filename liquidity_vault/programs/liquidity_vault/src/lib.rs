//! Liquidity Vault — учебный DeFi-vault.
//!
//! Пользователь вносит токены в vault и получает share-токены (свою долю).
//! Vault (через PDA) размещает токены в пуле ликвидности Raydium CLMM.

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod raydium;
pub mod state;

use instructions::*;

declare_id!("G6tDjfVNF75XLshiPFNtb6GsDQZtYjZxzTngcF1UxpVF");

#[program]
pub mod liquidity_vault {
    use super::*;

    /// Неделя 8. Создаёт аккаунт настроек vault, PDA-authority,
    /// share-mint и токен-аккаунт хранилища.
    pub fn initialize(ctx: Context<Initialize>, deposit_cap: u64) -> Result<()> {
        instructions::initialize::handler(ctx, deposit_cap)
    }

    /// Неделя 8. Создаёт PDA пользователя (его позицию в vault).
    pub fn init_user(ctx: Context<InitUser>) -> Result<()> {
        instructions::init_user::handler(ctx)
    }

    /// Неделя 8. Админ меняет настройки: пауза и лимит депозитов.
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        paused: Option<bool>,
        deposit_cap: Option<u64>,
    ) -> Result<()> {
        instructions::update_config::handler(ctx, paused, deposit_cap)
    }

    /// Неделя 9. Депозит: пользователь вносит токены и получает share-токены.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }

    /// Неделя 10. Вывод: share-токены сжигаются, vault возвращает токены
    /// через CPI в Token program с подписью PDA.
    pub fn withdraw(ctx: Context<Withdraw>, shares: u64, min_amount_out: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, shares, min_amount_out)
    }

    /// Неделя 11. Хранилище второго токена пары.
    pub fn init_quote_vault(ctx: Context<InitQuoteVault>) -> Result<()> {
        instructions::init_quote_vault::handler(ctx)
    }

    /// Неделя 11. Привязка vault к CLMM-пулу Raydium.
    pub fn set_pool(ctx: Context<SetPool>) -> Result<()> {
        instructions::set_pool::handler(ctx)
    }

    /// Неделя 12. Swap токенов vault через CLMM-пул Raydium
    /// с ограничением на минимальную сумму получения (slippage).
    pub fn swap<'info>(
        ctx: Context<'_, '_, 'info, 'info, Swap<'info>>,
        amount_in: u64,
        min_amount_out: u64,
        sqrt_price_limit_x64: u128,
        base_to_quote: bool,
    ) -> Result<()> {
        instructions::swap::handler(ctx, amount_in, min_amount_out, sqrt_price_limit_x64, base_to_quote)
    }

    /// Неделя 13. Открыть позицию vault в CLMM-пуле в диапазоне [tick_lower, tick_upper).
    #[allow(clippy::too_many_arguments)]
    pub fn open_position<'info>(
        ctx: Context<'_, '_, 'info, 'info, OpenPosition<'info>>,
        tick_lower: i32,
        tick_upper: i32,
        tick_array_lower_start: i32,
        tick_array_upper_start: i32,
        liquidity: u128,
        amount_0_max: u64,
        amount_1_max: u64,
    ) -> Result<()> {
        instructions::position::open_handler(
            ctx,
            tick_lower,
            tick_upper,
            tick_array_lower_start,
            tick_array_upper_start,
            liquidity,
            amount_0_max,
            amount_1_max,
        )
    }

    /// Неделя 13. Собрать накопленные комиссии позиции (decrease_liquidity с L = 0).
    pub fn collect_fees<'info>(ctx: Context<'_, '_, 'info, 'info, ModifyPosition<'info>>) -> Result<()> {
        instructions::position::decrease_handler(ctx, 0, 0, 0)
    }

    /// Неделя 13. Вывести ликвидность из позиции (вместе с комиссиями).
    pub fn decrease_liquidity<'info>(
        ctx: Context<'_, '_, 'info, 'info, ModifyPosition<'info>>,
        liquidity: u128,
        amount_0_min: u64,
        amount_1_min: u64,
    ) -> Result<()> {
        instructions::position::decrease_handler(ctx, liquidity, amount_0_min, amount_1_min)
    }

    /// Неделя 13. Закрыть пустую позицию (сжечь NFT, вернуть ренту).
    pub fn close_position(ctx: Context<ClosePosition>) -> Result<()> {
        instructions::position::close_handler(ctx)
    }

    /// Неделя 13. Проверить, вышла ли цена из диапазона позиции.
    pub fn check_rebalance(ctx: Context<CheckRebalance>) -> Result<()> {
        instructions::position::check_rebalance_handler(ctx)
    }
}
