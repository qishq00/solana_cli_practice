use std::fmt;

#[derive(Debug)]
pub enum WalletError {
    AccountNotFound,
    AccountAlreadyExists,
    InsufficientFunds,
    InvalidArguments,
    StorageError(String),
}

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WalletError::AccountNotFound => write!(f, "account not found"),
            WalletError::AccountAlreadyExists => write!(f, "account already exists"),
            WalletError::InsufficientFunds => write!(f, "insufficient funds"),
            WalletError::InvalidArguments => write!(f, "invalid arguments"),
            WalletError::StorageError(msg) => write!(f, "storage error: {}", msg),
        }
    }
}