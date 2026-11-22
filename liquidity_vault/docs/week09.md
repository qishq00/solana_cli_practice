# Неделя 9 — Vault: депозит и share-токены

## Инструкция `deposit(amount)`

1. Проверки: vault не на паузе, `amount > 0`, не превышен `deposit_cap`.
2. Считаем долю (`src/math.rs`):
   - первый депозит — 1:1;
   - дальше `shares = amount * total_shares / total_assets`,
     где `total_assets` — баланс `token_vault` **до** депозита, `total_shares` — `share_mint.supply`.
   - округление вниз, в пользу vault; если вышло 0 долей — ошибка `ZeroShares`.
3. CPI `token::transfer`: пользователь → `token_vault` (подписывает пользователь).
4. CPI `token::mint_to`: share-токены пользователю, подписывает PDA
   `vault_authority` через `invoke_signed` (`CpiContext::new_with_signer`).
5. Обновляем учёт в `VaultConfig` и `UserPosition`, эмитим `DepositEvent`.

## Пример

| Шаг | total_assets | total_shares | депозит | новые доли |
|---|---|---|---|---|
| первый | 0 | 0 | 100 | 100 |
| vault заработал +10 | 110 | 100 | 110 | 100 |

Unit-тесты математики: `cargo test -p liquidity_vault`.
