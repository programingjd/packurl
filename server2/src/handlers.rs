use crate::response::{
    CDN_RESPONSE, CONTENT_TOO_LARGE_RESPONSE, FAVICON_RESPONSE, LOCALHOST_RESPONSE,
    MANIFEST_RESPONSE, METHOD_NOT_ALLOWED_RESPONSE, OK_RESPONSE, ROOT_REDIRECT_RESPONSE,
    SERVICE_WORKER_RESPONSE,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

pub async fn handle_localhost_request(stream: &mut TlsStream<TcpStream>) {
    let _ = stream.write_all(LOCALHOST_RESPONSE).await;
}

pub async fn handle_cdn_request(stream: &mut TlsStream<TcpStream>) {
    let _ = stream.write_all(CDN_RESPONSE).await;
}

pub async fn handle_apex_request(stream: &mut TlsStream<TcpStream>) {
    let mut buf = [0; 16];
    if stream.read_exact(&mut buf).await.is_ok() {
        match &buf {
            b"GET / HTTP/1.1\r\n" => {
                let _ = stream.write_all(ROOT_REDIRECT_RESPONSE).await;
            }
            b"GET /sw.mjs HTTP" => {
                let _ = stream.write_all(SERVICE_WORKER_RESPONSE).await;
            }
            b"GET /pwa.json HT" => {
                let _ = stream.write_all(MANIFEST_RESPONSE).await;
            }
            b"GET /favicon.ico" => {
                let _ = stream.write_all(FAVICON_RESPONSE).await;
            }
            _ => {
                if buf.starts_with(b"GET ") {
                    let _ = stream.write_all(CONTENT_TOO_LARGE_RESPONSE).await;
                } else {
                    let _ = stream.write_all(METHOD_NOT_ALLOWED_RESPONSE).await;
                }
            }
        }
    }
}
