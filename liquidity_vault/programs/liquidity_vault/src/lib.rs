//! Liquidity Vault — учебный DeFi-vault.
//!
//! Пользователь вносит токены в vault и получает share-токены (свою долю).
//! Vault (через PDA) размещает токены в пуле ликвидности Raydium CLMM.

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod math;
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
}
