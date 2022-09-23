mod resolver;
mod response;
use resolver::{CertResolver, APEX, CDN, WWW};
use response::CDN_RESPONSE;
use rustls::server::Acceptor;
use rustls::version::TLS13;
use std::io;
use std::io::{Error, ErrorKind};
use std::net::Ipv6Addr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::{LazyConfigAcceptor, TlsAcceptor};

const PORT: u16 = 443;

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = Arc::new(
        rustls::ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(&[&TLS13])
            .map_err(|err| io::Error::new(io::ErrorKind::Unsupported, err))?
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(CertResolver::try_new()?)),
    );
    //let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind((Ipv6Addr::UNSPECIFIED, PORT)).await?;
    loop {
        if let Ok((tcp, remote_addr)) = listener.accept().await {
            let acceptor = LazyConfigAcceptor::new(
                Acceptor::new().map_err(|err| Error::new(ErrorKind::Unsupported, err))?,
                tcp,
            );
            let config = config.clone();
            let future = async move {
                if let Ok(startHandshake) = acceptor.await {
                    let client_hello = startHandshake.client_hello();
                    if let Some(sni) = client_hello.server_name() {
                        let server_name = sni.clone().to_string();
                        if let Ok(mut stream) = startHandshake.into_stream(config).await {
                            match server_name.as_str() {
                                CDN => {
                                    let _ = stream.write_all(CDN_RESPONSE).await;
                                }
                                APEX | WWW => {
                                    let mut buf = [0; 16];
                                    if stream.read_exact(&mut buf).await.is_ok() {}
                                }
                                _ => unreachable!(),
                            }
                            let _ = stream.shutdown().await;
                        }
                    }
                }
            };
            tokio::spawn(async move {
                let _ = future.await;
            });
        }
    }
}
