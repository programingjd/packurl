use base64::URL_SAFE_NO_PAD;
use ring::digest::{digest, SHA256};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair};
use serde::Serialize;
use serde_json::Value;
use std::io::{Error, ErrorKind, Result};

pub fn jose(
    keypair: &EcdsaKeyPair,
    payload: Option<Value>,
    kid: Option<&str>,
    nonce: &str,
    url: &str,
) -> Result<Value> {
    let (x, y) = keypair.public_key().as_ref()[1..].split_at(32);
    let jwk = match kid {
        None => Some(Jwk {
            alg: "ES256",
            crv: "P-256",
            kty: "EC",
            u: "sig",
            x: base64::encode_config(x, URL_SAFE_NO_PAD),
            y: base64::encode_config(y, URL_SAFE_NO_PAD),
        }),
        _ => None,
    };
    let protected = Protected {
        alg: "ES256",
        jwk,
        kid,
        nonce,
        url,
    };
    let protected = base64::encode_config(
        serde_json::to_vec(&protected).map_err(|err| Error::new(ErrorKind::InvalidData, err))?,
        URL_SAFE_NO_PAD,
    );
    let payload = match payload {
        Some(payload) => base64::encode_config(payload.to_string(), URL_SAFE_NO_PAD),
        None => String::new(),
    };
    let message = format!("{}.{}", protected, payload);
    let signature = keypair
        .sign(&SystemRandom::new(), message.as_bytes())
        .map_err(|_| Error::new(ErrorKind::Other, "Failed to sign message."))?;
    let signature = base64::encode_config(signature.as_ref(), URL_SAFE_NO_PAD);
    let body = Body {
        protected,
        payload,
        signature,
    };
    serde_json::to_value(&body).map_err(|err| Error::new(ErrorKind::InvalidData, err))
}

pub fn authorization_hash(keypair: &EcdsaKeyPair, token: &str) -> Result<Vec<u8>> {
    let (x, y) = keypair.public_key().as_ref()[1..].split_at(32);
    let jwk = Jwk {
        alg: "ES256",
        crv: "P-256",
        kty: "EC",
        u: "sig",
        x: base64::encode_config(x, URL_SAFE_NO_PAD),
        y: base64::encode_config(y, URL_SAFE_NO_PAD),
    };
    let thumbprint = base64::encode_config(
        digest(
            &SHA256,
            &serde_json::to_vec(&JwkThumb {
                crv: jwk.crv,
                kty: jwk.kty,
                x: &jwk.x,
                y: &jwk.y,
            })
            .map_err(|err| Error::new(ErrorKind::InvalidData, err))?,
        ),
        URL_SAFE_NO_PAD,
    );
    Ok(
        digest(&SHA256, format!("{}.{}", token, &thumbprint).as_bytes())
            .as_ref()
            .to_vec(),
    )
}

#[derive(Serialize)]
struct Jwk {
    alg: &'static str,
    crv: &'static str,
    kty: &'static str,
    #[serde(rename = "use")]
    u: &'static str,
    x: String,
    y: String,
}

#[derive(Serialize)]
struct JwkThumb<'a> {
    crv: &'a str,
    kty: &'a str,
    x: &'a str,
    y: &'a str,
}

#[derive(Serialize)]
struct Protected<'a> {
    alg: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    jwk: Option<Jwk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kid: Option<&'a str>,
    nonce: &'a str,
    url: &'a str,
}

#[derive(Serialize)]
struct Body {
    protected: String,
    payload: String,
    signature: String,
}
