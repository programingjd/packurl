use crate::cdn::cache::{CDN_ROOT, FILES};
use crate::log::LogLevel;
use colored::Colorize;
use std::io::Result;
use std::str::from_utf8;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

const NOT_FOUND_RESPONSE: &[u8] = b"HTTP/1.1 404 Not Found\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Length: 0\r\n\
\r\n";
pub const PAYLOAD_TOO_LARGE_RESPONSE: &[u8] = b"HTTP/1.1 413 Payload Too Large\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Length: 0\r\n\
\r\n";
pub const METHOD_NOT_ALLOWED_RESPONSE: &[u8] = b"HTTP/1.1 405 Method Not Allowed\r\n\
Cache-Control: no-cache\r\n\
Allow: GET\r\n\
Connection: close\r\n\
Content-Length: 0\r\n
\r\n";
pub const BAD_REQUEST_RESPONSE: &[u8] = b"HTTP/1.1 400 Bad Request\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Length: 0\r\n
\r\n";

const MAX_REQUEST_SIZE: usize = 4096;

pub async fn handle_cdn_request(stream: &mut TlsStream<TcpStream>) {
    if let Err(err) = handle_file_request(stream).await {
        LogLevel::Warning.log(|| {
            println!("{}", "Failed to accept TLS connection".red());
            println!("{:?}", err);
        });
    }
}

async fn read_request(stream: &mut TlsStream<TcpStream>) -> Result<Option<Vec<u8>>> {
    let mut len: usize = 0;
    let mut bounded = stream.take(MAX_REQUEST_SIZE as u64);
    loop {
        let mut buf: [u8; 1600] = [0u8; 1600];
        let n = bounded.read(&mut buf[len..]).await?;
        let start = if len > 2 { len - 2 } else { len };
        len += n;
        let scan = &buf[start..len];
        if scan.len() > 3 {
            if let Some(pos) = scan
                .windows(4)
                .position(|it| it[0] == b'\r' && it[1] == b'\n' && it[2] == b'\r' && it[3] == b'\n')
            {
                return Ok(Some(buf[..pos + start].to_vec()));
            }
        }
        if scan.len() == MAX_REQUEST_SIZE {
            return Ok(None);
        }
    }
}

async fn handle_file_request(stream: &mut TlsStream<TcpStream>) -> Result<()> {
    match read_request(stream).await? {
        None => {
            let _ = stream.write_all(PAYLOAD_TOO_LARGE_RESPONSE).await;
            Ok(())
        }
        Some(bytes) => {
            println!("{}", "request length is ok".yellow());
            println!("{}", from_utf8(&bytes).unwrap());
            let bytes = bytes.as_slice();
            if let Some(pos) = bytes.iter().position(|p| *p == b'\r') {
                match bytes.get(pos + 1) {
                    Some(b'\n') => match &bytes[0..4] {
                        b"GET " => {
                            if let Ok(path) = from_utf8(&bytes[4..pos]) {
                                let path = path.replace(CDN_ROOT.as_str(), "");
                                match FILES.get(&path) {
                                    None => {
                                        let _ = stream.write_all(NOT_FOUND_RESPONSE).await;
                                        return Ok(());
                                    }
                                    Some(entry) => {
                                        let not_modified = if let Some(etag) =
                                            get_if_none_match(&bytes[pos + 2..])
                                        {
                                            &entry.etag == etag
                                        } else {
                                            false
                                        };
                                        let _ = stream
                                            .write_all(if not_modified {
                                                &entry.not_modified
                                            } else {
                                                &entry.ok
                                            })
                                            .await;
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        _ => {
                            let _ = stream.write_all(METHOD_NOT_ALLOWED_RESPONSE).await;
                            return Ok(());
                        }
                    },
                    _ => {}
                }
            }
            let _ = stream.write_all(BAD_REQUEST_RESPONSE).await;
            Ok(())
        }
    }
}

fn get_if_none_match(bytes: &[u8]) -> Option<&str> {
    let mut bytes = bytes;
    loop {
        if let Some(pos) = bytes.iter().position(|p| *p == b'\r') {
            match bytes.get(pos + 1) {
                Some(b'\n') => {
                    if pos == 0 {
                        return None;
                    }
                    let header_line = &bytes[0..pos];
                    if let Some(pos) = header_line.iter().position(|p| *p == b':') {
                        if let Ok(header_name) =
                            from_utf8(&header_line[0..pos]).map(|it| it.trim().to_lowercase())
                        {
                            match header_name.as_str() {
                                "if-none-match" => {
                                    return from_utf8(&header_line[pos + 2..])
                                        .ok()
                                        .map(|it| it.trim())
                                }
                                _ => {}
                            }
                        }
                    }
                    bytes = &bytes[pos + 1..];
                }
                _ => return None,
            }
        } else {
            return None;
        }
    }
}
