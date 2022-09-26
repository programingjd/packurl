use crate::cache::{
    backup_account_keys, backup_account_kid, restore_account_keys, restore_account_kid,
    set_challenge_key,
};
use crate::domains::ACME_DOMAINS;
use crate::jose::{authorization_hash, jose};
use crate::{cache, LogLevel, CDN, LOG_LEVEL};
use colored::Colorize;
use rcgen::{Certificate, CertificateParams, CustomExtension, PKCS_ECDSA_P256_SHA256};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Response};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use rustls::sign::{any_ecdsa_type, CertifiedKey};
use rustls::PrivateKey;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Error, ErrorKind, Result};
use tokio::spawn;

const DIRECTORY_URL: &'static str = "https://acme-staging-v02.api.letsencrypt.org/directory";
const CONTACT: &'static str = "mailto:programingjd@gmail.com";

pub struct Account {
    keypair: EcdsaKeyPair,
    kid: String,
}

impl Account {
    pub async fn init() -> Result<Self> {
        let keypair = match restore_account_keys().await {
            Some(bytes) => {
                match LOG_LEVEL {
                    LogLevel::Info => {
                        println!("{}", "Restoring ACME account keys.");
                    }
                    _ => {}
                }
                EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, bytes.as_slice())
                    .map_err(|_| Error::new(ErrorKind::Other, "Failed to parse account keys."))?
            }
            None => {
                match LOG_LEVEL {
                    LogLevel::Info => {
                        println!("{}", "Creating ACME account keys.");
                    }
                    _ => {}
                }
                let rng = SystemRandom::new();
                let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                    .map_err(|_| Error::new(ErrorKind::Other, "Failed to create account keys."))?;
                let bytes = pkcs8.as_ref();
                backup_account_keys(bytes).await?;
                EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, bytes)
                    .map_err(|_| Error::new(ErrorKind::Other, "Failed to parse account keys."))?
            }
        };
        let kid = match restore_account_kid().await {
            Some(bytes) => {
                match LOG_LEVEL {
                    LogLevel::Info => {
                        println!("{}", "Restoring ACME account kid.");
                    }
                    _ => {}
                }
                String::from_utf8(bytes.to_vec())
                    .map_err(|err| Error::new(ErrorKind::Other, err))?
            }
            None => {
                match LOG_LEVEL {
                    LogLevel::Info => {
                        println!("{}", "Registering ACME account.");
                    }
                    _ => {}
                }
                let kid = Self::new_account(&keypair).await?;
                backup_account_kid(kid.as_bytes()).await?;
                kid
            }
        };
        Ok(Account { keypair, kid })
    }

    pub async fn auto_renew(&self) -> Result<()> {
        let client = Client::new();
        let directory = client
            .get(DIRECTORY_URL)
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
            Order::Invalid => return Err(Error::new(ErrorKind::Other, "Order is invalid.")),
            Order::Pending {
                authorizations,
                finalize,
            } => {
                authorizations
                    .iter()
                    .map(|url| self.authorize(&client, &directory, url.as_str()));
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    async fn new_account(keypair: &EcdsaKeyPair) -> Result<String> {
        let client = Client::new();
        match LOG_LEVEL {
            LogLevel::Info => {
                println!("{}", "Getting ACME directory.");
            }
            _ => {}
        }
        let directory = client
            .get(DIRECTORY_URL)
            .send()
            .await
            .map_err(|err| Error::new(ErrorKind::Other, err))?
            .json::<Directory>()
            .await
            .map_err(|err| Error::new(ErrorKind::Other, err))?;
        match LOG_LEVEL {
            LogLevel::Info => {
                println!("{}", "Requesting new nonce.");
            }
            _ => {}
        }
        let nonce = Self::new_nonce(&client, &directory).await?;

        match LOG_LEVEL {
            LogLevel::Info => {
                println!("{}", "Calling new account directory endpoint.");
            }
            _ => {}
        }
        let payload = json!({
            "termsOfServiceAgreed": true,
            "contact": vec![CONTACT]
        });
        let body = jose(
            keypair,
            &payload,
            None,
            nonce.as_str(),
            directory.new_account.as_str(),
        )?;
        let response = Self::jose_request(&client, directory.new_account.as_str(), &body).await?;
        if response.status().is_success() {
            let kid = response
                .headers()
                .get("location")
                .ok_or_else(|| Error::new(ErrorKind::Other, "Missing \"location\" header."))?
                .to_str()
                .map_err(|err| Error::new(ErrorKind::Other, err))?;
            Ok(kid.to_string())
        } else {
            match LOG_LEVEL {
                LogLevel::Error => {}
                _ => {
                    Self::print_request_error(response).await;
                }
            }
            Err(Error::new(
                ErrorKind::Other,
                "Failed to create new account.",
            ))
        }
    }

    async fn new_order(
        &self,
        client: &Client,
        directory: &Directory,
        domains: Vec<String>,
    ) -> Result<Order> {
        let nonce = Self::new_nonce(client, directory).await?;
        let identifiers: Vec<Identifier> = domains.into_iter().map(Identifier::Dns).collect();
        let payload = json!({ "identifiers": identifiers });
        let body = jose(
            &self.keypair,
            &payload,
            Some(self.kid.as_str()),
            nonce.as_str(),
            directory.new_order.as_str(),
        )?;
        let response = Self::jose_request(client, directory.new_order.as_str(), &body).await?;
        if response.status().is_success() {
            let order = response
                .json()
                .await
                .map_err(|err| Error::new(ErrorKind::Other, err))?;
            Ok(order)
        } else {
            match LOG_LEVEL {
                LogLevel::Error => {}
                _ => {
                    Self::print_request_error(response).await;
                }
            }
            Err(Error::new(ErrorKind::Other, "Failed to create new order."))
        }
    }

    async fn authorize(&self, client: &Client, directory: &Directory, url: &str) -> Result<()> {
        let nonce = Self::new_nonce(client, directory).await?;
        let payload = json!("");
        let body = jose(
            &self.keypair,
            &payload,
            Some(self.kid.as_str()),
            nonce.as_str(),
            url,
        )?;
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
                    let challenge = challenges
                        .iter()
                        .find(|it| it.typ == ChallengeType::TlsAlpn01)
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Unsupported,
                                "TlsAlpn01 challenge is not available.",
                            )
                        })?;
                    let auth = authorization_hash(&self.keypair, challenge.token.as_str())?;
                    let mut params = CertificateParams::new(ACME_DOMAINS);
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
                    set_challenge_key(key);
                    let nonce = Self::new_nonce(client, directory).await?;
                    let payload = json!({});
                    let body = jose(
                        &self.keypair,
                        &payload,
                        Some(self.kid.as_str()),
                        nonce.as_str(),
                        challenge.url.as_str(),
                    )?;
                    let response = Self::jose_request(client, url, &body).await?;
                    if response.status().is_success() {
                        Ok(())
                    } else {
                        match LOG_LEVEL {
                            LogLevel::Error => {}
                            _ => {
                                Self::print_request_error(response).await;
                            }
                        }
                        Err(Error::new(ErrorKind::Other, "Failed to trigger challenge."))
                    }
                }
                Auth::Valid => Ok(()),
                Auth::Invalid | Auth::Expired | Auth::Revoked => {
                    Err(Error::new(ErrorKind::Other, "ACME auth failed."))
                }
            }
        } else {
            match LOG_LEVEL {
                LogLevel::Error => {}
                _ => {
                    Self::print_request_error(response).await;
                }
            }
            Err(Error::new(ErrorKind::Other, "Failed to authorize url."))
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
            .ok_or_else(|| Error::new(ErrorKind::Other, "Missing \"replay-nonce\" header."))?
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