//! # ethers-fireblocks
//!
//! Provides [ethers](https://docs.rs/ethers)-compatible Signer and Middleware
//! implementations for the Fireblocks API.
//!
//! ```rust,no_run
//! # async fn broadcasts_tx() -> Result<(), Box<dyn std::error::Error>> {
//! use ethers_providers::{Middleware, Provider};
//! use ethers_core::types::Address;
//! use ethers_fireblocks::{FireblocksSigner, FireblocksMiddleware, Config};
//! use std::convert::TryFrom;
//!
//! let cfg = Config::new(
//!     "~/.fireblocks/fireblocks.key",
//!     &std::env::var("FIREBLOCKS_API_KEY").expect("fireblocks api key not set"),
//!     "1",
//!     3,
//! )?;
//! // The signer can be used with Ethers' Wallet.
//! let mut signer = FireblocksSigner::new(cfg).await;
//!
//! // You must add each address you will be calling to the Address map.
//! // example below uses the Greeter contract deployed by the Fireblocks team on
//! // Ropsten.
//! let address: Address = "cbe74e21b070a979b9d6426b11e876d4cb618daf".parse()?;
//! let address_id = std::env::var("EXTERNAL_WALLET_ID").expect("external wallet id not set");
//! signer.add_account(address_id, address);
//! let provider = Provider::try_from("http://localhost:8545")?;
//! let provider = FireblocksMiddleware::new(provider, signer);
//! # Ok(())
//! # }
//! ```
mod jwtclient;
mod types;
use types::{TransactionArguments, TransactionDetails, TransactionStatus};

mod api;
use api::FireblocksClient;

mod signer;

mod middleware;
pub use middleware::FireblocksMiddleware;

use ethers_core::types::Address;
use jsonwebtoken::EncodingKey;
use std::{collections::HashMap, time::Instant};
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, FireblocksError>;

#[derive(Debug, Error)]
/// Fireblocks API related errors
pub enum FireblocksError {
    #[error(transparent)]
    /// Thrown when JWT signing fails
    JwtError(#[from] jwtclient::JwtError),

    #[error(transparent)]
    /// Thrown when we cannot parse the RSA PEM file
    JwtParseError(#[from] jsonwebtoken::errors::Error),

    #[error(transparent)]
    /// Thrown when we cannot find the RSA PEM file
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    /// Thrown when submitting a POST/GET request fails
    ReqwestError(#[from] reqwest::Error),

    #[error("Deserialization Error: {err}. Response: {text}")]
    /// Serde JSON Error
    SerdeJson {
        err: serde_json::Error,
        text: String,
    },

    #[error(
        "Transaction was not completed successfully. Final Status: {:?}. Sub status: {1}",
        0
    )]
    /// Thrown when a transaction submission or message signing fails
    TxError(TransactionStatus, String),

    #[error("Could not parse data: {0}")]
    /// Thrown when parsing string as Ethereum data fails
    ParseError(String),

    #[error("Timed out while waiting for user to approve transaction")]
    Timeout,
}

#[derive(Debug, Clone)]
/// FireblocksSigner is a [`Signer`](ethers_signers::Signer) which utilizes Fireblocks'
/// MPC signing over its [API](https://docs.fireblocks.io/api) instead of a local private key.
///
/// Note: Using FireblocksSigner as a signer WILL NOT take advantage of Fireblock's contextual
/// policy engine and will only use the RAW signing functionalities.
///
/// Consider using [`FireblocksMiddleware`](crate::FireblocksMiddleware) to have an integrated
/// ethers [`Middleware`](eters_middleware::Middleware) experience.
pub struct FireblocksSigner {
    fireblocks: FireblocksClient,
    account_ids: HashMap<Address, String>,
    chain_id: u64,
    asset_id: String,
    address: Address,
    account_id: String,
    timeout: u128,
}

/// Configuration options for instantiating a [`FireblocksSigner`](FireblocksSigner)
pub struct Config {
    /// The RSA key file.
    pub key: EncodingKey,
    /// The API key which was provided to you by fireblocks support
    pub api_key: String,
    /// The chain id of the network you are connecting to
    pub chain_id: u64,
    /// Your vault's account id.
    pub account_id: String,
}

impl Config {
    /// Instantiates the config file given a path to the RSA file as well as the rest of the config
    /// args.
    pub fn new<T: AsRef<str>>(
        key: T,
        api_key: &str,
        account_id: &str,
        chain_id: u64,
    ) -> Result<Self> {
        let rsa_pem = std::fs::read(key.as_ref())?;
        let key = EncodingKey::from_rsa_pem(&rsa_pem)?;

        Ok(Self {
            key,
            chain_id,
            api_key: api_key.to_string(),
            account_id: account_id.to_string(),
        })
    }
}

impl AsRef<FireblocksClient> for FireblocksSigner {
    fn as_ref(&self) -> &FireblocksClient {
        &self.fireblocks
    }
}

impl FireblocksSigner {
    /// Instantiates a FireblocksSigner with the provided config
    pub async fn new(cfg: Config) -> Self {
        let fireblocks = FireblocksClient::new(cfg.key, &cfg.api_key);
        let asset_id = match cfg.chain_id {
            1 => "ETH",
            3 => "ETH_TEST",
            5 => "ETH_TEST3",
            42 => "ETH_TEST2",
            _ => panic!("Unsupported chain_id"),
        };

        let res = fireblocks
            .vault_addresses(&cfg.account_id, asset_id)
            .await
            .expect("could not get vault addrs");

        Self {
            fireblocks,
            account_ids: HashMap::new(),
            chain_id: cfg.chain_id,
            asset_id: asset_id.to_owned(),
            address: res[0].address[2..]
                .parse()
                .expect("could not parse as address"),
            account_id: cfg.account_id,
            timeout: 60_000,
        }
    }

    /// Sets the timeout duration in milliseconds. If the user does not approve a
    /// transaction within this time, the transaction request throws an error.
    pub fn timeout(&mut self, timeout_ms: u128) {
        self.timeout = timeout_ms;
    }

    /// Registers an Account ID to Address mapping.
    pub fn add_account(&mut self, account_id: String, address: Address) {
        self.account_ids.insert(address, account_id);
    }

    async fn handle_action<F, R>(&self, args: TransactionArguments, func: F) -> Result<R>
    where
        F: FnOnce(TransactionDetails) -> Result<R>,
    {
        let res = self.fireblocks.create_transaction(args).await?;
        let start = Instant::now();
        loop {
            if Instant::now().duration_since(start).as_millis() >= self.timeout {
                return Err(FireblocksError::Timeout);
            }

            let details = self.fireblocks.transaction(&res.id).await?;
            use TransactionStatus::*;
            // Loops in pending signature
            match details.status {
                BROADCASTING | COMPLETED => return func(details),
                BLOCKED | CANCELLED | FAILED => {
                    return Err(FireblocksError::TxError(details.status, details.sub_status))
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
async fn test_signer() -> FireblocksSigner {
    let config = Config::new(
        std::env::var("FIREBLOCKS_API_SECRET_PATH").unwrap(),
        &std::env::var("FIREBLOCKS_API_KEY").unwrap(),
        &std::env::var("FIREBLOCKS_SOURCE_VAULT_ACCOUNT").unwrap(),
        5,
    )
    .unwrap();
    FireblocksSigner::new(config).await
}
