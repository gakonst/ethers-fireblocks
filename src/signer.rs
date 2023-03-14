use crate::{
    types::{
        ExtraParameters, PeerType, RawMessageData, TransactionArguments, TransactionOperation,
        TransferPeerPath, UnsignedMessage,
    },
    FireblocksError, FireblocksSigner,
};
use async_trait::async_trait;
use ethers_core::{
    types::{transaction::{eip2718::TypedTransaction, eip712::Eip712}, 
        Address, Signature, H256, U256, },
    utils::hash_message,
};
use ethers_signers::{to_eip155_v, Signer};
use rustc_hex::ToHex;

#[async_trait]
impl Signer for FireblocksSigner {
    type Error = FireblocksError;

    async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature, FireblocksError> {
        let mut tx_with_chain = tx.clone();
        if tx_with_chain.chain_id().is_none() {
            // in the case we don't have a chain_id, let's use the signer chain id instead
            tx_with_chain.set_chain_id(self.chain_id);
        }
        let sighash = tx_with_chain.sighash();
        self.sign(tx_with_chain, sighash, true).await
    }

    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error> {
        let hash = hash_message(&message);
        self.sign(message.as_ref(), hash, false).await
    }

    /// Signs an EIP712 encoded domain separator and message
    /// TODO: Implement
    #[allow(unused_variables)]
    async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        payload: &T,
    ) -> Result<Signature, Self::Error> {
        unimplemented!()
    }

    fn address(&self) -> Address {
        self.address
    }

    fn with_chain_id<T: Into<u64>>(mut self, chain_id: T) -> Self {
        self.chain_id = chain_id.into();
        self
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }
}

impl FireblocksSigner {
    async fn sign<S: serde::Serialize>(
        &self,
        preimage: S,
        hash: H256,
        is_eip155: bool,
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
                    content: hash.as_ref().to_hex::<String>(),
                }],
            })),

            // rest is unnecessary
            asset_id: self.asset_id.clone(),
            amount: "".to_owned(),
            destination: None,
            gas_price: None,
            gas_limit: None,
            note: serde_json::to_string(&preimage).map_err(|err| FireblocksError::SerdeJson {
                err,
                text: "failed to serialize tx/message".to_owned(),
            })?,
        };

        // Parse the signature returned from the API
        self.handle_action(args, |details| {
            let sig = &details.signed_messages[0].signature;
            let r = sig
                .r
                .parse::<U256>()
                .map_err(|err| FireblocksError::ParseError(err.to_string()))?;
            let s = sig
                .s
                .parse::<U256>()
                .map_err(|err| FireblocksError::ParseError(err.to_string()))?;
            let v = if is_eip155 {
                to_eip155_v(sig.v as u8, self.chain_id)
            } else {
                sig.v + 27
            };
            Ok(Signature { r, s, v })
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_signer;
    use ethers_core::types::TransactionRequest;
    use rustc_hex::FromHex;

    #[tokio::test]
    async fn can_sign_transaction() {
        let signer = test_signer().await;
        let address: Address = "cbe74e21b070a979b9d6426b11e876d4cb618daf".parse().unwrap();
        let tx = TransactionRequest::new()
            .to(address)
            .chain_id(5)
            .data("ead710c40000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000548656c6c6f000000000000000000000000000000000000000000000000000000".from_hex::<Vec<u8>>().unwrap());
        let sighash = tx.sighash();
        let sig = signer.sign_transaction(&tx.into()).await.unwrap();
        sig.verify(sighash, signer.address()).unwrap();
    }

    #[tokio::test]
    async fn can_sign_msg() {
        let signer = test_signer().await;
        let msg = "Hello World 2";
        let sig = signer.sign_message(msg).await.unwrap();
        sig.verify(msg, signer.address()).unwrap();
    }
}
