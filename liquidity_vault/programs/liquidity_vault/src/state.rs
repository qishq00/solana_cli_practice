use anchor_lang::prelude::*;

/// Аккаунт настроек vault.
/// PDA: ["vault", token_mint]
#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    /// Администратор vault.
    pub admin: Pubkey,
    /// Mint токена, который принимает vault.
    pub token_mint: Pubkey,
    /// Mint share-токена (доли пользователей). PDA: ["share_mint", config]
    pub share_mint: Pubkey,
    /// Токен-аккаунт хранилища. PDA: ["token_vault", config]
    pub token_vault: Pubkey,
    /// Сколько токенов всего внесено (учётная величина).
    pub total_deposited: u64,
    /// Максимальный объём хранилища (0 — без лимита).
    pub deposit_cap: u64,
    /// Пауза депозитов/выводов.
    pub paused: bool,
    pub bump: u8,
    /// Bump PDA ["authority", config] — подписывает от имени vault.
    pub authority_bump: u8,
    pub share_mint_bump: u8,
    pub token_vault_bump: u8,

    // ---------- Неделя 11: пул Raydium CLMM ----------
    /// Второй токен пары (quote).
    pub quote_mint: Pubkey,
    /// Хранилище quote-токена. PDA: ["quote_vault", config]
    pub quote_vault: Pubkey,
    /// Пул Raydium CLMM, в который vault размещает ликвидность.
    pub clmm_pool: Pubkey,
    /// Программа Raydium CLMM (devnet или mainnet).
    pub clmm_program: Pubkey,
}

/// Позиция пользователя в vault.
/// PDA: ["user", config, owner]
#[account]
#[derive(InitSpace)]
pub struct UserPosition {
    pub owner: Pubkey,
    pub vault: Pubkey,
    /// Сколько токенов пользователь внёс за всё время.
    pub deposited: u64,
    /// Сколько токенов пользователь вывел за всё время.
    pub withdrawn: u64,
    /// Текущее количество share-токенов пользователя (учёт).
    pub shares: u64,
    pub bump: u8,
}
