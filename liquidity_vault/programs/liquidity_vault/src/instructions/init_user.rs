use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::{UserPosition, VaultConfig};

#[derive(Accounts)]
pub struct InitUser<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(seeds = [VAULT_SEED, config.token_mint.as_ref()], bump = config.bump)]
    pub config: Account<'info, VaultConfig>,

    #[account(
        init,
        payer = owner,
        space = 8 + UserPosition::INIT_SPACE,
        seeds = [USER_SEED, config.key().as_ref(), owner.key().as_ref()],
        bump
    )]
    pub user_position: Account<'info, UserPosition>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitUser>) -> Result<()> {
    let position = &mut ctx.accounts.user_position;
    position.owner = ctx.accounts.owner.key();
    position.vault = ctx.accounts.config.key();
    position.deposited = 0;
    position.withdrawn = 0;
    position.shares = 0;
    position.bump = ctx.bumps.user_position;
    Ok(())
}
