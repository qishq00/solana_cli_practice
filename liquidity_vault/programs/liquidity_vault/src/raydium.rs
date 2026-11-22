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

// ---------- Неделя 12: CPI в Raydium CLMM ----------

use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke_signed;

/// SPL Memo v2 — Raydium требует его в *_v2 инструкциях.
pub const MEMO_PROGRAM: Pubkey = pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

/// Anchor-дискриминаторы инструкций Raydium CLMM: sha256("global:<name>")[..8].
pub mod ix {
    pub const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
    pub const OPEN_POSITION_V2: [u8; 8] = [77, 184, 74, 214, 112, 86, 241, 199];
    pub const DECREASE_LIQUIDITY_V2: [u8; 8] = [58, 127, 188, 62, 79, 82, 196, 96];
    pub const CLOSE_POSITION: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
}

/// Metaplex Token Metadata — нужен сигнатурой open_position_v2.
pub const METADATA_PROGRAM: Pubkey = pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

/// Данные `open_position_v2`.
#[allow(clippy::too_many_arguments)]
pub fn open_position_v2_data(
    tick_lower: i32,
    tick_upper: i32,
    tick_array_lower_start: i32,
    tick_array_upper_start: i32,
    liquidity: u128,
    amount_0_max: u64,
    amount_1_max: u64,
    with_metadata: bool,
    base_flag: Option<bool>,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(8 + 16 + 16 + 16 + 1 + 2);
    d.extend_from_slice(&ix::OPEN_POSITION_V2);
    d.extend_from_slice(&tick_lower.to_le_bytes());
    d.extend_from_slice(&tick_upper.to_le_bytes());
    d.extend_from_slice(&tick_array_lower_start.to_le_bytes());
    d.extend_from_slice(&tick_array_upper_start.to_le_bytes());
    d.extend_from_slice(&liquidity.to_le_bytes());
    d.extend_from_slice(&amount_0_max.to_le_bytes());
    d.extend_from_slice(&amount_1_max.to_le_bytes());
    d.push(with_metadata as u8);
    match base_flag {
        None => d.push(0),
        Some(b) => {
            d.push(1);
            d.push(b as u8);
        }
    }
    d
}

/// Данные `decrease_liquidity_v2`. `liquidity = 0` — только сбор комиссий.
pub fn decrease_liquidity_v2_data(liquidity: u128, amount_0_min: u64, amount_1_min: u64) -> Vec<u8> {
    let mut d = Vec::with_capacity(8 + 16 + 16);
    d.extend_from_slice(&ix::DECREASE_LIQUIDITY_V2);
    d.extend_from_slice(&liquidity.to_le_bytes());
    d.extend_from_slice(&amount_0_min.to_le_bytes());
    d.extend_from_slice(&amount_1_min.to_le_bytes());
    d
}

/// Данные инструкции `swap_v2`.
pub fn swap_v2_data(
    amount: u64,
    other_amount_threshold: u64,
    sqrt_price_limit_x64: u128,
    is_base_input: bool,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 8 + 8 + 16 + 1);
    data.extend_from_slice(&ix::SWAP_V2);
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&other_amount_threshold.to_le_bytes());
    data.extend_from_slice(&sqrt_price_limit_x64.to_le_bytes());
    data.push(is_base_input as u8);
    data
}

pub fn meta(info: &AccountInfo, is_signer: bool) -> AccountMeta {
    AccountMeta {
        pubkey: info.key(),
        is_signer,
        is_writable: info.is_writable,
    }
}

/// CPI в Raydium CLMM с подписью PDA vault. `accounts` — в порядке,
/// который ожидает инструкция Raydium; `signer` — ключи, которые должны
/// быть подписантами внутри CPI (наш PDA и, например, новый NFT-mint).
pub fn invoke_clmm<'info>(
    program: &AccountInfo<'info>,
    accounts: &[AccountInfo<'info>],
    signers: &[Pubkey],
    data: Vec<u8>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let metas = accounts
        .iter()
        .map(|a| meta(a, signers.contains(a.key)))
        .collect();
    let ix = Instruction {
        program_id: program.key(),
        accounts: metas,
        data,
    };
    let mut infos = accounts.to_vec();
    infos.push(program.clone());
    invoke_signed(&ix, &infos, signer_seeds)?;
    Ok(())
}
