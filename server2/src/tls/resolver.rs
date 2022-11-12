use crate::acme::{get_certificate, get_challenge_key};
use crate::domains::{ACME_DOMAINS, SELF_SIGNED_DOMAINS};
use crate::tls::ALPN_ACME_TLS;
use crate::LogLevel;
use colored::Colorize;
use rcgen::generate_simple_self_signed;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::{any_supported_type, CertifiedKey};
use rustls::{Certificate, PrivateKey};
use std::io::{Error, ErrorKind};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time::Instant;

const MAX_DURATION: Duration = Duration::from_secs(90_000); // a little over 1 day

pub struct CertResolver {
    acme: RwLock<Option<(Arc<CertifiedKey>, Instant)>>,
    self_signed: Arc<CertifiedKey>,
}

impl CertResolver {
    pub fn try_new() -> Result<Self, Error> {
        LogLevel::Info.log(|| println!("Creating self-signed certificates"));
        let self_signed = generate_simple_self_signed(SELF_SIGNED_DOMAINS)
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
            acme: RwLock::new(None),
            self_signed: Arc::new(key),
        })
    }
}

impl CertResolver {
    fn update_certificate(&self) -> Option<Arc<CertifiedKey>> {
        if let Some((key, instant)) = get_certificate() {
            LogLevel::Debug.log(|| println!("Certificate found"));
            if let Some(mut lock) = self.acme.write().ok() {
                let key = Arc::new(key);
                let _ = lock.replace((key.clone(), instant));
                LogLevel::Debug.log(|| println!("Certificate loaded"));
                Some(key)
            } else {
                LogLevel::Debug.log(|| println!("Certificate could not be loaded"));
                None
            }
        } else {
            LogLevel::Debug.log(|| println!("Certificate not found"));
            None
        }
    }
}

impl ResolvesServerCert for CertResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        if let Some(sni) = client_hello.server_name() {
            if ACME_DOMAINS.find(sni).is_some() {
                if client_hello
                    .alpn()
                    .and_then(|mut it| it.find(|&it| it == ALPN_ACME_TLS))
                    .is_some()
                {
                    LogLevel::Debug
                        .log(|| println!("Looking for unsigned certificate for {}", sni.purple()));
                    if let Some(key) = get_challenge_key(sni) {
                        LogLevel::Debug.log(|| println!("Certificate found"));
                        Some(Arc::new(key))
                    } else {
                        LogLevel::Debug.log(|| println!("Certificate not found"));
                        None
                    }
                } else {
                    if let Some(inner) = self.acme.read().ok() {
                        if let Some(inner) = inner.clone() {
                            let key = inner.0.clone();
                            let elapsed = inner.1.elapsed();
                            drop(inner);
                            if elapsed > MAX_DURATION {
                                LogLevel::Debug.log(|| {
                                    println!("Updating issued certificate for {}", sni.purple())
                                });
                                let _ = self.update_certificate();
                            }
                            Some(key)
                        } else {
                            drop(inner);
                            LogLevel::Debug.log(|| {
                                println!("Looking for issued certificate for {}", sni.purple())
                            });
                            self.update_certificate()
                        }
                    } else {
                        None
                    }
                }
            } else {
                Some(self.self_signed.clone())
            }
        } else {
            None
        }
    }
}
