use crate::cdn::Cache;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

const OK_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: text/plain\r\n\
Content-Length: 6\r\n\
\r\n\
local\
\n";
const NOT_FOUND_RESPONSE: &[u8] = b"HTTP/1.1 404 Not Found\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Length: 0\r\n\
\r\n";

pub async fn handle_local_request(stream: &mut TlsStream<TcpStream>) {
    let mut buf = [0; 22];
    if stream.read_exact(&mut buf).await.is_ok() {
        match &buf {
            b"GET /update HTTP/1.1\r\n" => {
                if let Err(err) = Cache::update().await {
                    let trace = format!("{:?}", err);
                    let body = trace.as_bytes();
                    let text = format!(
                        "\
HTTP/1.1 500 Internal Server Error\r\n\
Cache-Control: no-store\r\n\
Connection: close\r\n\
Content-Type: text/plain\r\n\
Content-Length: {}\r\n\
\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(text.as_bytes()).await;
                    let _ = stream.write_all(body).await;
                } else {
                    let _ = stream.write_all(OK_RESPONSE).await;
                }
            }
            _ => {
                let _ = stream.write_all(NOT_FOUND_RESPONSE).await;
            }
        }
    }
}
