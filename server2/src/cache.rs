// use lazy_static::lazy_static;
use rustls::sign::CertifiedKey;
use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::RwLock;

const ACCOUNT_KEYS: RwLock<Option<Vec<u8>>> = RwLock::new(None);
const ACCOUNT_KID: RwLock<Option<Vec<u8>>> = RwLock::new(None);
// lazy_static! {
//     static ref CHALLENGE_KEY: Arc<RefCell<HashMap<String, CertifiedKey>>> =
//         Arc::new(RefCell::new(HashMap::new()));
// }
const CHALLENGE_KEY: RwLock<Option<HashMap<String, CertifiedKey>>> = RwLock::new(None);

pub async fn restore_account_keys() -> Option<Vec<u8>> {
    ACCOUNT_KEYS.read().map_or(None, |it| it.clone())
}
pub async fn restore_account_kid() -> Option<Vec<u8>> {
    ACCOUNT_KID.read().map_or(None, |it| it.clone())
}
pub async fn backup_account_keys(bytes: &[u8]) -> Result<()> {
    *ACCOUNT_KEYS
        .write()
        .map_err(|err| Error::new(ErrorKind::Other, "Failed to write account keys."))? =
        Some(bytes.to_vec());
    *ACCOUNT_KID
        .write()
        .map_err(|err| Error::new(ErrorKind::Other, "Failed to reset account kid."))? = None;
    Ok(())
}
pub async fn backup_account_kid(bytes: &[u8]) -> Result<()> {
    *ACCOUNT_KID
        .write()
        .map_err(|err| Error::new(ErrorKind::Other, "Failed to write account kid."))? =
        Some(bytes.to_vec());
    Ok(())
}
pub fn set_challenge_key(domain: &str, key: CertifiedKey) -> Result<()> {
    if let Ok(mut lock) = CHALLENGE_KEY.write() {
        let mut option = lock.as_mut();
        if let Some(map) = option {
            println!("Inserting into existing map.");
            map.insert(domain.to_string(), key);
        } else {
            println!("Inserting into new map.");
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
