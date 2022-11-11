use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

const OK_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: text/plain\r\n\
Content-Length: 0\r\n\
\r\n";

pub async fn handle_acme_request(stream: &mut TlsStream<TcpStream>) {
    let _ = stream.write_all(OK_RESPONSE).await;
}
