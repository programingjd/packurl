use rcgen::generate_simple_self_signed;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::{any_supported_type, CertifiedKey};
use rustls::{Certificate, PrivateKey};
use std::io::{Error, ErrorKind};
use std::sync::Arc;

pub struct CertResolver {
    //acme: Arc<CertResolver>,
    self_signed: Arc<CertifiedKey>,
}

pub const APEX: &'static str = "packurl.net";
pub const WWW: &'static str = "www.packurl.net";
pub const CDN: &'static str = "cdn.packurl.net";

impl CertResolver {
    pub fn try_new() -> Result<Self, Error> {
        let self_signed = generate_simple_self_signed(vec![
            "localhost".to_string(),
            "cdn.packurl.net".to_string(),
        ])
        .map_err(|err| Error::new(ErrorKind::Unsupported, err))?;
        let private_key = PrivateKey(self_signed.serialize_private_key_der());
        let key = CertifiedKey {
            cert: vec![Certificate(
                self_signed
                    .serialize_der()
                    .map_err(|err| Error::new(ErrorKind::Unsupported, err))?,
            )],
            key: any_supported_type(&private_key)
                .map_err(|err| Error::new(ErrorKind::Unsupported, err))?,
            ocsp: None,
            sct_list: None,
        };
        Ok(CertResolver {
            self_signed: Arc::new(key),
        })
    }
}

impl ResolvesServerCert for CertResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        if let Some(sni) = client_hello.server_name() {
            match sni {
                CDN => Some(self.self_signed.clone()),
                APEX | WWW => None,
                _ => None,
            }
        } else {
            None
        }
    }
}
