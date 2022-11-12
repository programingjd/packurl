use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

const METHOD_NOT_ALLOWED_RESPONSE: &[u8] = b"HTTP/1.1 405 Method Not Allowed\r\n\
Cache-Control: no-cache\r\n\
Allow: GET\r\n\
Connection: close\r\n\
Content-Length: 0\r\n\
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload\r\n\
\r\n";
const ROOT_REDIRECT_RESPONSE: &[u8] = b"HTTP/1.1 308 Permanent Redirect\r\n\
Cache-Control: immutable\r\n\
Connection: close\r\n\
Location: https://cdn.packurl.net\r\n\
\r\n";
const CONTENT_TOO_LARGE_RESPONSE: &[u8] = const_str::concat_bytes!(
    b"HTTP/1.1 414 URI Too Long\r\n\
Cache-Control: no-store\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: text/html\r\n\
Content-Length: 915\r\n\
X-Content-Type-Options: nosniff\r\n\
X-Frame-Options: DENY\r\n\
X-XSS-Protection: 1; mode=block\r\n\
Cross-Origin-Resource-Policy: same-origin\r\n\
Cross-Origin-Embedder-Policy: require-corp\r\n\
Cross-Security-Policy: default-src 'self' 'unsafe-inline'; worker-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'\r\n\
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload\r\n\
\r\n",
    include_bytes!("414.html.br")
);
const SERVICE_WORKER_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: immutable\r\n\
Connection: close\r\n\
Content-Type: application/javascript\r\n\
Content-Length: 39\r\n\
Service-Worker-Allowed: /\r\n\
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload\r\n\
\r\n\
import 'https://cdn.packurl.net/sw.mjs'\
\n";
const MANIFEST_RESPONSE: &[u8] = const_str::concat_bytes!(
    b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: application/manifest+json\r\n\
Content-Length: 302\r\n\
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload\r\n\
\r\n\
",
    include_bytes!("pwa.json.br")
);
const FAVICON_RESPONSE: &[u8] = const_str::concat_bytes!(
    b"HTTP/1.1 200 OK\r\n\
Cache-Control: immutable\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: image/x-icon\r\n\
Content-Length: 443\r\n\
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload\r\n\
\r\n\
",
    include_bytes!("fav.ico.br")
);
const HTTPS_REDIRECT_RESPONSE: &[u8] = b"HTTP/1.1 301 Moved Permanently\r\n\
Location: https://packurl.net\r\n\
Connection: close\r\n\
Content-Length: 0\r\n\
\r\n\
";

pub async fn handle_redirect_to_https(stream: &mut TcpStream) {
    let _ = stream.write_all(HTTPS_REDIRECT_RESPONSE).await;
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
