use std::collections::HashMap;
use crate::{error::WalletError, instruction::Instruction, state::Account};

pub fn process(
    accounts: &mut HashMap<String, Account>,
    ix: Instruction,
) -> Result<String, WalletError> {
    match ix {
        Instruction::Create { owner } => {
            if accounts.contains_key(&owner) {
                return Err(WalletError::AccountAlreadyExists);
            }
            accounts.insert(owner.clone(), Account { owner: owner.clone(), balance: 0 });
            Ok(format!("created account for {}", owner))
        }
        Instruction::Deposit { owner, amount } => {
            let acc = accounts.get_mut(&owner).ok_or(WalletError::AccountNotFound)?;
            acc.balance += amount;
            Ok(format!("{} balance: {}", acc.owner, acc.balance))
        }
        Instruction::Transfer { from, to, amount } => {
            if !accounts.contains_key(&to) {
                return Err(WalletError::AccountNotFound);
            }
            let from_acc = accounts.get_mut(&from).ok_or(WalletError::AccountNotFound)?;
            if from_acc.balance < amount {
                return Err(WalletError::InsufficientFunds);
            }
            from_acc.balance -= amount;
            let to_acc = accounts.get_mut(&to).ok_or(WalletError::AccountNotFound)?;
            to_acc.balance += amount;
            Ok(format!("transferred {} from {} to {}", amount, from, to))
        }
        Instruction::Balance { owner } => {
            let acc = accounts.get(&owner).ok_or(WalletError::AccountNotFound)?;
            Ok(format!("{} balance: {}", acc.owner, acc.balance))
        }
    }
}#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> HashMap<String, Account> {
        let mut accounts = HashMap::new();
        process(&mut accounts, Instruction::Create { owner: "alice".into() }).unwrap();
        process(&mut accounts, Instruction::Create { owner: "bob".into() }).unwrap();
        accounts
    }

    #[test]
    fn deposit_increases_balance() {
        let mut accounts = setup();
        process(&mut accounts, Instruction::Deposit { owner: "alice".into(), amount: 100 }).unwrap();
        assert_eq!(accounts["alice"].balance, 100);
    }

    #[test]
    fn transfer_moves_funds() {
        let mut accounts = setup();
        process(&mut accounts, Instruction::Deposit { owner: "alice".into(), amount: 100 }).unwrap();
        process(&mut accounts, Instruction::Transfer {
            from: "alice".into(), to: "bob".into(), amount: 30,
        }).unwrap();
        assert_eq!(accounts["alice"].balance, 70);
        assert_eq!(accounts["bob"].balance, 30);
    }

    #[test]
    fn transfer_fails_on_insufficient_funds() {
        let mut accounts = setup();
        let result = process(&mut accounts, Instruction::Transfer {
            from: "alice".into(), to: "bob".into(), amount: 10,
        });
        assert!(matches!(result, Err(WalletError::InsufficientFunds)));
    }

    #[test]
    fn create_twice_fails() {
        let mut accounts = setup();
        let result = process(&mut accounts, Instruction::Create { owner: "alice".into() });
        assert!(matches!(result, Err(WalletError::AccountAlreadyExists)));
    }

    #[test]
    fn unknown_account_fails() {
        let mut accounts = setup();
        let result = process(&mut accounts, Instruction::Balance { owner: "zed".into() });
        assert!(matches!(result, Err(WalletError::AccountNotFound)));
    }
}
