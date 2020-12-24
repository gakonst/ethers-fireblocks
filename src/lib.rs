mod jwtclient;
mod types;
use types::{TransactionArguments, TransactionDetails, TransactionStatus};

mod api;
pub use api::FireblocksClient;

mod middleware;
mod signer;

use ethers_core::types::Address;
use jsonwebtoken::EncodingKey;
use std::collections::HashMap;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, FireblocksError>;

#[derive(Debug, Error)]
pub enum FireblocksError {
    #[error(transparent)]
    JwtError(#[from] jwtclient::JwtError),

    #[error(transparent)]
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
    TxError(TransactionStatus, String),
}

/// Note that using Fireblocks as a signer WILL NOT take advantage of Fireblock's contextual
/// policy engine and will only use the signing functionalities.
///
/// You may alternatively use Fireblocks as a middleware.
#[derive(Debug, Clone)]
pub struct Fireblocks {
    fireblocks: FireblocksClient,
    account_ids: HashMap<Address, String>,
    pub chain_id: Option<u64>,
    pub asset_id: String,
    pub address: Address,
    pub account_id: String,
}

impl AsRef<FireblocksClient> for Fireblocks {
    fn as_ref(&self) -> &FireblocksClient {
        &self.fireblocks
    }
}

impl Fireblocks {
    pub async fn new(
        key: EncodingKey,
        account_id: &str,
        api_key: &str,
        chain_id: Option<u64>,
    ) -> Self {
        let fireblocks = FireblocksClient::new(key, api_key);
        let asset_id = match chain_id {
            Some(chain_id) => match chain_id {
                1 => "ETH",
                3 => "ETH_TEST",
                42 => "ETH_TEST2",
                _ => panic!("Unsupported chain_id"),
            },
            None => "ETH",
        };

        let res = fireblocks
            .vault_addresses(account_id, asset_id)
            .await
            .expect("could not get vault addrs");

        Self {
            fireblocks,
            account_ids: HashMap::new(),
            chain_id,
            asset_id: asset_id.to_owned(),
            address: res[0].address[2..]
                .parse()
                .expect("could not parse as address"),
            account_id: account_id.to_owned(),
        }
    }

    /// adds an account id to the mapping,
    // TODO: Remove once API does not require this
    pub fn add_account(&mut self, account_id: String, address: Address) {
        self.account_ids.insert(address, account_id);
    }

    async fn handle_action<F, R>(&self, args: TransactionArguments, func: F) -> Result<R>
    where
        F: FnOnce(TransactionDetails) -> Result<R>,
    {
        let res = self.fireblocks.create_transaction(args).await?;
        loop {
            let details = self.fireblocks.transaction(&res.id).await?;
            use TransactionStatus::*;
            match details.status {
                COMPLETED => return func(details),
                BLOCKED | CANCELLED | FAILED => {
                    return Err(FireblocksError::TxError(details.status, details.sub_status))
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tst() {
        let key = {
            let fireblocks_key = std::env::var("FIREBLOCKS_API_SECRET_PATH")
                .expect("fireblocks api secret key not set");
            let rsa_pem = std::fs::read(fireblocks_key).unwrap();
            EncodingKey::from_rsa_pem(&rsa_pem).unwrap()
        };
        let api_key = std::env::var("FIREBLOCKS_API_KEY").expect("fireblocks api key not set");
        let mut client = Fireblocks::new(key, "1", &api_key, Some(3)).await;
        let address: Address = "cbe74e21b070a979b9d6426b11e876d4cb618daf".parse().unwrap();
        let account_id = "af5371a9-9ff7-4015-9bf1-44bbf7964a04".to_owned();
        client.add_account(account_id, address);
        use rustc_hex::FromHex;
        let tx = TransactionRequest::new()
            .to(address)
            .data("ead710c40000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000548656c6c6f000000000000000000000000000000000000000000000000000000".from_hex::<Vec<u8>>().unwrap());
        let sig = client.sign_transaction(&tx).await.unwrap();
        // let tx = client.send_transaction(&tx).await.unwrap();
    }
}
