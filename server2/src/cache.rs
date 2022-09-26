use rustls::sign::CertifiedKey;
use std::cell::RefCell;
use std::io::Result;

const ACCOUNT_KEYS: RefCell<Option<Vec<u8>>> = RefCell::new(None);
const ACCOUNT_KID: RefCell<Option<Vec<u8>>> = RefCell::new(None);
const CHALLENGE_KEY: RefCell<Option<CertifiedKey>> = RefCell::new(None);

pub async fn restore_account_keys() -> Option<Vec<u8>> {
    ACCOUNT_KEYS.into_inner()
}
pub async fn restore_account_kid() -> Option<Vec<u8>> {
    ACCOUNT_KID.into_inner()
}
pub async fn backup_account_keys(bytes: &[u8]) -> Result<()> {
    ACCOUNT_KEYS.replace(Some(bytes.to_vec()));
    ACCOUNT_KID.replace(None);
    Ok(())
}
pub async fn backup_account_kid(bytes: &[u8]) -> Result<()> {
    ACCOUNT_KID.replace(Some(bytes.to_vec()));
    Ok(())
}
pub fn set_challenge_key(key: CertifiedKey) {
    CHALLENGE_KEY.replace(Some(key));
}
pub fn get_challenge_key() -> Option<CertifiedKey> {
    CHALLENGE_KEY.into_inner()
}
