use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Vault на паузе")]
    VaultPaused,
    #[msg("Только админ может выполнить это действие")]
    Unauthorized,
    #[msg("Сумма должна быть больше нуля")]
    ZeroAmount,
    #[msg("Арифметическое переполнение")]
    MathOverflow,
    #[msg("Превышен лимит депозитов vault")]
    DepositCapExceeded,
    #[msg("Депозит слишком мал: 0 share-токенов")]
    ZeroShares,
    #[msg("Недостаточно share-токенов")]
    InsufficientShares,
    #[msg("Получено меньше минимальной суммы (slippage)")]
    SlippageExceeded,
    #[msg("В vault недостаточно свободных токенов")]
    InsufficientLiquidity,
    #[msg("Аккаунт не является пулом Raydium CLMM или не подходит vault")]
    InvalidPool,
    #[msg("Пул vault ещё не настроен")]
    PoolNotSet,
    #[msg("Нужно задать min_amount_out (защита от slippage)")]
    SlippageNotSet,
}
