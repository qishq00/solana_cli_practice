//! Минимальная интеграция с Raydium CLMM без внешнего крейта:
//! адреса программ и чтение нужных полей `PoolState` по смещениям.
//!
//! `PoolState` в Raydium — zero-copy `#[repr(C, packed)]`, поэтому смещения фиксированы:
//! ```text
//!   0..8    discriminator
//!   8       bump
//!   9..41   amm_config
//!  41..73   owner
//!  73..105  token_mint_0
//! 105..137  token_mint_1
//! 137..169  token_vault_0
//! 169..201  token_vault_1
//! 201..233  observation_key
//! 233       mint_decimals_0
//! 234       mint_decimals_1
//! 235..237  tick_spacing (u16)
//! 237..253  liquidity (u128)
//! 253..269  sqrt_price_x64 (u128)
//! 269..273  tick_current (i32)
//! ```

use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey;

use crate::errors::VaultError;

/// Raydium CLMM, mainnet-beta.
pub const CLMM_PROGRAM_MAINNET: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
/// Raydium CLMM, devnet (актуальный адрес из docs.raydium.io).
pub const CLMM_PROGRAM_DEVNET: Pubkey = pubkey!("DRayAUgENGQBKVaX8owNhgzkEDyoHTGVEGHVJT1E9pfH");
/// Raydium CLMM, старый devnet-деплой.
pub const CLMM_PROGRAM_DEVNET_LEGACY: Pubkey =
    pubkey!("devi51mZmdwUJGU9hjN27vEz64Gps7uUefqxg27EAtH");

pub fn is_clmm_program(key: &Pubkey) -> bool {
    *key == CLMM_PROGRAM_MAINNET
        || *key == CLMM_PROGRAM_DEVNET
        || *key == CLMM_PROGRAM_DEVNET_LEGACY
}

const POOL_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];

pub struct PoolSnapshot {
    pub amm_config: Pubkey,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub observation: Pubkey,
    pub tick_spacing: u16,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
}

fn pk(data: &[u8], at: usize) -> Pubkey {
    Pubkey::new_from_array(data[at..at + 32].try_into().unwrap())
}

/// Проверяет, что аккаунт — пул Raydium CLMM, и читает его ключевые поля.
pub fn read_pool(pool: &AccountInfo) -> Result<PoolSnapshot> {
    require!(is_clmm_program(pool.owner), VaultError::InvalidPool);
    let data = pool.try_borrow_data()?;
    require!(data.len() >= 273, VaultError::InvalidPool);
    require!(data[..8] == POOL_DISCRIMINATOR, VaultError::InvalidPool);

    Ok(PoolSnapshot {
        amm_config: pk(&data, 9),
        token_mint_0: pk(&data, 73),
        token_mint_1: pk(&data, 105),
        token_vault_0: pk(&data, 137),
        token_vault_1: pk(&data, 169),
        observation: pk(&data, 201),
        tick_spacing: u16::from_le_bytes(data[235..237].try_into().unwrap()),
        sqrt_price_x64: u128::from_le_bytes(data[253..269].try_into().unwrap()),
        tick_current: i32::from_le_bytes(data[269..273].try_into().unwrap()),
    })
}
