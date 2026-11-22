# Неделя 8 — BUILD SPRINT: каркас Liquidity Vault

Старт сквозного проекта: DeFi-vault принимает токены пользователей и размещает их в пуле ликвидности.

## Что сделано

| Аккаунт | Тип | Seeds | Назначение |
|---|---|---|---|
| `VaultConfig` | PDA с данными | `["vault", token_mint]` | Аккаунт настроек: админ, mint-ы, лимит, пауза, bump-ы |
| `vault_authority` | PDA без данных | `["authority", config]` | Подписывает от имени vault (CPI) |
| `share_mint` | PDA mint | `["share_mint", config]` | Share-токен — доля пользователя в vault |
| `token_vault` | PDA token account | `["token_vault", config]` | Хранилище токенов vault |
| `UserPosition` | PDA с данными | `["user", config, owner]` | Позиция пользователя |

Инструкции: `initialize`, `init_user`, `update_config` (только админ, `has_one = admin`).

## Почему так

- Отдельный `vault_authority` без данных: он может и подписывать токен-операции, и (позже, неделя 13)
  платить ренту через System Program — у PDA с данными этого сделать нельзя.
- `mint::authority = vault_authority` — выпустить share-токены можно только через программу.
- Все bump-ы хранятся в `VaultConfig`, чтобы не пересчитывать их на каждом вызове.
