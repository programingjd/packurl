use lazy_static::lazy_static;
use rustls::sign::CertifiedKey;
use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::io::Result;
use std::ops::Deref;
use std::rc::Rc;

const ACCOUNT_KEYS: RefCell<Option<Vec<u8>>> = RefCell::new(None);
const ACCOUNT_KID: RefCell<Option<Vec<u8>>> = RefCell::new(None);
// lazy_static! {
//     static ref CHALLENGE_KEY: Arc<RefCell<HashMap<String, CertifiedKey>>> =
//         Arc::new(RefCell::new(HashMap::new()));
// }
const CHALLENGE_KEY: RefCell<Option<HashMap<String, CertifiedKey>>> = RefCell::new(None);

pub async fn restore_account_keys() -> Option<Vec<u8>> {
    ACCOUNT_KEYS.borrow().as_ref().map(|it| it.clone())
}
pub async fn restore_account_kid() -> Option<Vec<u8>> {
    ACCOUNT_KID.borrow().as_ref().map(|it| it.clone())
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
pub fn set_challenge_key(domain: &str, key: CertifiedKey) {
    if let Some(map) = CHALLENGE_KEY.borrow_mut().as_mut() {
        map.insert(domain.to_string(), key);
    } else {
        let mut map = HashMap::new();
        map.insert(domain.to_string(), key);
        *CHALLENGE_KEY.borrow_mut() = Some(map);
    }
    match get_challenge_key(domain) {
        Some(_) => println!("ok"),
        None => println!("ko"),
    }
}
pub fn get_challenge_key(domain: &str) -> Option<CertifiedKey> {
    Ref::filter_map(CHALLENGE_KEY.borrow(), |it| {
        it.as_ref().and_then(|it| it.get(domain))
    })
    .ok()
    .map(|it| it.deref().clone())
}
