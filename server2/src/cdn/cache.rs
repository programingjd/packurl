use crate::domains::CDN;
use crate::log::LogLevel;
use async_recursion::async_recursion;
use colored::Colorize;
use dashmap::DashMap;
use lazy_static::lazy_static;
use normalize_path::NormalizePath;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::fs::{metadata, read, read_dir};
use tokio::sync::Mutex;

const ROOT: &'static str = "/home/admin/www";
const PREFIX: &'static str = "/";

lazy_static! {
    pub static ref FILES: DashMap<String, FileEntry> = DashMap::with_capacity(1024);
    pub static ref CDN_ROOT: String = format!("https://{}", CDN);
    static ref LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

pub struct Cache {}

impl Cache {
    pub fn init() {
        tokio::spawn(async move {
            match Self::update().await {
                Ok(_) => {
                    LogLevel::Info
                        .log(|| println!("{}", "Successfully initialized file cache.".green()));
                }
                Err(err) => LogLevel::Warning.log(|| {
                    println!("{}", "Failed to initialize file cache.".red());
                    println!("{:?}", err);
                }),
            }
        });
    }

    pub async fn update() -> Result<()> {
        let lock = match LOCK.try_lock() {
            Ok(lock) => lock,
            Err(_) => {
                LogLevel::Info
                    .log(|| println!("{}", "Waiting for previous update to finish.".yellow()));
                LOCK.lock().await
            }
        };
        LogLevel::Info.log(|| println!("{}", "Updating file cache.".purple()));
        let path = Path::new(ROOT);
        for entry in FILES.iter() {
            match metadata(path.join(entry.key())).await {
                Ok(stat) => {
                    if stat.is_dir() {
                        match metadata(path.join("index.html")).await {
                            Ok(stat) => {
                                if !stat.is_file() {
                                    LogLevel::Debug.log(|| {
                                        println!("{}", format!("Removing {}.", entry.key().red()))
                                    });
                                    FILES.remove(entry.key());
                                }
                            }
                            Err(_) => {
                                LogLevel::Debug.log(|| {
                                    println!("{}", format!("Removing {}.", entry.key().red()))
                                });
                                FILES.remove(entry.key());
                            }
                        }
                    } else if !stat.is_file() {
                        LogLevel::Debug
                            .log(|| println!("{}", format!("Removing {}.", entry.key().red())));
                        FILES.remove(entry.key());
                    }
                }
                Err(_) => {
                    LogLevel::Debug
                        .log(|| println!("{}", format!("Removing {}.", entry.key().red())));
                    FILES.remove(entry.key());
                }
            }
        }
        walk(path).await?;
        drop(lock);
        Ok(())
    }
}

pub struct FileEntry {
    pub etag: String,
    pub ok: Vec<u8>,
    pub not_modified: Vec<u8>,
}

async fn etag_and_size(path: &Path) -> Result<(String, u64)> {
    let meta = metadata(path).await?;
    let content_length = meta.len();
    let last_modified = meta
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::new(ErrorKind::InvalidData, err))?
        .as_secs();
    Ok((
        format!("{:#x}{:#x}", content_length, last_modified),
        meta.len(),
    ))
}

fn cache_control_and_content_type(filename: &String) -> Option<(&str, &str)> {
    if filename.ends_with(".html") || filename.ends_with(".htm") {
        Some(("public,no-cache", "text/html"))
    } else if filename.ends_with(".css") {
        Some(("public,no-cache", "text/css"))
    } else if filename.ends_with(".js")
        || filename.ends_with(".mjs")
        || filename.ends_with(".js.map")
        || filename.ends_with(".mjs.map")
    {
        Some(("public,no-cache", "application/javascript"))
    } else if filename.ends_with(".svg") {
        Some(("public,max-age=31536000,immutable", "image/svg+xml"))
    } else if filename.ends_with(".jpg") {
        Some(("public,max-age=31536000,immutable", "image/jpeg"))
    } else if filename.ends_with(".png") {
        Some(("public,max-age=31536000,immutable", "image/png"))
    } else if filename.ends_with(".webp") {
        Some(("public,max-age=31536000,immutable", "image/webp"))
    } else if filename.ends_with(".woff2") {
        Some(("public,max-age=31536000,immutable", "font/woff2"))
    } else if filename.ends_with(".json") {
        Some(("public,max-age=3600,must-revalidate", "application/json"))
    } else if filename.ends_with(".wasm") {
        Some(("public,max-age=31536000,immutable", "application/wasm"))
    } else if filename.ends_with(".glb") {
        Some(("public,max-age=31536000,immutable", "model/gltf-binary"))
    } else if filename.ends_with(".md") {
        Some(("public,no-cache", "text/markdown"))
    } else if filename.ends_with(".xml") {
        Some(("public,max-age=3600,must-revalidate", "application/xml"))
    } else if filename.ends_with(".txt")
        || filename.ends_with(".glsl")
        || filename.ends_with(".wat")
    {
        Some(("public,no-cache", "text/plain"))
    } else if filename.ends_with(".mp3") {
        Some(("public,max-age=31536000,immutable", "audio/mp3"))
    } else if filename.ends_with(".mp4") {
        Some(("public,max-age=31536000,immutable", "audio/mp4"))
    } else if filename.ends_with(".sig") {
        Some(("no-store", "application/pgp-signature"))
    } else {
        None
    }
}

async fn build_response(
    path: &Path,
    etag: String,
    content_length: u64,
    cache_control: &str,
    content_type: &str,
) -> Result<FileEntry> {
    if let Some(filename) = path.file_name().and_then(|it| it.to_str()) {
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
                    "\
HTTP/1.1 200 OK\r\n\
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
                "\
HTTP/1.1 304 Not Modified\r\n\
Cache-Control: {}\r\n\
Connection: close\r\n\
ETag: {}\r\n\
\r\n",
                cache_control, etag
            )
            .into_bytes(),
            etag,
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
            let uri_path = Path::new(PREFIX).join(relative_path).normalize();
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
                        if let Ok((etag, content_length)) = etag_and_size(path).await {
                            let stored = FILES.get(&parent);
                            let update = stored.is_some();
                            let insert = if let Some(stored) = stored {
                                stored.etag == etag
                            } else {
                                true
                            };
                            if insert {
                                if update {
                                    LogLevel::Debug.log(|| {
                                        println!("{}", format!("Updating {}.", parent.yellow()))
                                    });
                                } else {
                                    LogLevel::Debug.log(|| {
                                        println!("{}", format!("Adding   {}.", parent.green()))
                                    });
                                }
                                let _ = FILES.insert(
                                    parent,
                                    build_response(
                                        path,
                                        etag,
                                        content_length,
                                        "public,no-cache",
                                        "text/html",
                                    )
                                    .await?,
                                );
                            }
                        }
                    } else if let Some((cache_control, content_type)) =
                        cache_control_and_content_type(&filename)
                    {
                        if let Ok((etag, content_length)) = etag_and_size(path).await {
                            let key = format!("{}/{}", parent, filename);
                            let stored = FILES.get(&key);
                            let update = stored.is_some();
                            let insert = if let Some(stored) = stored {
                                stored.etag == etag
                            } else {
                                true
                            };
                            if insert {
                                if update {
                                    LogLevel::Debug.log(|| {
                                        println!("{}", format!("Updating {}.", key.yellow()))
                                    });
                                } else {
                                    LogLevel::Debug.log(|| {
                                        println!("{}", format!("Adding   {}.", key.green()))
                                    });
                                }
                                let _ = FILES.insert(
                                    key,
                                    build_response(
                                        path,
                                        etag,
                                        content_length,
                                        cache_control,
                                        content_type,
                                    )
                                    .await?,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
