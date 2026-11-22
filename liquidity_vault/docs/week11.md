# Неделя 11 — AMM и концентрированная ликвидность (Raydium CLMM)

## Теория

**AMM с постоянным произведением** (Uniswap v2, Raydium AMM v4 / CPMM): пул хранит
резервы `x` и `y`, инвариант `x · y = k`. Цена `P = y / x`. Ликвидность размазана
по всему диапазону цен `(0, ∞)`, поэтому большая её часть почти не работает.

**Концентрированная ликвидность** (Uniswap v3, Raydium CLMM): LP выбирает диапазон
`[P_lower, P_upper]` и даёт ликвидность только в нём. Внутри диапазона позиция ведёт
себя как обычный AMM с «виртуальными» резервами, поэтому при том же капитале
глубина рынка (и доход с комиссий) намного выше.

- **Тик**: цена дискретна, `P(i) = 1.0001^i`. Каждый тик ≈ 0.01% изменения цены.
- **tick_spacing**: позицию можно открыть только на тиках, кратных spacing
  (зависит от AmmConfig/уровня комиссии: 1, 10, 60, 120…).
- **sqrt_price_x64**: пул хранит `√P` в формате Q64.64 — так проще считать.
- **Ликвидность L** для позиции `[a, b]` при текущей цене `P`:
  - `P < a` — позиция целиком в токене 0: `amount0 = L · (1/√a − 1/√b)`
  - `P > b` — целиком в токене 1: `amount1 = L · (√b − √a)`
  - `a ≤ P ≤ b` — оба токена: `amount0 = L · (1/√P − 1/√b)`, `amount1 = L · (√P − √a)`
- **Вне диапазона** позиция не работает и не получает комиссий → её нужно
  переставлять (неделя 13).
- **Tick array**: тики хранятся пачками по 60 штук в отдельных аккаунтах
  `["tick_array", pool, start_index]`; их нужно передавать в swap и в позиции.
- **Impermanent loss**: при движении цены LP фактически продаёт растущий токен;
  в CLMM эффект сильнее, т.к. ликвидность сконцентрирована.

## Аккаунты Raydium CLMM

| Аккаунт | Что хранит |
|---|---|
| `AmmConfig` | комиссия (trade_fee_rate), tick_spacing |
| `PoolState` | mint-ы и хранилища пары, `sqrt_price_x64`, `tick_current`, ликвидность |
| `ObservationState` | история цены (оракул) |
| `TickArrayState` | 60 тиков: ликвидность и комиссии на границах |
| `PersonalPositionState` | позиция LP (привязана к NFT) |

## Что сделано в проекте

1. `scripts/01-create-tokens.ts` — два своих SPL-токена VLTA / VLTB в devnet.
2. `scripts/02-create-clmm-pool.ts` — CLMM-пул Raydium через SDK v2
   (`raydium.clmm.createPool`), стартовая цена 1, начальная ликвидность в диапазоне ±10%.
3. Программа:
   - `raydium.rs` — адреса CLMM (devnet `DRayAUgENGQBKVaX8owNhgzkEDyoHTGVEGHVJT1E9pfH`,
     mainnet `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK`) и чтение `PoolState`
     по смещениям (проверка owner + discriminator);
   - `init_quote_vault` — хранилище второго токена;
   - `set_pool` — привязка vault к пулу с проверкой, что пара токенов совпадает.
4. `scripts/03-setup-vault.ts` — инициализация vault и привязка к пулу.

## Запуск

    solana config set --url devnet && solana airdrop 2
    anchor build && anchor deploy --provider.cluster devnet
    npm install
    npm run w11:tokens && npm run w11:pool && npm run w11:vault
