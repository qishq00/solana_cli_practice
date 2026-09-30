# Solana CLI Practice

Небольшая CLI-программа на Rust: мини-кошелёк с аккаунтами и балансами.
Учебный проект, где используются конструкции, типичные для разработки Solana-программ.

## Запуск

    cargo run -- create alice
    cargo run -- deposit alice 100
    cargo run -- transfer alice bob 30
    cargo run -- balance alice

Состояние сохраняется в файле `accounts.json`.

## Тесты

    cargo test

## Использованные конструкции и их аналог в Solana

- `struct Account` — состояние аккаунта (account data)
- `enum Instruction` с данными — инструкции программы
- `match` в `processor.rs` — диспетчеризация инструкций (`process_instruction`)
- `Result` и оператор `?` — обработка ошибок
- собственный `enum WalletError` — custom program errors
- `Option` / `ok_or` — безопасная работа с отсутствующими аккаунтами
- `&mut` и заимствование — изменение состояния без копирования
- `HashMap` — хранение аккаунтов
- модули (`mod`) — структура проекта как в Solana-программах
- `serde` — сериализация данных аккаунтов
- проверки перед изменениями — как валидация в Solana