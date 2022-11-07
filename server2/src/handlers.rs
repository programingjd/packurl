use crate::domains::CDN;
use crate::log::LogLevel;
use crate::response::{
    BAD_REQUEST_RESPONSE, CDN_RESPONSE, CONTENT_TOO_LARGE_RESPONSE, FAVICON_RESPONSE,
    MANIFEST_RESPONSE, METHOD_NOT_ALLOWED_RESPONSE, NOT_FOUND_RESPONSE, OK_RESPONSE,
    PAYLOAD_TOO_LARGE_RESPONSE, ROOT_REDIRECT_RESPONSE, SERVICE_WORKER_RESPONSE,
};
use async_recursion::async_recursion;
use colored::Colorize;
use dashmap::DashMap;
use lazy_static::lazy_static;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::str::from_utf8;
use std::time::UNIX_EPOCH;
use tokio::fs::{metadata, read, read_dir};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

const ROOT: &'static str = "/home/admin/www";
const PREFIX: &'static str = "/";

struct FileEntry {
    etag: Option<String>,
    ok: Vec<u8>,
    not_modified: Vec<u8>,
}

lazy_static! {
    static ref FILES: DashMap<String, FileEntry> = DashMap::with_capacity(1024);
    static ref CDN_ROOT: String = format!("https://{}", CDN);
}

pub async fn handle_localhost_request(stream: &mut TlsStream<TcpStream>) {
    let mut buf = [0; 22];
    if stream.read_exact(&mut buf).await.is_ok() {
        match &buf {
            b"GET /update HTTP/1.1\r\n" => {
                if let Err(err) = update_files().await {
                    let trace = format!("{:?}", err);
                    let body = trace.as_bytes();
                    let text = format!(
                        "HTTP/1.1 500 Internal Server Error\r\n\
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

pub async fn handle_cdn_request(stream: &mut TlsStream<TcpStream>) {
    if let Err(err) = handle_file_request(stream).await {
        LogLevel::Warning.log(|| {
            println!("{}", "Failed to accept TLS connection.".red());
            println!("{:?}", err);
        });
        let _ = stream.write_all(CDN_RESPONSE).await;
    }
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

async fn update_files() -> Result<()> {
    let path = Path::new(ROOT);
    walk(path).await?;
    Ok(())
}

async fn build_response(path: &Path, cache_control: &str, content_type: &str) -> Result<FileEntry> {
    if let Some(filename) = path.file_name().and_then(|it| it.to_str()) {
        let meta = metadata(path).await?;
        let content_length = meta.len();
        let last_modified = meta
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(|err| Error::new(ErrorKind::InvalidData, err))?
            .as_secs();
        let etag = format!("{:#x}{:#x}", content_length, last_modified);
        let (data, compressed) = if let Some(compressed_path) =
            path.parent().map(|it| it.join(format!("{}.br", filename)))
        {
            match read(compressed_path).await {
                Ok(data) => (data, "Content-Encoding: br\r\n"),
                Err(_) => (read(path).await?, ""),
            }
        } else {
            (read(path).await?, "")
        };
        Ok(FileEntry {
            ok: [
                format!(
                    "HTTP/1.1 200 OK\r\n\
Cache-Control: {}\r\n\
Connection: close\r\n\
{}\
ETag: {}\r\n\
Content-Type: {}\r\n\
Content-Length: {}\r\n\
\r\n",
                    compressed, cache_control, etag, content_type, content_length
                )
                .into_bytes(),
                data,
            ]
            .concat(),
            not_modified: format!(
                "HTTP/1.1 304 Not Modified\r\n\
Cache-Control: {}\r\n\
Connection: close\r\n\
ETag: {}\r\n\
\r\n",
                cache_control, etag
            )
            .into_bytes(),
            etag: Some(etag),
        })
    } else {
        Err(Error::new(
            ErrorKind::Other,
            "Could not extract filename from path",
        ))
    }
}

#[async_recursion]
async fn walk(path: &Path) -> Result<()> {
    let stat = metadata(path).await?;
    if stat.is_dir() {
        let mut iterator = read_dir(path).await?;
        while let Some(entry) = iterator.next_entry().await? {
            let path = path.join(entry.file_name());
            walk(path.as_path()).await?;
        }
    }
    if stat.is_file() {
        if let Ok(relative_path) = path.strip_prefix(ROOT) {
            let uri_path = Path::new(PREFIX).join(relative_path);
            if let Some(parent) = uri_path
                .parent()
                .and_then(|it| it.to_str())
                .map(|it| it.to_lowercase())
            {
                if let Some(filename) = uri_path
                    .file_name()
                    .and_then(|it| it.to_str().map(|it| it.to_lowercase()))
                {
                    if filename == "index.html" {
                        let _ = FILES.insert(
                            parent,
                            build_response(path, "public,no-cache", "text/html").await?,
                        );
                    } else if filename.ends_with(".html") || filename.ends_with(".htm") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,no-cache", "text/html").await?,
                        );
                    } else if filename.ends_with(".css") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,no-cache", "text/css").await?,
                        );
                    } else if filename.ends_with(".js")
                        || filename.ends_with(".mjs")
                        || filename.ends_with(".js.map")
                        || filename.ends_with(".mjs.map")
                    {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,no-cache", "application/javascript")
                                .await?,
                        );
                    } else if filename.ends_with(".svg") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(
                                path,
                                "public,max-age=31536000,immutable",
                                "image/svg+xml",
                            )
                            .await?,
                        );
                    } else if filename.ends_with(".jpg") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,max-age=31536000,immutable", "image/jpeg")
                                .await?,
                        );
                    } else if filename.ends_with(".png") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,max-age=31536000,immutable", "image/png")
                                .await?,
                        );
                    } else if filename.ends_with(".webp") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,max-age=31536000,immutable", "image/webp")
                                .await?,
                        );
                    } else if filename.ends_with(".woff2") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,max-age=31536000,immutable", "font/woff2")
                                .await?,
                        );
                    } else if filename.ends_with(".json") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(
                                path,
                                "public,max-age=3600,must-revalidate",
                                "application/json",
                            )
                            .await?,
                        );
                    } else if filename.ends_with(".wasm") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(
                                path,
                                "public,max-age=31536000,immutable",
                                "application/wasm",
                            )
                            .await?,
                        );
                    } else if filename.ends_with(".glb") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(
                                path,
                                "public,max-age=31536000,immutable",
                                "model/gltf-binary",
                            )
                            .await?,
                        );
                    } else if filename.ends_with(".md") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,no-cache", "text/markdown").await?,
                        );
                    } else if filename.ends_with(".xml") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(
                                path,
                                "public,max-age=3600,must-revalidate",
                                "application/xml",
                            )
                            .await?,
                        );
                    } else if filename.ends_with(".txt")
                        || filename.ends_with(".glsl")
                        || filename.ends_with(".wat")
                    {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,no-cache", "text/plain").await?,
                        );
                    } else if filename.ends_with(".mp3") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,max-age=31536000,immutable", "audio/mp3")
                                .await?,
                        );
                    } else if filename.ends_with(".mp4") {
                        let _ = FILES.insert(
                            format!("{}/{}", parent, filename),
                            build_response(path, "public,max-age=31536000,immutable", "audio/mp4")
                                .await?,
                        );
                    } else if filename.ends_with(".sig") {
                        let _ = FILES.insert(
                            parent,
                            build_response(path, "no-store", "application/pgp-signature").await?,
                        );
                    }
                }
            }
        }
    }
    Ok(())
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

async fn handle_file_request(stream: &mut TlsStream<TcpStream>) -> Result<()> {
    let max: u64 = 4096;
    let mut bounded = stream.take(max);
    let mut bytes = Vec::new();
    if max as usize == bounded.read_to_end(&mut bytes).await? {
        let _ = stream.write_all(PAYLOAD_TOO_LARGE_RESPONSE).await;
        Ok(())
    } else {
        let bytes = bytes.as_slice();
        if let Some(pos) = bytes.iter().position(|p| *p == b'\r') {
            match bytes.get(pos + 1) {
                Some(b'\n') => match &bytes[0..4] {
                    b"GET " => {
                        if let Ok(path) = from_utf8(&bytes[4..pos]) {
                            let path = path.replace(CDN_ROOT.as_str(), "");
                            match FILES.get(path.as_str()) {
                                None => {
                                    let _ = stream.write_all(NOT_FOUND_RESPONSE).await;
                                    return Ok(());
                                }
                                Some(entry) => {
                                    let not_modified = if let Some(if_none_match) =
                                        get_if_none_match(&bytes[pos + 2..])
                                    {
                                        if let Some(etag) = &entry.etag {
                                            etag.as_str() == if_none_match
                                        } else {
                                            false
                                        }
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
