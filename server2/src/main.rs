mod acme;
mod cache;
mod domains;
mod jose;
mod log;
mod resolver;
mod response;
mod tls;
use crate::acme::Account;
use crate::domains::{APEX, LOCALHOST, WWW};
use crate::log::LogLevel;
use crate::tls::{config, ALPN_ACME_TLS};
use colored::Colorize;
use domains::CDN;
use response::{CDN_RESPONSE, OK_RESPONSE};
use rustls::server::Acceptor;
use std::io::Result;
use std::net::Ipv6Addr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::LazyConfigAcceptor;

const PORT: u16 = 443;

#[tokio::main]
async fn main() -> Result<()> {
    LogLevel::Error.log(|| colored::control::set_override(true));

    let acme_account = Account::init().await?;
    tokio::spawn(async move { acme_account.auto_renew().await });

    let config = config()?;

    LogLevel::Info.log(|| println!("{}", "Starting HTTP1.1 server."));
    let listener = TcpListener::bind((Ipv6Addr::UNSPECIFIED, PORT)).await?;

    println!("{}", "Listening on:".green());
    println!("https://{}", APEX.blue().underline());
    println!("https://{}", WWW.blue().underline());
    println!("https://{}", CDN.blue().underline());
    println!("https://{}", LOCALHOST.blue().underline());
    loop {
        match listener.accept().await {
            Ok((tcp, remote_addr)) => {
                LogLevel::Info.log(|| {
                    println!(
                        "Accepted TCP connection from {}.",
                        format!("{}", remote_addr.ip()).purple()
                    );
                });
                let acceptor = LazyConfigAcceptor::new(Acceptor::default(), tcp);
                let config = config.clone();
                let future = async move {
                    match acceptor.await {
                        Ok(start_handshake) => {
                            LogLevel::Info.log(|| {
                                println!(
                                    "Starting TLS handshake with {}.",
                                    format!("{}", remote_addr.ip()).purple()
                                );
                            });
                            let client_hello = start_handshake.client_hello();
                            match client_hello.server_name() {
                                Some(sni) => {
                                    LogLevel::Info.log(|| {
                                        println!(
                                            "{}",
                                            format!("TLS SNI extension: {}.", sni.purple())
                                        );
                                    });
                                    let server_name = sni.clone().to_string();
                                    if client_hello
                                        .alpn()
                                        .and_then(|mut it| it.find(|&it| it == ALPN_ACME_TLS))
                                        .is_some()
                                    {
                                        LogLevel::Info.log(|| {
                                            println!(
                                                "{}",
                                                format!(
                                                    "Responding to ACME Challenge for {}.",
                                                    server_name.purple()
                                                )
                                            );
                                        });
                                        LogLevel::Info.log(|| {
                                            println!(
                                                "{}",
                                                format!(
                                                    "Responding to ACME Challenge for {}.",
                                                    server_name.purple()
                                                )
                                            );
                                        });
                                        match start_handshake.into_stream(config).await {
                                            Ok(mut stream) => {
                                                let _ = stream.write_all(OK_RESPONSE).await;
                                                let _ = stream.shutdown().await;
                                            }
                                            Err(err) => {
                                                LogLevel::Warning.log(|| {
                                                    println!("{}", "TLS handshake failed.".red());
                                                    println!("{:?}", err);
                                                });
                                            }
                                        }
                                    } else {
                                        match start_handshake.into_stream(config).await {
                                            Ok(mut stream) => {
                                                match server_name.as_str() {
                                                    CDN => {
                                                        let _ =
                                                            stream.write_all(CDN_RESPONSE).await;
                                                    }
                                                    _ => {
                                                        let mut buf = [0; 16];
                                                        if stream.read_exact(&mut buf).await.is_ok()
                                                        {
                                                        }
                                                        let _ = stream.write_all(OK_RESPONSE).await;
                                                    }
                                                }

                                                let _ = stream.shutdown().await;
                                            }
                                            Err(err) => {
                                                LogLevel::Warning.log(|| {
                                                    println!("{}", "TLS handshake failed.".red());
                                                    println!("{:?}", err);
                                                });
                                            }
                                        }
                                    }
                                }
                                None => {
                                    LogLevel::Info.log(|| {
                                        println!("{}", "TLS SNI extension is missing.".red())
                                    });
                                }
                            }
                        }
                        Err(err) => LogLevel::Warning.log(|| {
                            println!("{}", "Failed to accept TLS connection.".red());
                            println!("{:?}", err);
                        }),
                    }
                };
                tokio::spawn(async move {
                    let _ = future.await;
                });
            }
            Err(err) => LogLevel::Warning.log(|| {
                println!("{}", "Failed to accept TCP connection.".red());
                println!("{:?}", err);
            }),
        }
    }
}
