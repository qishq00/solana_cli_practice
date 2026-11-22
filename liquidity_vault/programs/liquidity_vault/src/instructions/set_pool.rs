use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::VaultError;
use crate::raydium::read_pool;
use crate::state::VaultConfig;

/// Неделя 11. Привязывает vault к CLMM-пулу Raydium.
/// Проверяем, что аккаунт действительно пул Raydium (owner + discriminator)
/// и что его пара токенов совпадает с парой vault.
#[derive(Accounts)]
pub struct SetPool<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        has_one = admin @ VaultError::Unauthorized,
    )]
    pub config: Account<'info, VaultConfig>,

    /// CHECK: проверяется в `read_pool` (owner = Raydium CLMM, discriminator PoolState).
    pub pool_state: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<SetPool>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    require!(config.quote_mint != Pubkey::default(), VaultError::PoolNotSet);

    let pool = read_pool(&ctx.accounts.pool_state.to_account_info())?;
    let pair_ok = (pool.token_mint_0 == config.token_mint && pool.token_mint_1 == config.quote_mint)
        || (pool.token_mint_0 == config.quote_mint && pool.token_mint_1 == config.token_mint);
    require!(pair_ok, VaultError::InvalidPool);

    config.clmm_pool = ctx.accounts.pool_state.key();
    config.clmm_program = *ctx.accounts.pool_state.owner;
    msg!(
        "Пул привязан: {}, tick_current={}, tick_spacing={}",
        config.clmm_pool,
        pool.tick_current,
        pool.tick_spacing
    );
    Ok(())
}
