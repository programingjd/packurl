use lazy_static::lazy_static;
use rustls::sign::CertifiedKey;
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::sync::RwLock;

lazy_static! {
    static ref ACCOUNT_KEYS: RwLock<Option<Vec<u8>>> = RwLock::new(None);
    static ref ACCOUNT_KID: RwLock<Option<Vec<u8>>> = RwLock::new(None);
    static ref CHALLENGE_KEY: RwLock<Option<HashMap<String, CertifiedKey>>> = RwLock::new(None);
}
//const CHALLENGE_KEY: RwLock<Option<HashMap<String, CertifiedKey>>> = RwLock::new(None);

pub async fn restore_account_keys() -> Option<Vec<u8>> {
    ACCOUNT_KEYS.read().map_or(None, |it| it.clone())
}
pub async fn restore_account_kid() -> Option<Vec<u8>> {
    ACCOUNT_KID.read().map_or(None, |it| it.clone())
}
pub async fn backup_account_keys(bytes: &[u8]) -> Result<()> {
    *ACCOUNT_KEYS
        .write()
        .map_err(|_err| Error::new(ErrorKind::Other, "Failed to write account keys."))? =
        Some(bytes.to_vec());
    *ACCOUNT_KID
        .write()
        .map_err(|_err| Error::new(ErrorKind::Other, "Failed to reset account kid."))? = None;
    Ok(())
}
pub async fn backup_account_kid(bytes: &[u8]) -> Result<()> {
    *ACCOUNT_KID
        .write()
        .map_err(|_err| Error::new(ErrorKind::Other, "Failed to write account kid."))? =
        Some(bytes.to_vec());
    Ok(())
}
pub fn set_challenge_key(domain: &str, key: CertifiedKey) -> Result<()> {
    if let Ok(mut lock) = CHALLENGE_KEY.write() {
        let option = lock.as_mut();
        if let Some(map) = option {
            map.insert(domain.to_string(), key);
        } else {
            let mut map = HashMap::new();
            map.insert(domain.to_string(), key);
            *lock = Some(map);
        }
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::Other,
            "Failed to write challenge key.",
        ))
    }
}
pub fn get_challenge_key(domain: &str) -> Option<CertifiedKey> {
    if let Ok(lock) = CHALLENGE_KEY.read() {
        let option = lock.as_ref();
        if let Some(map) = option {
            map.get(domain).map(|it| it.clone())
        } else {
            None
        }
    } else {
        None
    }
}
