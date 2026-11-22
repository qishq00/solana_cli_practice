//! Математика долей vault. Чистые функции без зависимостей от Anchor —
//! покрыты unit-тестами (`cargo test`).

/// Сколько share-токенов выпустить за депозит `amount`.
///
/// Первый депозит — 1:1. Дальше — пропорционально:
/// `shares = amount * total_shares / total_assets` (округление вниз — в пользу vault).
pub fn shares_for_deposit(amount: u64, total_assets: u64, total_shares: u64) -> Option<u64> {
    if total_shares == 0 || total_assets == 0 {
        return Some(amount);
    }
    let shares = (amount as u128)
        .checked_mul(total_shares as u128)?
        .checked_div(total_assets as u128)?;
    u64::try_from(shares).ok()
}

/// Сколько токенов вернуть за сжигание `shares`.
///
/// `amount = shares * total_assets / total_shares` (округление вниз — в пользу vault).
pub fn assets_for_shares(shares: u64, total_assets: u64, total_shares: u64) -> Option<u64> {
    if total_shares == 0 {
        return None;
    }
    let amount = (shares as u128)
        .checked_mul(total_assets as u128)?
        .checked_div(total_shares as u128)?;
    u64::try_from(amount).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_deposit_is_one_to_one() {
        assert_eq!(shares_for_deposit(1_000, 0, 0), Some(1_000));
    }

    #[test]
    fn proportional_deposit() {
        // в vault 1000 токенов и 1000 долей, доход +100 → цена доли 1.1
        assert_eq!(shares_for_deposit(110, 1_100, 1_000), Some(100));
        // округление вниз
        assert_eq!(shares_for_deposit(100, 1_100, 1_000), Some(90));
    }

    #[test]
    fn withdraw_returns_profit() {
        assert_eq!(assets_for_shares(100, 1_100, 1_000), Some(110));
        assert_eq!(assets_for_shares(1_000, 1_100, 1_000), Some(1_100));
    }

    #[test]
    fn no_overflow_on_big_numbers() {
        assert_eq!(shares_for_deposit(u64::MAX, u64::MAX, u64::MAX), Some(u64::MAX));
        assert_eq!(assets_for_shares(1, 1, 0), None);
    }
}
