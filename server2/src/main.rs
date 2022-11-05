mod acme;
mod cache;
mod domains;
mod handlers;
mod jose;
mod log;
mod resolver;
mod response;
mod tls;
use crate::response::OK_RESPONSE;
use acme::Account;
use colored::Colorize;
use domains::{APEX, CDN, LOCALHOST, LOCALHOST_IPV4, LOCALHOST_IPV6, WWW};
use handlers::{handle_apex_request, handle_cdn_request, handle_localhost_request};
use log::LogLevel;
use rustls::server::Acceptor;
use std::io::Result;
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::Duration;
use tls::{config, ALPN_ACME_TLS};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::time::interval;
use tokio_rustls::LazyConfigAcceptor;

const PORT: u16 = 443;

#[tokio::main]
async fn main() -> Result<()> {
    LogLevel::init();
    let acme_account = Account::init().await?;
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(86_400)); // 1 day
        loop {
            match acme_account.auto_renew().await {
                Ok(_) => {
                    LogLevel::Info
                        .log(|| println!("{}", "Successfully renewed certificate.".green()));
                }
                Err(err) => LogLevel::Warning.log(|| {
                    println!("{}", "Failed to renew certificate.".red());
                    println!("{:?}", err);
                }),
            }
            interval.tick().await;
        }
    });

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
                            LogLevel::Debug.log(|| {
                                println!(
                                    "Starting TLS handshake with {}.",
                                    format!("{}", remote_addr.ip()).purple()
                                );
                            });
                            let client_hello = start_handshake.client_hello();
                            match client_hello.server_name() {
                                Some(sni) => {
                                    LogLevel::Debug.log(|| {
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
                                        let mut acme_config = config.as_ref().clone();
                                        acme_config.alpn_protocols = vec![ALPN_ACME_TLS.to_vec()];
                                        match start_handshake
                                            .into_stream(Arc::new(acme_config))
                                            .await
                                        {
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
                                                    LOCALHOST | LOCALHOST_IPV4 | LOCALHOST_IPV6 => {
                                                        handle_localhost_request(&mut stream).await;
                                                    }
                                                    CDN => {
                                                        handle_cdn_request(&mut stream).await;
                                                    }
                                                    APEX | WWW => {
                                                        handle_apex_request(&mut stream).await;
                                                    }
                                                    _ => {}
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
                                    LogLevel::Debug.log(|| {
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
