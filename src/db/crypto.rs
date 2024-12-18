use aes_gcm::{aead::{Aead, KeyInit}, AeadCore, Aes256Gcm, Key as AesKey};
use sha2::{Digest, Sha256};


pub fn derive_key_from_password(password: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    result.into()
}

pub fn encrypt_private_key(private_key: &str, password: &str) -> String {
    let encryption_key = derive_key_from_password(password);
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(&encryption_key));
    let nonce = Aes256Gcm::generate_nonce(&mut rand::thread_rng());

    let ciphertext = cipher
        .encrypt(&nonce, private_key.as_bytes())
        .expect("encryption failure!");

    let mut combined = nonce.to_vec();
    combined.extend(ciphertext);

    hex::encode(combined)
}

pub fn decrypt_private_key(encrypted: &str, password: &str) -> eyre::Result<String> {
    let encrypted_data = hex::decode(encrypted)
        .map_err(|_| eyre::eyre!("Invalid hex data"))?;

    if encrypted_data.len() < 12 {
        return Err(eyre::eyre!("Invalid encrypted data"));
    }

    let encryption_key = derive_key_from_password(password);
    let (nonce, ciphertext) = encrypted_data.split_at(12);
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(&encryption_key));

    let plaintext = cipher
        .decrypt(nonce.into(), ciphertext)
        .map_err(|_| eyre::eyre!("Decryption failed"))?;

    String::from_utf8(plaintext)
        .map_err(|_| eyre::eyre!("Invalid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::constants::DEFAULT_PASSWORD;

    #[test]
    fn test_encryption_decryption() {
        // 测试数据
        let private_key = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        // 测试默认密码
        let encrypted = encrypt_private_key(private_key, DEFAULT_PASSWORD);
        println!("Encrypted (hex): {}", encrypted);  // 打印加密后的 hex 字符串
        let decrypted = decrypt_private_key(&encrypted, DEFAULT_PASSWORD).unwrap();
        assert_eq!(private_key, decrypted);

        // 测试自定义密码
        let custom_password = "my_custom_password";
        let encrypted = encrypt_private_key(private_key, custom_password);
        println!("Encrypted with custom password (hex): {}", encrypted);
        let decrypted = decrypt_private_key(&encrypted, custom_password).unwrap();
        assert_eq!(private_key, decrypted);

        // 测试错误密码
        let wrong_password = "wrong_password";
        let result = decrypt_private_key(&encrypted, wrong_password);
        assert!(result.is_err());

        // 测试无效的加密数据
        let invalid_data = "invalid_hex_data";
        let result = decrypt_private_key(invalid_data, DEFAULT_PASSWORD);
        assert!(result.is_err());
    }

    #[test]
    fn test_key_derivation() {
        // 测试相同密码生成相同密钥
        let password = "test_password";
        let key1 = derive_key_from_password(password);
        let key2 = derive_key_from_password(password);
        assert_eq!(key1, key2);

        // 测试不同密码生成不同密钥
        let password2 = "different_password";
        let key3 = derive_key_from_password(password2);
        assert_ne!(key1, key3);
    }
}