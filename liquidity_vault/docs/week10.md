# Неделя 10 — CPI и вывод из vault

## Инструкция `withdraw(shares, min_amount_out)`

1. Проверки: не пауза, `shares > 0`, у пользователя достаточно долей.
2. `amount = shares * total_assets / total_shares` (`math::assets_for_shares`, округление вниз).
3. `amount >= min_amount_out` — защита пользователя от изменения цены доли между
   подписанием и исполнением транзакции.
4. CPI `token::burn` — сжигаем share-токены (подписывает пользователь).
5. CPI `token::transfer` из `token_vault` пользователю. Владелец `token_vault` — PDA
   `vault_authority`, у него нет приватного ключа, поэтому программа подписывает
   через `invoke_signed` сидами `["authority", config, bump]`:

```rust
let signer_seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, config_key.as_ref(), &[bump]]];
token::transfer(CpiContext::new_with_signer(token_program, accounts, signer_seeds), amount)?;
```

Порядок важен: сначала сжигаем доли, потом считаем/отдаём токены — расчёт сделан
по supply до сжигания, поэтому повторный вход (reentrancy) невозможен: Solana
запрещает рекурсивный CPI обратно в нашу программу через Token program.
