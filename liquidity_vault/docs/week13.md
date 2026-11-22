# Неделя 13 — Позиция в CLMM-пуле Raydium

Vault открывает позицию концентрированной ликвидности из своих хранилищ,
собирает комиссии и переставляет позицию, когда цена выходит из диапазона.

## Инструкции программы

| Инструкция | CPI в Raydium | Что делает |
|---|---|---|
| `open_position(tick_lower, tick_upper, …, liquidity, amount_0_max, amount_1_max)` | `open_position_v2` | Открывает позицию в `[tick_lower, tick_upper)` из `token_vault`/`quote_vault`. NFT позиции принадлежит PDA `vault_authority` |
| `collect_fees()` | `decrease_liquidity_v2` c `L = 0` | Забирает накопленные комиссии в хранилища vault |
| `decrease_liquidity(L, min0, min1)` | `decrease_liquidity_v2` | Выводит ликвидность (+ комиссии) |
| `close_position()` | `close_position` | Сжигает NFT пустой позиции, рента возвращается `vault_authority` |
| `check_rebalance()` | — | Читает `tick_current` из `PoolState`; `Ok` — цена вне диапазона, ошибка `PositionInRange` — всё в порядке |

### Ключевые моменты

- **Кто платит и подписывает.** В `open_position_v2` один и тот же `payer` и платит
  ренту за NFT / personal position / tick arrays, и является authority для перевода
  токенов. Поэтому это PDA `vault_authority` **без данных** (заложено ещё на неделе 8):
  System Program разрешает переводить SOL только с аккаунтов без данных. Админ пополняет
  его SOL, а программа подписывает через `invoke_signed`. Второй подписант CPI —
  новый keypair `position_nft_mint`.
- **Проверки.** Пул и программа — только из `VaultConfig`; хранилища пула, mint-ы
  и `token_account_0/1` сверяются с `PoolState`; получатели токенов — только хранилища
  vault (`is_vault_account`); NFT-аккаунт должен принадлежать `vault_authority`
  и содержать NFT из `config.position_nft_mint`; в vault может быть только одна позиция.
- **Диапазон.** `tick_lower < tick_upper`, оба кратны `tick_spacing` пула.
  В Raydium позиция активна при `tick_lower ≤ tick_current < tick_upper`.
- **Расчёт ликвидности** off-chain: `LiquidityMath.getLiquidityFromTokenAmounts`
  по `sqrt_price` текущей цены и границ — берётся минимум из ликвидности,
  которую можно дать токеном 0 и токеном 1.

## Перестановка (rebalance)

```
check_rebalance ──(в диапазоне)──► ничего не делаем
       │
  (вне диапазона)
       ▼
collect_fees → decrease_liquidity(всё) → close_position
       ▼
swap (неделя 12) — выравниваем токены ~50/50 по стоимости
       ▼
open_position в новом диапазоне ±N% вокруг текущего тика
```

Когда цена выходит за диапазон, позиция целиком лежит в одном токене и не получает
комиссий — поэтому её закрывают и открывают новую вокруг текущей цены.

## Запуск (devnet)

    npm run w13 -- open 5          # позиция ±5% вокруг текущей цены
    npm run w13 -- status
    npm run w13 -- push-price 50000  # демо: двигаем цену крупным swap-ом с кошелька
    npm run w13 -- fees
    npm run w13 -- rebalance 5
    npm run w13 -- close

## Ограничения (учебный проект)

- `total_assets` для депозита/вывода считается по балансу `token_vault`. Токены в позиции
  и в `quote_vault` в цену доли не входят, поэтому пользователь может вывести только
  свободный остаток. Полная оценка NAV (позиция + quote по цене пула) — следующий шаг.
- Стратегией (swap / позиции) управляет админ; автоматизировать можно через keeper,
  который периодически вызывает `check_rebalance`.
