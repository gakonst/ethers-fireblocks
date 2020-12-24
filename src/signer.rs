use crate::{
    types::{
        ExtraParameters, PeerType, RawMessageData, TransactionArguments, TransactionOperation,
        TransferPeerPath, UnsignedMessage,
    },
    FireblocksError, FireblocksSigner,
};
use async_trait::async_trait;
use ethers_core::{
    types::{Address, Signature, TransactionRequest, H256},
    utils::hash_message,
};
use ethers_signers::{to_eip155_v, Signer};
use rustc_hex::ToHex;

#[async_trait]
impl Signer for FireblocksSigner {
    type Error = FireblocksError;

    async fn sign_transaction(
        &self,
        tx: &TransactionRequest,
    ) -> Result<Signature, FireblocksError> {
        let sighash = tx.sighash(self.chain_id);
        self.sign_with_eip155(tx, sighash, self.chain_id).await
    }

    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error> {
        let hash = hash_message(&message);
        self.sign_with_eip155(message.as_ref(), hash, None).await
    }

    fn address(&self) -> Address {
        self.address
    }
}

impl FireblocksSigner {
    async fn sign_with_eip155<S: serde::Serialize>(
        &self,
        preimage: S,
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
                .parse::<H256>()
                .map_err(|err| FireblocksError::ParseError(err.to_string()))?;
            let s = sig
                .s
                .parse::<H256>()
                .map_err(|err| FireblocksError::ParseError(err.to_string()))?;
            let v = to_eip155_v(sig.v as u8, self.chain_id);
            Ok(Signature { r, s, v })
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_signer;
    use rustc_hex::FromHex;

    #[tokio::test]
    async fn can_sign_transaction() {
        let signer = test_signer().await;
        let address: Address = "cbe74e21b070a979b9d6426b11e876d4cb618daf".parse().unwrap();
        let tx = TransactionRequest::new()
            .to(address)
            .data("ead710c40000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000548656c6c6f000000000000000000000000000000000000000000000000000000".from_hex::<Vec<u8>>().unwrap());
        let sig = signer.sign_transaction(&tx).await.unwrap();
        sig.verify(tx.sighash(Some(3)), signer.address()).unwrap();

        let msg = "Hello World";
        let sig = signer.sign_message(msg).await.unwrap();
        sig.verify(msg, signer.address()).unwrap();
    }
}
