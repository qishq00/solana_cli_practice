# Неделя 12 — Swap через Raydium

## Инструкция `swap(amount_in, min_amount_out, sqrt_price_limit_x64, base_to_quote)`

Vault (только админ-стратегия) меняет свои токены через CLMM-пул Raydium.

1. Проверки: `amount_in > 0`, **`min_amount_out > 0` обязателен** — swap без
   ограничения slippage запрещён (`SlippageNotSet`).
2. Все аккаунты Raydium сверяются с `PoolState` (`raydium::read_pool`):
   `amm_config`, `observation_state`, хранилища пула для входа/выхода, mint-ы.
   Пул и программа — только те, что сохранены в `VaultConfig` на неделе 11.
3. CPI `swap_v2` (дискриминатор `sha256("global:swap_v2")[..8]`), аккаунты в порядке
   `SwapSingleV2`: payer (= PDA `vault_authority`, подпись через `invoke_signed`),
   amm_config, pool_state, input/output токен-аккаунты vault, хранилища пула,
   observation, token / token-2022 / memo программы, mint-ы;
   в `remaining_accounts` — bitmap extension и tick array-аккаунты.
4. Аргументы: `is_base_input = true` (точный вход), `other_amount_threshold = min_amount_out` —
   Raydium сам откатит транзакцию, если выйдет меньше.
5. **Двойная защита**: после CPI перечитываем балансы vault (`reload()`) и проверяем,
   что потрачено ≤ `amount_in`, получено ≥ `min_amount_out`, иначе `SlippageExceeded`.

## Как считается `min_amount_out`

Off-chain через Raydium SDK: `PoolUtils.computeAmountOutFormat` симулирует обмен
по текущим тикам пула и возвращает `amountOut`, `minAmountOut = amountOut · (1 − slippage)`
и список tick array-ов, через которые пройдёт цена.

`sqrt_price_limit_x64 = 0` — без ограничения цены (Raydium подставит MIN/MAX).

## Запуск

    npm run w12:swap -- 30000 1     # обменять 30 000 VLTA, slippage 1%

Скрипт: депозит 100 000 VLTA в vault → котировка → swap → попытка обмена с
невыполнимым минимумом (откатывается с ошибкой slippage).
