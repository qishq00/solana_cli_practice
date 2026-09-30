mod error;
mod instruction;
mod processor;
mod state;
mod storage;

use std::env;
use error::WalletError;
use instruction::Instruction;

fn parse(args: &[String]) -> Result<Instruction, WalletError> {
    let arg = |i: usize| args.get(i).cloned().ok_or(WalletError::InvalidArguments);
    let amount = |i: usize| -> Result<u64, WalletError> {
        arg(i)?.parse().map_err(|_| WalletError::InvalidArguments)
    };

    match args.get(1).map(|s| s.as_str()) {
        Some("create") => Ok(Instruction::Create { owner: arg(2)? }),
        Some("deposit") => Ok(Instruction::Deposit { owner: arg(2)?, amount: amount(3)? }),
        Some("transfer") => Ok(Instruction::Transfer {
            from: arg(2)?,
            to: arg(3)?,
            amount: amount(4)?,
        }),
        Some("balance") => Ok(Instruction::Balance { owner: arg(2)? }),
        _ => Err(WalletError::InvalidArguments),
    }
}

fn run() -> Result<String, WalletError> {
    let args: Vec<String> = env::args().collect();
    let ix = parse(&args)?;
    let mut accounts = storage::load()?;
    let result = processor::process(&mut accounts, ix)?;
    storage::save(&accounts)?;
    Ok(result)
}

fn main() {
    match run() {
        Ok(msg) => println!("OK: {}", msg),
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Usage: create <owner> | deposit <owner> <amount> | transfer <from> <to> <amount> | balance <owner>");
        }
    }
}