use std::fs::File;

use super::{
    account::Account,
    constants::{DB_FILE_PATH, ENCRYPTED_PRIVATE_KEYS_FILE_PATH, PROXIES_FILE_PATH, RECIPIENTS_FILE_PATH},
};
use crate::db::constants::DEFAULT_PASSWORD;
use crate::db::crypto::decrypt_private_key;
use crate::utils::files::read_file_lines;
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Database(pub Vec<Account>);

impl Database {
    async fn read_from_file(file_path: &str, password: Option<&str>) -> eyre::Result<Self> {
        let password = password.unwrap_or(DEFAULT_PASSWORD);
        let contents = tokio::fs::read_to_string(file_path).await?;
        let mut db = serde_json::from_str::<Self>(&contents)?;
        for account in &mut db.0 {
            let private_key = decrypt_private_key(&account.get_encrypted_private_key(), password)?;
            account.set_private_key(&private_key);
        }
        Ok(db)
    }

    #[allow(unused)]
    pub async fn read(password: Option<&str>) -> eyre::Result<Self> {
        Self::read_from_file(DB_FILE_PATH, password).await
    }

    pub async fn new(password: Option<&str>) -> eyre::Result<Self> {
        let password = password.unwrap_or(DEFAULT_PASSWORD);
        let encrypted_private_keys = read_file_lines(ENCRYPTED_PRIVATE_KEYS_FILE_PATH).await.unwrap();
        let proxies = read_file_lines(PROXIES_FILE_PATH).await.unwrap();
        let recipients = read_file_lines(RECIPIENTS_FILE_PATH).await.unwrap();
        let mut data = Vec::with_capacity(encrypted_private_keys.len());

        let max_len = encrypted_private_keys
            .len()
            .max(proxies.len())
            .max(recipients.len());

        for i in 0..max_len {
            let encrypted_private_key = encrypted_private_keys.get(i)
                .ok_or_else(|| eyre::eyre!("Missing private key at position {}", i))?;
            // println!("Decrypting private key {} {}", encrypted_private_key, password);
            let private_key = decrypt_private_key(encrypted_private_key, password)
                .map_err(|e| eyre::eyre!("Failed to decrypt private key at position {}: {}", i, e))?;

            let proxy = proxies.get(i).cloned();
            let recipient = recipients.get(i).cloned();
            let account = Account::new(&private_key, &encrypted_private_key, proxy, recipient);
            data.push(account);
        }

        let db_file = File::create(DB_FILE_PATH)?;
        serde_json::to_writer_pretty(db_file, &data)?;

        Ok(Self(data))
    }

    pub fn get_random_account_with_filter<F>(&mut self, filter: F) -> Option<&mut Account>
    where
        F: Fn(&Account) -> bool,
    {
        let mut rng = thread_rng();

        self.0
            .iter_mut()
            .filter(|account| filter(account))
            .choose(&mut rng)
    }

    pub fn update(&self) {
        let file = File::create(DB_FILE_PATH).expect("Default database must be vaild");
        let _ = serde_json::to_writer_pretty(file, &self);
    }

    pub fn shuffle(&mut self) {
        self.0.shuffle(&mut thread_rng());
        self.update();
    }
}
