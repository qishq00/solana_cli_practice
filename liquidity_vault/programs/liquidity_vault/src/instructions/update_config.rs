use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::VaultError;
use crate::state::VaultConfig;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, config.token_mint.as_ref()],
        bump = config.bump,
        has_one = admin @ VaultError::Unauthorized,
    )]
    pub config: Account<'info, VaultConfig>,
}

pub fn handler(
    ctx: Context<UpdateConfig>,
    paused: Option<bool>,
    deposit_cap: Option<u64>,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    if let Some(p) = paused {
        config.paused = p;
    }
    if let Some(cap) = deposit_cap {
        config.deposit_cap = cap;
    }
    Ok(())
}
