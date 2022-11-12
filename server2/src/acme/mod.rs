use crate::acme::cache::{
    backup_account_keys, backup_account_kid, restore_account_keys, restore_account_kid,
    set_certificate, set_challenge_key,
};
use crate::acme::jose::{authorization_hash, jose};
use crate::domains::ACME_DOMAINS;
use crate::log::LOG_LEVEL;
use crate::LogLevel;
use base64::URL_SAFE_NO_PAD;
pub use cache::{get_certificate, get_challenge_key};
use colored::Colorize;
pub use handler::handle_acme_request;
use lazy_static::lazy_static;
use rcgen::{
    Certificate, CertificateParams, CustomExtension, DistinguishedName, PKCS_ECDSA_P256_SHA256,
};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Response};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use rustls::sign::{any_ecdsa_type, CertifiedKey};
use rustls::PrivateKey;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env::var;
use std::io::{Error, ErrorKind, Result};
use tokio::time::{interval, sleep, Duration};

mod cache;
mod handler;
mod jose;

lazy_static! {
    pub static ref DIRECTORY_URL: String = var("XDG_ACME_DIRECTORY")
        .unwrap_or("https://acme-staging-v02.api.letsencrypt.org/directory".to_string());
    pub static ref CONTACT: String =
        var("XDG_ACME_CONTACT").unwrap_or("mailto:admin@packurl.net".to_string());
}

pub struct Account {
    keypair: EcdsaKeyPair,
    kid: String,
}

impl Account {
    pub async fn init() -> Result<Self> {
        LogLevel::Info.log(|| {
            println!(
                "Using ACME directory {} with account {}",
                DIRECTORY_URL.yellow(),
                CONTACT.yellow()
            )
        });
        let keypair = match restore_account_keys().await {
            Some(bytes) => {
                LogLevel::Info.log(|| println!("{}", "Restoring ACME account keys"));
                EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, bytes.as_slice())
                    .map_err(|_| Error::new(ErrorKind::Other, "Failed to parse account keys"))?
            }
            None => {
                LogLevel::Info.log(|| println!("{}", "Creating ACME account keys"));
                let rng = SystemRandom::new();
                let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                    .map_err(|_| Error::new(ErrorKind::Other, "Failed to create account keys"))?;
                let bytes = pkcs8.as_ref();
                backup_account_keys(bytes).await?;
                EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, bytes)
                    .map_err(|_| Error::new(ErrorKind::Other, "Failed to parse account keys"))?
            }
        };
        let kid = match restore_account_kid().await {
            Some(bytes) => {
                LogLevel::Info.log(|| println!("{}", "Restoring ACME account kid"));
                String::from_utf8(bytes.clone()).map_err(|err| Error::new(ErrorKind::Other, err))?
            }
            None => {
                LogLevel::Info.log(|| println!("{}", "Registering ACME account"));
                let kid = Self::new_account(&keypair).await?;
                backup_account_kid(kid.as_bytes()).await?;
                kid
            }
        };
        Ok(Account { keypair, kid })
    }

    pub fn auto_renew_certificate_every(self, duration: Duration) {
        tokio::spawn(async move {
            let mut interval = interval(duration);
            loop {
                match self.auto_renew().await {
                    Ok(_) => {
                        LogLevel::Info
                            .log(|| println!("{}", "Successfully renewed certificate".green()));
                    }
                    Err(err) => LogLevel::Warning.log(|| {
                        println!("{}", "Failed to renew certificate".red());
                        println!("{:?}", err);
                    }),
                }
                interval.tick().await;
            }
        });
    }

    async fn auto_renew(&self) -> Result<()> {
        LogLevel::Info.log(|| println!("{}", "Creating new ACME order"));
        let client = Client::new();
        LogLevel::Debug.log(|| println!("{}", "Getting ACME directory"));
        let directory = client
            .get(DIRECTORY_URL.as_str())
            .send()
            .await
            .map_err(|err| Error::new(ErrorKind::Other, err))?
            .json::<Directory>()
            .await
            .map_err(|err| Error::new(ErrorKind::Other, err))?;
        match self
            .new_order(&client, &directory, ACME_DOMAINS.into())
            .await?
        {
            Order::Invalid => return Err(Error::new(ErrorKind::Other, "Order is invalid")),
            Order::Pending {
                authorizations,
                finalize,
            } => {
                LogLevel::Info.log(|| {
                    println!(
                        "{}",
                        format!("Order needs {} authorizations", authorizations.len())
                    );
                });
                for url in authorizations {
                    LogLevel::Info.log(|| {
                        println!("{}", format!("Authorizing {}", url));
                    });
                    self.authorize(&client, &directory, &url).await?;
                }
                LogLevel::Info.log(|| println!("{}", "Finalizing order"));
                self.finalize(&client, &directory, &finalize).await?;
            }
            Order::Ready { finalize } => {
                LogLevel::Info.log(|| println!("Order already authorized"));
                LogLevel::Info.log(|| println!("{}", "Finalizing order"));
                self.finalize(&client, &directory, &finalize).await?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    async fn new_account(keypair: &EcdsaKeyPair) -> Result<String> {
        let client = Client::new();
        LogLevel::Debug.log(|| println!("{}", "Getting ACME directory"));
        let directory = client
            .get(DIRECTORY_URL.as_str())
            .send()
            .await
            .map_err(|err| Error::new(ErrorKind::Other, err))?
            .json::<Directory>()
            .await
            .map_err(|err| Error::new(ErrorKind::Other, err))?;
        LogLevel::Debug.log(|| println!("{}", "Requesting new nonce"));
        let nonce = Self::new_nonce(&client, &directory).await?;
        LogLevel::Debug.log(|| println!("{}", "Calling new account directory endpoint"));
        let contact = vec![CONTACT.as_str()];
        let payload = json!({
            "termsOfServiceAgreed": true,
            "contact": contact
        });
        let body = jose(keypair, Some(payload), None, &nonce, &directory.new_account)?;
        let response = Self::jose_request(&client, &directory.new_account, &body).await?;
        if response.status().is_success() {
            let kid = response
                .headers()
                .get("location")
                .ok_or_else(|| Error::new(ErrorKind::Other, "Missing \"location\" header"))?
                .to_str()
                .map_err(|err| Error::new(ErrorKind::Other, err))?;
            Ok(kid.to_string())
        } else {
            if LOG_LEVEL > LogLevel::Error {
                Self::print_request_error(response).await;
            }
            Err(Error::new(ErrorKind::Other, "Failed to create new account"))
        }
    }

    async fn new_order(
        &self,
        client: &Client,
        directory: &Directory,
        domains: Vec<String>,
    ) -> Result<Order> {
        LogLevel::Debug.log(|| println!("{}", "Requesting new nonce"));
        let nonce = Self::new_nonce(client, directory).await?;
        let identifiers: Vec<Identifier> = domains.into_iter().map(Identifier::Dns).collect();
        let payload = json!({ "identifiers": identifiers });
        let body = jose(
            &self.keypair,
            Some(payload),
            Some(&self.kid),
            &nonce,
            &directory.new_order,
        )?;
        LogLevel::Debug.log(|| println!("{}", "Calling new order directory endpoint"));
        let response = Self::jose_request(client, &directory.new_order, &body).await?;
        if response.status().is_success() {
            let order = response
                .json()
                .await
                .map_err(|err| Error::new(ErrorKind::Other, err))?;
            Ok(order)
        } else {
            if LOG_LEVEL > LogLevel::Error {
                Self::print_request_error(response).await;
            }
            Err(Error::new(ErrorKind::Other, "Failed to create new order"))
        }
    }

    async fn authorize(&self, client: &Client, directory: &Directory, url: &str) -> Result<()> {
        LogLevel::Debug.log(|| println!("{}", "Requesting new nonce"));
        let nonce = Self::new_nonce(client, directory).await?;
        let body = jose(&self.keypair, None, Some(&self.kid), &nonce, url)?;
        LogLevel::Info.log(|| println!("{}", "Calling authorization endpoint"));
        let response = Self::jose_request(client, url, &body).await?;
        if response.status().is_success() {
            match response
                .json()
                .await
                .map_err(|err| Error::new(ErrorKind::Other, err))?
            {
                Auth::Pending {
                    challenges,
                    identifier,
                } => {
                    let Identifier::Dns(domain) = identifier;
                    LogLevel::Info.log(|| {
                        println!(
                            "{}",
                            format!("Selecting TlsAlpn01 challenge for {}", &domain.purple())
                        );
                    });
                    let challenge = challenges
                        .iter()
                        .find(|it| it.typ == ChallengeType::TlsAlpn01)
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Unsupported,
                                "TlsAlpn01 challenge is not available",
                            )
                        })?;
                    let auth = authorization_hash(&self.keypair, &challenge.token)?;
                    // let mut params = CertificateParams::new(ACME_DOMAINS);
                    let mut params = CertificateParams::new(vec![domain.clone()]);
                    params.alg = &PKCS_ECDSA_P256_SHA256;
                    params.custom_extensions =
                        vec![CustomExtension::new_acme_identifier(auth.as_slice())];
                    let certificate = Certificate::from_params(params)
                        .map_err(|err| Error::new(ErrorKind::Unsupported, err))?;
                    let key = CertifiedKey::new(
                        vec![rustls::Certificate(
                            certificate
                                .serialize_der()
                                .map_err(|err| Error::new(ErrorKind::Unsupported, err))?,
                        )],
                        any_ecdsa_type(&PrivateKey(certificate.serialize_private_key_der()))
                            .map_err(|err| Error::new(ErrorKind::Unsupported, err))?,
                    );
                    LogLevel::Info
                        .log(|| println!("Storing unsigned certificate for {}", &domain.purple()));
                    set_challenge_key(&domain, key)?;
                    LogLevel::Debug.log(|| println!("{}", "Requesting new nonce"));
                    let nonce = Self::new_nonce(client, directory).await?;
                    let payload = json!({});
                    let body = jose(
                        &self.keypair,
                        Some(payload),
                        Some(&self.kid),
                        &nonce,
                        &challenge.url,
                    )?;
                    let response = Self::jose_request(client, &challenge.url, &body).await?;
                    LogLevel::Info.log(|| println!("{}", "Calling challenge trigger endpoint"));
                    if response.status().is_success() {
                        LogLevel::Debug.log(|| {
                            println!("{}", "Waiting 5 seconds before checking status again")
                        });
                        sleep(Duration::from_millis(5_000)).await;
                        LogLevel::Info.log(|| {
                            println!("{}", format!("Checking status again for url {}", url))
                        });
                        LogLevel::Debug.log(|| println!("{}", "Requesting new nonce"));
                        let nonce = Self::new_nonce(client, directory).await?;
                        let body = jose(&self.keypair, None, Some(&self.kid), &nonce, url)?;
                        LogLevel::Info.log(|| println!("{}", "Calling authorization endpoint"));
                        let response = Self::jose_request(client, url, &body).await?;
                        if response.status().is_success() {
                            match response
                                .json()
                                .await
                                .map_err(|err| Error::new(ErrorKind::Other, err))?
                            {
                                Auth::Pending {
                                    challenges: _,
                                    identifier: _,
                                } => {
                                    Err(Error::new(ErrorKind::Other, "Challenge is still pending"))
                                }
                                Auth::Valid => Ok(()),
                                Auth::Invalid => {
                                    Err(Error::new(ErrorKind::Other, "Challenge is invalid"))
                                }
                                Auth::Expired => {
                                    Err(Error::new(ErrorKind::Other, "Challenge has expired"))
                                }
                                Auth::Revoked => {
                                    Err(Error::new(ErrorKind::Other, "Challenge was revoked"))
                                }
                            }
                        } else {
                            if LOG_LEVEL > LogLevel::Error {
                                Self::print_request_error(response).await;
                            }
                            Err(Error::new(ErrorKind::Other, "Failed to authorize url"))
                        }
                    } else {
                        if LOG_LEVEL > LogLevel::Error {
                            Self::print_request_error(response).await;
                        }
                        Err(Error::new(ErrorKind::Other, "Failed to trigger challenge"))
                    }
                }
                Auth::Valid => Ok(()),
                Auth::Invalid | Auth::Expired | Auth::Revoked => {
                    Err(Error::new(ErrorKind::Other, "ACME auth failed"))
                }
            }
        } else {
            if LOG_LEVEL > LogLevel::Error {
                Self::print_request_error(response).await;
            }
            Err(Error::new(ErrorKind::Other, "Failed to authorize url"))
        }
    }

    async fn finalize(&self, client: &Client, directory: &Directory, url: &str) -> Result<()> {
        LogLevel::Info.log(|| println!("{}", "Creating CSR"));
        let mut params = CertificateParams::new(ACME_DOMAINS);
        params.distinguished_name = DistinguishedName::new();
        params.alg = &PKCS_ECDSA_P256_SHA256;
        let cert =
            Certificate::from_params(params).map_err(|err| Error::new(ErrorKind::Other, err))?;
        let csr = cert
            .serialize_request_der()
            .map_err(|err| Error::new(ErrorKind::Other, err))?;
        LogLevel::Debug.log(|| println!("{}", "Requesting new nonce"));
        let nonce = Self::new_nonce(client, directory).await?;
        let payload = json!({ "csr": base64::encode_config(csr, URL_SAFE_NO_PAD) });
        let body = jose(&self.keypair, Some(payload), Some(&self.kid), &nonce, url)?;
        LogLevel::Info.log(|| println!("{}", "Calling finalize endpoint"));
        let response = Self::jose_request(client, url, &body).await?;
        if response.status().is_success() {
            match response
                .json()
                .await
                .map_err(|err| Error::new(ErrorKind::Other, err))?
            {
                Order::Invalid => return Err(Error::new(ErrorKind::Other, "Order is invalid")),
                Order::Valid { certificate } => {
                    LogLevel::Info.log(|| println!("{}", "Certificate has been issued"));
                    let pem = [
                        &cert.serialize_private_key_pem(),
                        "\n",
                        &self
                            .download_certificate(client, directory, &certificate)
                            .await?,
                    ]
                    .concat()
                    .into_bytes();
                    LogLevel::Info.log(|| println!("{}", "Saving certificate"));
                    set_certificate(&pem)?;
                    get_certificate()
                        .ok_or_else(|| Error::new(ErrorKind::Other, "Saving failed"))?;
                    LogLevel::Info.log(|| println!("{}", "Certificate saved"));
                    Ok(())
                }
                _ => unreachable!(),
            }
        } else {
            if LOG_LEVEL > LogLevel::Error {
                Self::print_request_error(response).await;
            }
            Err(Error::new(ErrorKind::Other, "Failed to finalize order"))
        }
    }

    async fn download_certificate(
        &self,
        client: &Client,
        directory: &Directory,
        url: &str,
    ) -> Result<String> {
        LogLevel::Info.log(|| println!("{}", "Downloading certificate"));
        let nonce = Self::new_nonce(client, directory).await?;
        let body = jose(&self.keypair, None, Some(&self.kid), &nonce, url)?;
        LogLevel::Info.log(|| println!("{}", "Calling download endpoint."));
        let response = Self::jose_request(client, url, &body).await?;
        if response.status().is_success() {
            let order = response
                .text()
                .await
                .map_err(|err| Error::new(ErrorKind::Other, err))?;
            Ok(order)
        } else {
            if LOG_LEVEL > LogLevel::Error {
                Self::print_request_error(response).await;
            }
            Err(Error::new(
                ErrorKind::Other,
                "Failed to download certificate",
            ))
        }
    }

    async fn new_nonce(client: &Client, directory: &Directory) -> Result<String> {
        Ok(client
            .get(directory.new_nonce.as_str())
            .send()
            .await
            .map_err(|err| Error::new(ErrorKind::Other, err))?
            .headers()
            .get("replay-nonce")
            .ok_or_else(|| Error::new(ErrorKind::Other, "Missing \"replay-nonce\" header"))?
            .to_str()
            .map_err(|err| Error::new(ErrorKind::Other, err))?
            .to_string())
    }

    async fn print_request_error(response: Response) {
        let status = response.status();
        let url = response.url().to_string();
        println!(
            "{}\n{}\n{}\n",
            url.blue(),
            format!("{:?}", status).red(),
            response.text().await.unwrap_or_else(|_| String::new()),
        );
    }

    async fn jose_request(client: &Client, url: &str, body: &Value) -> Result<Response> {
        let mut headers = HeaderMap::new();
        headers.append(
            CONTENT_TYPE,
            HeaderValue::from_static("application/jose+json"),
        );
        client
            .post(url)
            .json(body)
            .headers(headers)
            .send()
            .await
            .map_err(|err| Error::new(ErrorKind::Other, err))
    }
}

#[derive(Deserialize)]
#[serde(tag = "status")]
enum Order {
    #[serde(rename = "pending")]
    Pending {
        authorizations: Vec<String>,
        finalize: String,
    },
    #[serde(rename = "ready")]
    Ready { finalize: String },
    #[serde(rename = "valid")]
    Valid { certificate: String },
    #[serde(rename = "invalid")]
    Invalid,
}

#[derive(Deserialize)]
#[serde(tag = "status")]
enum Auth {
    #[serde(rename = "pending")]
    Pending {
        identifier: Identifier,
        challenges: Vec<Challenge>,
    },
    #[serde(rename = "valid")]
    Valid,
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "revoked")]
    Revoked,
    #[serde(rename = "expired")]
    Expired,
}

#[derive(Deserialize, Eq, PartialEq)]
enum ChallengeType {
    #[serde(rename = "http-01")]
    Http01,
    #[serde(rename = "dns-01")]
    Dns01,
    #[serde(rename = "tls-alpn-01")]
    TlsAlpn01,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
enum Identifier {
    #[serde(rename = "dns")]
    Dns(String),
}

#[derive(Deserialize)]
struct Challenge {
    #[serde(rename = "type")]
    pub typ: ChallengeType,
    pub url: String,
    pub token: String,
}

#[derive(Deserialize)]
struct Directory {
    #[serde(rename = "newAccount")]
    new_account: String,
    #[serde(rename = "newNonce")]
    new_nonce: String,
    #[serde(rename = "newOrder")]
    new_order: String,
}
