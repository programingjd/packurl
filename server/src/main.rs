use brotli::enc::backward_references::BrotliEncoderMode::{BROTLI_MODE_GENERIC, BROTLI_MODE_TEXT};
use brotli::enc::BrotliEncoderParams;
use brotli::BrotliCompress;
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use lazy_static::lazy_static;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::{any_supported_type, CertifiedKey};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_acme::caches::DirCache;
use rustls_acme::{AcmeAcceptor, AcmeConfig, ResolvesServerCertAcme};
use smol::net::TcpListener;
use smol::spawn;
use std::fs;
use std::io::Read;
use std::net::Ipv6Addr;
use std::path::Path;
use std::sync::Arc;

// struct AlwaysResolvedChain {
//     key: Arc<CertifiedKey>
// }
//
// impl ResolvesServerCert for AlwaysResolvedChain {
//     fn resolve(&self, _: ClientHello) -> Option<Arc<CertifiedKey>> {
//         Some(Arc::clone(&self.key))
//     }
// }

struct CertResolver {
    acme_resolver: Arc<ResolvesServerCertAcme>,
    key: Arc<CertifiedKey>,
}

impl ResolvesServerCert for CertResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        if let Some(sni) = client_hello.server_name() {
            match sni {
                "packurl.net" | "www.packurl.net" => self.acme_resolver.resolve(client_hello),
                "cnd.packurl.net" => Some(self.key.clone()),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[smol_potat::main]
async fn main() {
    let self_signed = rcgen::generate_simple_self_signed(vec![
        "localhost".to_string(),
        "cdn.packurl.net".to_string(),
    ])
    .unwrap();
    let private_key = PrivateKey(self_signed.serialize_private_key_der());
    let key = CertifiedKey {
        cert: vec![Certificate(self_signed.serialize_der().unwrap())],
        key: any_supported_type(&private_key).unwrap(),
        ocsp: None,
        sct_list: None,
    };
    let mut state = AcmeConfig::new(vec!["packurl.net", "www.packurl.net"])
        .contact(vec!["mailto:programingjd@gmail.com"])
        .cache_option(Some(DirCache::new(".")))
        .directory_lets_encrypt(true)
        .state();
    let resolver = CertResolver {
        acme_resolver: state.resolver(),
        key: Arc::new(key),
    };
    let rustls_config = ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));

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

async fn serve(acceptor: AcmeAcceptor, rustls_config: Arc<ServerConfig>, port: u16) {
    let content_too_large_response = &CONTENT_TOO_LARGE_RESPONSE;
    let root_response = &ROOT_RESPONSE;
    let favicon_ico_response = &FAVICON_ICO_RESPONSE;
    let favicon_png_response = &FAVICON_PNG_RESPONSE;
    let favicon_svg_response = &FAVICON_SVG_RESPONSE;
    let favicon_maskable_svg_response = &FAVICON_SVG_MASKABLE_RESPONSE;
    let touchicon_response = &TOUCHICON_RESPONSE;
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
                        if let Some(sni) = tls.get_ref().1.sni_hostname() {
                            match sni {
                                "www.packurl.net" | "packurl.net" => {
                                    let mut buf = [0u8; 16];
                                    if tls.read_exact(&mut buf).await.is_ok() {
                                        match &buf {
                                            ROOT_REQUEST_PREFIX => {
                                                let _ = tls.write_all(root_response).await;
                                            }
                                            FAVICON_ICO_REQUEST_PREFIX => {
                                                let _ = tls.write_all(favicon_ico_response).await;
                                            }
                                            FAVICON_PNG_REQUEST_PREFIX => {
                                                let _ = tls.write_all(favicon_png_response).await;
                                            }
                                            FAVICON_SVG_REQUEST_PREFIX => {
                                                let _ = tls.write_all(favicon_svg_response).await;
                                            }
                                            FAVICON_MASKABLE_SVG_REQUEST_PREFIX => {
                                                let _ = tls
                                                    .write_all(favicon_maskable_svg_response)
                                                    .await;
                                            }
                                            TOUCHICON_REQUEST_PREFIX => {
                                                let _ = tls.write_all(touchicon_response).await;
                                            }
                                            SERVICE_WORKER_REQUEST_PREFIX => {
                                                let _ =
                                                    tls.write_all(service_worker_response).await;
                                            }
                                            MANIFEST_REQUEST_PREFIX => {
                                                let _ = tls.write_all(manifest_response).await;
                                            }
                                            _ => {
                                                if buf.starts_with(GET_REQUEST_PREFIX) {
                                                    let _ = tls
                                                        .write_all(content_too_large_response)
                                                        .await;
                                                } else {
                                                    let _ = tls
                                                        .write_all(METHOD_NOT_ALLOWED_RESPONSE)
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                }
                                "cdn.packurl.net" => {
                                    let _ = tls.write_all(CDN_RESPONSE).await;
                                }
                                _ => {}
                            }
                            let _ = tls.close().await;
                        }
                    }
                }
            })
            .detach();
        }
    }
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
    static ref FAVICON_ICO_RESPONSE: Vec<u8> = {
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
    static ref FAVICON_PNG_RESPONSE: Vec<u8> = {
        let path = Path::new("./www/favicon.png");
        let size = path.metadata().unwrap().len() as usize;
        let mut file = fs::File::open(path).unwrap();
        let mut vec: Vec<u8> = Vec::new();
        vec.append(
            &mut b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: image/pngn\r\n"
                .to_vec(),
        );
        vec.append(
            &mut format!("Content-Length: {}\r\n\r\n", size)
                .as_bytes()
                .to_vec(),
        );
        file.read_to_end(&mut vec).unwrap();
        vec
    };
    static ref FAVICON_SVG_RESPONSE: Vec<u8> = {
        let path = Path::new("./www/favicon.svg");
        let size = path.metadata().unwrap().len() as usize;
        let mut file = fs::File::open(path).unwrap();
        let mut vec: Vec<u8> = Vec::new();
        vec.append(
            &mut b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: image/svg+xml\r\n"
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
    static ref FAVICON_SVG_MASKABLE_RESPONSE: Vec<u8> = {
        let path = Path::new("./www/mask.svg");
        let size = path.metadata().unwrap().len() as usize;
        let mut file = fs::File::open(path).unwrap();
        let mut vec: Vec<u8> = Vec::new();
        vec.append(
            &mut b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: image/svg+xml\r\n"
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
    static ref TOUCHICON_RESPONSE: Vec<u8> = {
        let path = Path::new("./www/apple.png");
        let size = path.metadata().unwrap().len() as usize;
        let mut file = fs::File::open(path).unwrap();
        let mut vec: Vec<u8> = Vec::new();
        vec.append(
            &mut b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: image/pngn\r\n"
                .to_vec(),
        );
        vec.append(
            &mut format!("Content-Length: {}\r\n\r\n", size)
                .as_bytes()
                .to_vec(),
        );
        file.read_to_end(&mut vec).unwrap();
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
const FAVICON_ICO_REQUEST_PREFIX: &[u8; 16] = b"GET /favicon.ico";
const FAVICON_SVG_REQUEST_PREFIX: &[u8; 16] = b"GET /favicon.svg";
const FAVICON_PNG_REQUEST_PREFIX: &[u8; 16] = b"GET /favicon.png";
const FAVICON_MASKABLE_SVG_REQUEST_PREFIX: &[u8; 16] = b"GET /mask.svg HT";
const TOUCHICON_REQUEST_PREFIX: &[u8; 16] = b"GET /apple.svg H";
const METHOD_NOT_ALLOWED_RESPONSE: &[u8] = b"HTTP/1.1 405 Method Not Allowed\r\n\
Allow: GET\r\n\
Connection: close\r\n\
Content-Length: 0\r\n
\r\n";
const CDN_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: text/plain\r\n\
Content-Length: 3\r\n
\r\n\
cdn";
