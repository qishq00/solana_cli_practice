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
}
