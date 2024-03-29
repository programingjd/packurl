mod acme;
mod apex;
mod cdn;
mod domains;
mod local;
mod log;
mod tls;
use crate::acme::handle_acme_request;
use crate::apex::{handle_apex_request, handle_redirect_to_https};
use crate::cdn::{handle_cdn_request, Cache};
use crate::local::handle_local_request;
use acme::Account;
use colored::Colorize;
use domains::{APEX, CDN, LOCALHOST, LOCALHOST_IPV4, LOCALHOST_IPV6, WWW};
use log::LogLevel;
use rustls::server::Acceptor;
use std::io::Result;
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::Duration;
use tls::{config, ALPN_ACME_TLS};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio_rustls::LazyConfigAcceptor;

const FIVE_DAYS: u64 = 86_400 * 5;

#[tokio::main]
async fn main() -> Result<()> {
    LogLevel::init();
    Cache::init();
    Account::init()
        .await?
        .auto_renew_certificate_every(Duration::from_secs(FIVE_DAYS));

    let tls_config = config()?;

    LogLevel::Info.log(|| println!("{}", "Starting HTTP1.1 server"));
    let https_listener = TcpListener::bind((Ipv6Addr::UNSPECIFIED, 443)).await?;
    let http_listener = TcpListener::bind((Ipv6Addr::UNSPECIFIED, 80)).await?;

    println!("{}", "Listening on:".green());
    println!("{}", format!("http://{}", APEX).cyan().underline());
    println!("{}", format!("http://{}", WWW).cyan().underline());
    tokio::spawn(async move {
        loop {
            match http_listener.accept().await {
                Ok((mut stream, remote_addr)) => {
                    LogLevel::Info.log(|| {
                        println!(
                            "Accepted TCP connection from {} on port 80",
                            format!("{}", remote_addr.ip()).purple()
                        );
                    });
                    handle_redirect_to_https(&mut stream).await;
                    let _ = stream.shutdown().await;
                }
                Err(err) => LogLevel::Warning.log(|| {
                    println!("{}", "Failed to accept TCP connection on port 80".red());
                    println!("{:?}", err);
                }),
            }
        }
    });

    println!("{}", "Listening on:".green());
    println!("{}", format!("https://{}", APEX).cyan().underline());
    println!("{}", format!("https://{}", WWW).cyan().underline());
    println!("{}", format!("https://{}", CDN).cyan().underline());
    println!("{}", format!("https://{}", LOCALHOST).cyan().underline());

    loop {
        match https_listener.accept().await {
            Ok((tcp, remote_addr)) => {
                LogLevel::Info.log(|| {
                    println!(
                        "Accepted TCP connection from {} on port 443",
                        format!("{}", remote_addr.ip()).purple()
                    );
                });
                LogLevel::Debug.log(|| {
                    println!(
                        "Starting TLS handshake with {}",
                        format!("{}", remote_addr.ip()).purple()
                    );
                });
                let acceptor = LazyConfigAcceptor::new(Acceptor::default(), tcp);
                let config = tls_config.clone();
                let future = async move {
                    match acceptor.await {
                        Ok(start_handshake) => {
                            let client_hello = start_handshake.client_hello();
                            match client_hello.server_name() {
                                Some(sni) => {
                                    LogLevel::Debug.log(|| {
                                        println!(
                                            "{}",
                                            format!("TLS SNI extension: {}", sni.purple())
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
                                                    "Responding to ACME Challenge for {}",
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
                                                handle_acme_request(&mut stream).await;
                                                let _ = stream.shutdown().await;
                                            }
                                            Err(err) => {
                                                LogLevel::Warning.log(|| {
                                                    println!("{}", "TLS handshake failed".red());
                                                    println!("{:?}", err);
                                                });
                                            }
                                        }
                                    } else {
                                        match start_handshake.into_stream(config).await {
                                            Ok(mut stream) => {
                                                match server_name.as_str() {
                                                    LOCALHOST | LOCALHOST_IPV4 | LOCALHOST_IPV6 => {
                                                        handle_local_request(&mut stream).await;
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
                                                    println!("{}", "TLS handshake failed".red());
                                                    println!("{:?}", err);
                                                });
                                            }
                                        }
                                    }
                                }
                                None => {
                                    LogLevel::Debug.log(|| {
                                        println!("{}", "TLS SNI extension is missing".red())
                                    });
                                }
                            }
                        }
                        Err(err) => LogLevel::Warning.log(|| {
                            println!("{}", "Failed to accept TLS connection".red());
                            println!("{:?}", err);
                        }),
                    }
                };
                tokio::spawn(async move {
                    let _ = future.await;
                });
            }
            Err(err) => LogLevel::Warning.log(|| {
                println!("{}", "Failed to accept TCP connection on port 443".red());
                println!("{:?}", err);
            }),
        }
    }
}
