use crate::resolver::CertResolver;
use rustls::version::TLS13;
use rustls::ServerConfig;
use std::io::{Error, ErrorKind, Result};
use std::sync::Arc;

pub const ALPN_HTTP1: &'static [u8] = b"http/1.1";
pub const ALPN_ACME_TLS: &'static [u8] = b"acme-tls/1";

pub fn config() -> Result<Arc<ServerConfig>> {
    let mut config = ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&TLS13])
        .map_err(|err| Error::new(ErrorKind::Unsupported, err))?
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(CertResolver::try_new()?));
    config.alpn_protocols = vec![ALPN_HTTP1.to_vec()];
    Ok(Arc::new(config))
}

pub fn acme_config() -> Result<Arc<ServerConfig>> {
    let mut config = ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&TLS13])
        .map_err(|err| Error::new(ErrorKind::Unsupported, err))?
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(CertResolver::try_new()?));
    config.alpn_protocols = vec![ALPN_ACME_TLS.to_vec()];
    Ok(Arc::new(config))
}
