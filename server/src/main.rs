use futures::{AsyncWriteExt, StreamExt};
use rustls::ServerConfig;
use rustls_acme::caches::DirCache;
use rustls_acme::{AcmeAcceptor, AcmeConfig};
use smol::net::TcpListener;
use smol::spawn;
use std::net::Ipv6Addr;
use std::sync::Arc;

#[smol_potat::main]
async fn main() {
    let mut state = AcmeConfig::new(vec!["packurl.net", "www.packurl.net"])
        .contact(vec!["mailto:programingjd@gmail.com"])
        .cache_option(Some(DirCache::new(".")))
        .directory_lets_encrypt(false)
        .state();
    let rustls_config = ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_cert_resolver(state.resolver());
    let acceptor = state.acceptor();

    spawn(async move {
        loop {
            match state.next().await.unwrap() {
                Ok(ok) => println!("event: {:?}", ok),
                Err(err) => println!("error: {:?}", err),
            }
        }
    })
    .detach();

    serve(acceptor, Arc::new(rustls_config), 8081).await;
}

async fn serve(acceptor: AcmeAcceptor, rustls_config: Arc<ServerConfig>, port: u16) {
    let listener = TcpListener::bind((Ipv6Addr::UNSPECIFIED, port))
        .await
        .unwrap();

    while let Some(tcp) = listener.incoming().next().await {
        let rustls_config = rustls_config.clone();
        let accept_future = acceptor.accept(tcp.unwrap());

        spawn(async move {
            match accept_future.await.unwrap() {
                None => println!("received TLS-ALPN-01 validation request"),
                Some(start_handshake) => {
                    let mut tls = start_handshake.into_stream(rustls_config).await.unwrap();
                    tls.write_all(HELLO).await.unwrap();
                    tls.close().await.unwrap();
                }
            }
        })
        .detach();
    }
}

const HELLO: &'static [u8] = br#"HTTP/1.1 200 OK
Content-Length: 10
Content-Type: text/plain; charset=utf-8
Hello Tls!"#;
