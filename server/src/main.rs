use brotli::enc::backward_references::BrotliEncoderMode::{BROTLI_MODE_GENERIC, BROTLI_MODE_TEXT};
use brotli::enc::BrotliEncoderParams;
use brotli::BrotliCompress;
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use lazy_static::lazy_static;
use rustls::ServerConfig;
use rustls_acme::caches::DirCache;
use rustls_acme::{AcmeAcceptor, AcmeConfig};
use smol::net::TcpListener;
use smol::spawn;
use std::fs;
use std::net::Ipv6Addr;
use std::path::Path;
use std::sync::Arc;

#[smol_potat::main]
async fn main() {
    let mut state = AcmeConfig::new(vec!["packurl.net", "www.packurl.net"])
        .contact(vec!["mailto:programingjd@gmail.com"])
        .cache_option(Some(DirCache::new(".")))
        .directory_lets_encrypt(true)
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

    serve(acceptor, Arc::new(rustls_config), 443).await;
}

lazy_static! {
    static ref CONTENT_TOO_LARGE_RESPONSE: Vec<u8> = {
        let path = Path::new("./www/414.html");
        let size = path.metadata().unwrap().len() as usize;
        let mut file = fs::File::open(path).unwrap();
        let mut vec: Vec<u8> = Vec::new();
        vec.append(
            &mut b"HTTP/1.1 414 URI Too Long\r\n\
Cache-Control: no-store\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: text/html\r\n"
                .to_vec(),
        );
        vec.append(
            &mut format!("Content-Length: {}\r\n\r\n", size)
                .as_bytes()
                .to_vec(),
        );
        BrotliCompress(
            &mut file,
            &mut vec,
            &BrotliEncoderParams {
                quality: 11,
                size_hint: size,
                mode: BROTLI_MODE_TEXT,
                ..BrotliEncoderParams::default()
            },
        )
        .unwrap();
        vec
    };
    static ref FAVICON_RESPONSE: Vec<u8> = {
        let path = Path::new("./www/favicon.ico");
        let size = path.metadata().unwrap().len() as usize;
        let mut file = fs::File::open(path).unwrap();
        let mut vec: Vec<u8> = Vec::new();
        vec.append(
            &mut b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: image/x-icon\r\n"
                .to_vec(),
        );
        vec.append(
            &mut format!("Content-Length: {}\r\n\r\n", size)
                .as_bytes()
                .to_vec(),
        );
        BrotliCompress(
            &mut file,
            &mut vec,
            &BrotliEncoderParams {
                quality: 11,
                size_hint: size,
                mode: BROTLI_MODE_GENERIC,
                ..BrotliEncoderParams::default()
            },
        )
        .unwrap();
        vec
    };
    static ref ROOT_RESPONSE: Vec<u8> = {
        let path = Path::new("./www/index.html");
        let size = path.metadata().unwrap().len() as usize;
        let mut file = fs::File::open(path).unwrap();
        let mut vec: Vec<u8> = Vec::new();
        vec.append(
            &mut b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: text/html\r\n"
                .to_vec(),
        );
        vec.append(
            &mut format!("Content-Length: {}\r\n\r\n", size)
                .as_bytes()
                .to_vec(),
        );
        BrotliCompress(
            &mut file,
            &mut vec,
            &BrotliEncoderParams {
                quality: 11,
                size_hint: size,
                mode: BROTLI_MODE_TEXT,
                ..BrotliEncoderParams::default()
            },
        )
        .unwrap();
        vec
    };
    static ref MANIFEST_RESPONSE: Vec<u8> = {
        let path = Path::new("./www/pwa.json");
        let size = path.metadata().unwrap().len() as usize;
        let mut file = fs::File::open(path).unwrap();
        let mut vec: Vec<u8> = Vec::new();
        vec.append(
            &mut b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: application/manifest+json\r\n"
                .to_vec(),
        );
        vec.append(
            &mut format!("Content-Length: {}\r\n\r\n", size)
                .as_bytes()
                .to_vec(),
        );
        BrotliCompress(
            &mut file,
            &mut vec,
            &BrotliEncoderParams {
                quality: 11,
                size_hint: size,
                mode: BROTLI_MODE_TEXT,
                ..BrotliEncoderParams::default()
            },
        )
        .unwrap();
        vec
    };
    static ref SERVICE_WORKER_RESPONSE: Vec<u8> = {
        let path = Path::new("./www/sw.mjs");
        let size = path.metadata().unwrap().len() as usize;
        let mut file = fs::File::open(path).unwrap();
        let mut vec: Vec<u8> = Vec::new();
        vec.append(
            &mut b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: application/javascript\r\n"
                .to_vec(),
        );
        vec.append(
            &mut format!("Content-Length: {}\r\n\r\n", size)
                .as_bytes()
                .to_vec(),
        );
        BrotliCompress(
            &mut file,
            &mut vec,
            &BrotliEncoderParams {
                quality: 11,
                size_hint: size,
                mode: BROTLI_MODE_TEXT,
                ..BrotliEncoderParams::default()
            },
        )
        .unwrap();
        vec
    };
}
const GET_REQUEST_PREFIX: &[u8] = b"GET ";
const ROOT_REQUEST_PREFIX: &[u8; 16] = b"GET / HTTP/1.1\r\n"; // 16 bytes long
const SERVICE_WORKER_REQUEST_PREFIX: &[u8; 16] = b"GET /sw.mjs HTTP";
const MANIFEST_REQUEST_PREFIX: &[u8; 16] = b"GET /pwa.json HT";
const FAVICON_REQUEST_PREFIX: &[u8; 16] = b"GET /favicon.ico";
const METHOD_NOT_ALLOWED_RESPONSE: &[u8] = b"HTTP/1.1 405 Method Not Allowed\r\n\
Allow: GET\r\n\
Connection: close\r\n\
Content-Length: 0\r\n
\r\n";

async fn serve(acceptor: AcmeAcceptor, rustls_config: Arc<ServerConfig>, port: u16) {
    let content_too_large_response = &CONTENT_TOO_LARGE_RESPONSE;
    let root_response = &ROOT_RESPONSE;
    let favicon_response = &FAVICON_RESPONSE;
    let service_worker_response = &SERVICE_WORKER_RESPONSE;
    let manifest_response = &MANIFEST_RESPONSE;
    let listener = TcpListener::bind((Ipv6Addr::UNSPECIFIED, port))
        .await
        .unwrap();
    loop {
        if let Some(Ok(tcp)) = listener.incoming().next().await {
            let rustls_config = rustls_config.clone();
            let accept = acceptor.accept(tcp);
            spawn(async move {
                if let Ok(Some(handshake)) = accept.await {
                    if let Ok(mut tls) = handshake.into_stream(rustls_config).await {
                        let mut buf = [0u8; 16];
                        if tls.read_exact(&mut buf).await.is_ok() {
                            match &buf {
                                ROOT_REQUEST_PREFIX => {
                                    let _ = tls.write_all(root_response).await;
                                }
                                FAVICON_REQUEST_PREFIX => {
                                    let _ = tls.write_all(favicon_response).await;
                                }
                                SERVICE_WORKER_REQUEST_PREFIX => {
                                    let _ = tls.write_all(service_worker_response).await;
                                }
                                MANIFEST_REQUEST_PREFIX => {
                                    let _ = tls.write_all(manifest_response).await;
                                }
                                _ => {
                                    if buf.starts_with(GET_REQUEST_PREFIX) {
                                        let _ = tls.write_all(content_too_large_response).await;
                                    } else {
                                        let _ = tls.write_all(METHOD_NOT_ALLOWED_RESPONSE).await;
                                    }
                                }
                            }
                        }
                        let _ = tls.close().await;
                    }
                }
            })
            .detach();
        }
    }
}
