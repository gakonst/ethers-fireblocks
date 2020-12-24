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
    Fireblocks, FireblocksError,
};
use async_trait::async_trait;
use rustc_hex::ToHex;
use thiserror::Error;

#[derive(Debug)]
pub struct FireblocksMiddleware<M> {
    fireblocks: Fireblocks,
    inner: M,
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

    async fn send_transaction(
        &self,
        tx: TransactionRequest,
        _: Option<BlockNumber>,
    ) -> Result<PendingTransaction<'_, Self::Provider>, Self::Error> {
        let tx_hash = self.fireblocks.submit_transaction(&tx).await?;
        Ok(PendingTransaction::new(tx_hash, self.provider()))
    }

    async fn sign<T: Into<Bytes> + Send + Sync>(
        &self,
        data: T,
        _: &Address,
    ) -> Result<Signature, Self::Error> {
        Ok(self.fireblocks.sign_message(data.into()).await?)
    }
}

impl Fireblocks {
    async fn submit_transaction(&self, tx: &TransactionRequest) -> Result<TxHash, FireblocksError> {
        let ota = match tx.to {
            Some(ref to) => match to {
                NameOrAddress::Address(addr) => Some(OneTimeAddress {
                    address: format!("{:?}", addr),
                    tag: None,
                }),
                _ => panic!("no ens"),
            },
            _ => None,
        };

        // TODO: Tighten up creation of TransactionArguments
        let args = TransactionArguments {
            operation: TransactionOperation::CONTRACT_CALL,
            source: TransferPeerPath {
                peer_type: Some(PeerType::VAULT_ACCOUNT),
                id: Some(self.account_id.clone()),
            },
            destination: Some(DestinationTransferPeerPath {
                peer_type: PeerType::EXTERNAL_WALLET,
                id: ota
                    .as_ref()
                    .map(|to| {
                        self.account_ids
                            .get(&to.address[2..].parse::<Address>().unwrap())
                    })
                    .flatten()
                    .unwrap()
                    .to_string(),
                one_time_address: ota,
            }),
            extra_parameters: tx
                .data
                .as_ref()
                .map(|data| ExtraParameters::ContractCallData(data.0.to_hex::<String>())),

            // rest is jank
            asset_id: self.asset_id.clone(),
            amount: tx.value.unwrap_or_default().to_string(),
            gas_price: None, // tx.gas_price.unwrap_or_default().to_string(),
            gas_limit: None, // tx.gas.unwrap_or_default().to_string(),
            note: "".to_owned(),
        };

        self.handle_action(args, |details| {
            Ok(details.tx_hash[2..].parse::<TxHash>().unwrap())
        })
        .await
    }
}
