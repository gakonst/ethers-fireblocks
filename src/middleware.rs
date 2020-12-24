use ethers_core::types::{
    Address, BlockNumber, Bytes, NameOrAddress, Signature, TransactionRequest, TxHash,
};
use ethers_providers::{FromErr, Middleware, PendingTransaction};
use ethers_signers::Signer;

use crate::{
    types::{
        DestinationTransferPeerPath, ExtraParameters, OneTimeAddress, PeerType,
        TransactionArguments, TransactionOperation, TransferPeerPath,
    },
    FireblocksError, FireblocksSigner,
};
use async_trait::async_trait;
use rustc_hex::ToHex;
use thiserror::Error;

#[derive(Debug)]
/// The `FireblocksMiddleware` is an ethers-compatible middleware which sends transactions
/// and signs messages using Fireblocks' API. Sending transactions utilizes the `CONTRACT_CALL`
/// mode and signing messages utilizes the `RAW` mode.
pub struct FireblocksMiddleware<M> {
    fireblocks: FireblocksSigner,
    inner: M,
}

impl<M: Middleware> FireblocksMiddleware<M> {
    /// Creates a new FireblocksMiddleware.
    pub fn new(inner: M, fireblocks: FireblocksSigner) -> Self {
        Self { inner, fireblocks }
    }
}

#[derive(Error, Debug)]
pub enum FireblocksMiddlewareError<M: Middleware> {
    #[error(transparent)]
    FireblocksError(#[from] FireblocksError),
    #[error("{0}")]
    MiddlewareError(M::Error),
}

impl<M: Middleware> FromErr<M::Error> for FireblocksMiddlewareError<M> {
    fn from(err: M::Error) -> FireblocksMiddlewareError<M> {
        FireblocksMiddlewareError::MiddlewareError(err)
    }
}

#[async_trait]
impl<M: Middleware> Middleware for FireblocksMiddleware<M> {
    type Provider = M::Provider;
    type Inner = M;
    type Error = FireblocksMiddlewareError<M>;

    fn inner(&self) -> &Self::Inner {
        &self.inner
    }

    /// Submits a transaction with the Fireblocks CONTRACT_CALL mode and returns
    /// a pending transaction object.
    async fn send_transaction(
        &self,
        tx: TransactionRequest,
        _: Option<BlockNumber>,
    ) -> Result<PendingTransaction<'_, Self::Provider>, Self::Error> {
        let tx_hash = self
            .fireblocks
            .submit_transaction(&tx, "".to_owned())
            .await?;
        Ok(PendingTransaction::new(tx_hash, self.provider()))
    }

    /// Signs a message using Fireblocks' Signer. Uses the RAW operation mode under
    /// the hood.
    async fn sign<T: Into<Bytes> + Send + Sync>(
        &self,
        data: T,
        _: &Address,
    ) -> Result<Signature, Self::Error> {
        Ok(self.fireblocks.sign_message(data.into()).await?)
    }
}

impl FireblocksSigner {
    /// Submits a transaction with the Fireblocks `CONTRACT_CALL` mode, using the provided
    /// note.
    pub async fn submit_transaction(
        &self,
        tx: &TransactionRequest,
        note: String,
    ) -> Result<TxHash, FireblocksError> {
        let args = TransactionArguments {
            operation: TransactionOperation::CONTRACT_CALL,
            source: TransferPeerPath {
                peer_type: Some(PeerType::VAULT_ACCOUNT),
                id: Some(self.account_id.clone()),
            },
            destination: self.to_destination(tx.to.as_ref()),
            extra_parameters: tx
                .data
                .as_ref()
                .map(|data| ExtraParameters::ContractCallData(data.0.to_hex::<String>())),

            // rest is unnecessary
            asset_id: self.asset_id.clone(),
            amount: tx.value.unwrap_or_default().to_string(),
            gas_price: tx.gas_price.map(|x| x.to_string()),
            gas_limit: tx.gas.map(|x| x.to_string()),
            note,
        };

        self.handle_action(args, |details| {
            details.tx_hash[2..]
                .parse::<TxHash>()
                .map_err(|err| FireblocksError::ParseError(err.to_string()))
        })
        .await
    }

    fn to_destination(&self, to: Option<&NameOrAddress>) -> Option<DestinationTransferPeerPath> {
        match to {
            Some(ref to) => match to {
                NameOrAddress::Address(addr) => {
                    let id = self.account_ids.get(addr);
                    id.map(|id| {
                        let ota = OneTimeAddress {
                            address: format!("{:?}", addr),
                            tag: None,
                        };
                        DestinationTransferPeerPath {
                            peer_type: PeerType::EXTERNAL_WALLET,
                            id: id.clone(),
                            one_time_address: Some(ota),
                        }
                    })
                }
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_signer;
    use ethers_providers::Provider;
    use rustc_hex::FromHex;
    use std::convert::TryFrom;

    #[tokio::test]
    async fn broadcasts_tx() {
        let fireblocks = test_signer().await;

        // create the middleware
        let inner =
            Provider::try_from("https://ropsten.infura.io/v3/fd8b88b56aa84f6da87b60f5441d6778")
                .unwrap();
        let provider = FireblocksMiddleware::new(inner, fireblocks);

        // make a simple setGreeting transaction and illustrate that it works
        // with the ethers-middleware arch
        let tx = TransactionRequest::new()
            .send_to_str("cbe74e21b070a979b9d6426b11e876d4cb618daf").unwrap()
            .data("ead710c40000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000548656c6c6f000000000000000000000000000000000000000000000000000000".from_hex::<Vec<u8>>().unwrap());
        let pending_tx = provider.send_transaction(tx, None).await.unwrap();
        let tx_hash = *pending_tx;
        let receipt = pending_tx.await.unwrap();
        assert_eq!(receipt.transaction_hash, tx_hash);
    }
}
