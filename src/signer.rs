use crate::{
    types::{
        ExtraParameters, PeerType, RawMessageData, TransactionArguments, TransactionOperation,
        TransferPeerPath, UnsignedMessage,
    },
    Fireblocks, FireblocksError,
};
use async_trait::async_trait;
use ethers_core::{
    types::{Address, Signature, TransactionRequest, H256},
    utils::hash_message,
};
use ethers_signers::Signer;

#[async_trait]
impl Signer for Fireblocks {
    type Error = FireblocksError;

    async fn sign_transaction(
        &self,
        tx: &TransactionRequest,
    ) -> Result<Signature, FireblocksError> {
        // get the sighash
        let sighash = tx.sighash(self.chain_id);
        self.sign_with_eip155(sighash, self.chain_id).await
    }

    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error> {
        let hash = hash_message(message);
        self.sign_with_eip155(hash, None).await
    }

    fn address(&self) -> Address {
        self.address
    }
}

impl Fireblocks {
    async fn sign_with_eip155(
        &self,
        hash: H256,
        _chain_id: Option<u64>,
    ) -> Result<Signature, FireblocksError> {
        // send the hash for signing - this will NOT take advantage
        // of the policy engine
        let args = TransactionArguments {
            operation: TransactionOperation::RAW,
            source: TransferPeerPath {
                peer_type: Some(PeerType::VAULT_ACCOUNT),
                id: Some(self.account_id.clone()),
            },
            extra_parameters: Some(ExtraParameters::RawMessageData(RawMessageData {
                messages: vec![UnsignedMessage {
                    content: hash.as_ref().to_vec(),
                }],
            })),
            // rest is jank
            asset_id: self.asset_id.clone(),
            amount: "".to_owned(),

            destination: None,
            gas_price: None,
            gas_limit: None,
            note: "".to_owned(),
        };

        // TODO: Parse the signature
        self.handle_action(args, |_details| unimplemented!()).await
    }
}
