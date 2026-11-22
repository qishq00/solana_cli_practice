# Liquidity Vault (Anchor)

Сквозной проект курса: DeFi-vault, который принимает токены пользователей
и размещает их в пуле концентрированной ликвидности Raydium CLMM.

## Сборка и тесты

    anchor build
    anchor keys sync
    anchor test

## Прогресс по неделям

- [x] Неделя 8 — каркас: config, PDA пользователя, share-токен ([docs/week08.md](docs/week08.md))
- [x] Неделя 9 — депозит и share-токены ([docs/week09.md](docs/week09.md))
- [x] Неделя 10 — CPI и вывод из vault ([docs/week10.md](docs/week10.md))
- [x] Неделя 11 — AMM, Raydium CLMM, свои токены и пул в devnet ([docs/week11.md](docs/week11.md))
- [x] Неделя 12 — swap через Raydium CLMM со slippage ([docs/week12.md](docs/week12.md))
- [x] Неделя 13 — позиция в CLMM: открыть, комиссии, закрыть, перестановка ([docs/week13.md](docs/week13.md))

## Архитектура

```
пользователь ──deposit──► token_vault (PDA) ──► share-токены пользователю
             ◄─withdraw── (burn shares, transfer с подписью PDA)

админ-стратегия:
  token_vault / quote_vault ──swap (CPI swap_v2)──► Raydium CLMM pool
  token_vault + quote_vault ──open_position (CPI)──► позиция (NFT у vault_authority)
                            ◄─collect_fees / decrease / close
```

| Файл | Содержимое |
|---|---|
| `programs/liquidity_vault/src/state.rs` | `VaultConfig`, `UserPosition` |
| `programs/liquidity_vault/src/math.rs` | расчёт долей |
| `programs/liquidity_vault/src/raydium.rs` | адреса, чтение `PoolState`, сборка CPI в Raydium CLMM |
| `programs/liquidity_vault/src/instructions/` | инструкции по неделям |
| `scripts/` | devnet-скрипты (токены, пул, swap, позиция) |
| `tests/` | anchor-тесты депозита/вывода |
