use std::{collections::HashMap, fs};
use crate::{error::WalletError, state::Account};

const FILE: &str = "accounts.json";

pub fn load() -> Result<HashMap<String, Account>, WalletError> {
    match fs::read_to_string(FILE) {
        Ok(data) => serde_json::from_str(&data)
            .map_err(|e| WalletError::StorageError(e.to_string())),
        Err(_) => Ok(HashMap::new()),
    }
}

pub fn save(accounts: &HashMap<String, Account>) -> Result<(), WalletError> {
    let data = serde_json::to_string_pretty(accounts)
        .map_err(|e| WalletError::StorageError(e.to_string()))?;
    fs::write(FILE, data).map_err(|e| WalletError::StorageError(e.to_string()))
}