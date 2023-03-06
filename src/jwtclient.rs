use jsonwebtoken::{errors as jwterrors, Algorithm, EncodingKey, Header};

use digest::Digest;
use rustc_hex::ToHex;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use rand::Rng;

const EXPIRY: u64 = 55;

#[derive(Debug, Clone)]
pub struct JwtSigner {
    // TODO: Make this work with Zeroize/Secrecy
    pub key: EncodingKey,
    pub api_key: String,
}

impl JwtSigner {
    pub fn new(key: EncodingKey, api_key: &str) -> Self {
        Self {
            key,
            api_key: api_key.to_string(),
        }
    }

    pub fn sign<S: Serialize>(&self, path: &str, body: S) -> Result<String, JwtError> {
        let header = Header::new(Algorithm::RS256);
        let claims = Claims::new(path, &self.api_key, body)?;
        Ok(jsonwebtoken::encode(&header, &claims, &self.key)?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
/// JWT Claims as specified in https://docs.fireblocks.com/api/#signing-a-request
struct Claims<'a> {
    /// The URI part of the request (e.g., /v1/transactions)
    uri: &'a str,
    /// Constantly increasing number. Usually, a timestamp can be used.
    nonce: u64,
    /// The time at which the JWT was issued, in seconds since Epoch.
    iat: u64,
    /// The expiration time on and after which the JWT must not be accepted for processing, in seconds since Epoch. Must be less than iat+30sec.
    exp: u64,
    /// The API key
    sub: &'a str,
    #[serde(rename = "bodyHash")]
    /// Hex-encoded SHA-256 hash of the raw HTTP request body.
    body_hash: String,
}

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Could not serialize JWT body: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Could not create JWT time: {0}")]
    Time(#[from] std::time::SystemTimeError),
    #[error(transparent)]
    Jwt(#[from] jwterrors::Error),
}

impl<'a> Claims<'a> {
    fn new<S: Serialize>(uri: &'a str, sub: &'a str, body: S) -> Result<Self, JwtError> {
        // use millisecond precision to ensure that it's not reused
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        let mut rng = rand::thread_rng();
        let nonce = rng.gen::<u64>();
        let now = now / 1000;

        let body_hash = {
            let mut digest = Sha256::new();
            digest.update(serde_json::to_vec(&body)?);
            digest.finalize().to_vec()
        };

        Ok(Self {
            uri,
            sub,
            body_hash: body_hash.to_hex::<String>(),
            nonce,
            iat: now,
            exp: now + EXPIRY,
        })
    }
}
