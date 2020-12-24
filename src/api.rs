use crate::{
    jwtclient::JwtSigner,
    types::{
        AssetResponse, CreateTransactionResponse, CreateVaultRequest, CreateVaultResponse,
        DepositAddressResponse, TransactionArguments, TransactionDetails, VaultAccountResponse,
    },
    FireblocksError, Result,
};

use jsonwebtoken::EncodingKey;
use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Serialize};

const FIREBLOCKS_API: &str = "https://api.fireblocks.io";
const VERSION: &str = "v1";

#[derive(Debug, Clone)]
pub struct FireblocksClient {
    pub signer: JwtSigner,
    client: Client,
    url: String,
    version: String,
}

impl FireblocksClient {
    // TODO: Make this work by just providing the PEM file path
    pub fn new(key: EncodingKey, api_key: &str) -> Self {
        Self::new_with_url(key, api_key, FIREBLOCKS_API)
    }

    pub fn new_with_url(key: EncodingKey, api_key: &str, url: &str) -> Self {
        Self {
            signer: JwtSigner::new(key, api_key),
            client: Client::new(),
            url: url.to_owned(),
            version: VERSION.to_owned(),
        }
    }

    pub async fn create_transaction(
        &self,
        tx: TransactionArguments,
    ) -> Result<CreateTransactionResponse> {
        self.post("transactions", tx).await
    }

    pub async fn transaction(&self, txid: &str) -> Result<TransactionDetails> {
        self.get(&format!("transactions/{}", txid)).await
    }

    async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        // craft the path (e.g. /v1/vault/accounts)
        let path = format!("/{}/{}", self.version, path);
        // create the Get Request
        let req = self.client.get(&format!("{}{}", self.url, path));
        // pass it the auth params
        // TODO: Arbitrary data can be passed here. Is this an issue?
        let req = self.authed(&path, req, ())?;
        // send it
        let res = req.send().await?;
        let text = res.text().await?;
        let res: R =
            serde_json::from_str(&text).map_err(|err| FireblocksError::SerdeJson { err, text })?;
        Ok(res)
    }

    async fn post<S: Serialize, R: DeserializeOwned>(&self, path: &str, body: S) -> Result<R> {
        // craft the path (e.g. /v1/vault/accounts)
        let path = format!("/{}/{}", self.version, path);
        // create the POST Request
        let req = self
            .client
            .post(&format!("{}{}", self.url, path))
            .json(&body);
        // pass it the auth params
        let req = self.authed(&path, req, body)?;
        // send it
        let res = req.send().await?;
        let text = res.text().await?;
        let res: R =
            serde_json::from_str(&text).map_err(|err| FireblocksError::SerdeJson { err, text })?;
        Ok(res)
    }

    // Helper function which adds the necessary authorization headers to auth into the Fireblocks
    // API
    fn authed<S: Serialize>(
        &self,
        url: &str,
        req: RequestBuilder,
        body: S,
    ) -> Result<RequestBuilder> {
        let jwt = self.signer.sign(url, body)?;
        Ok(req
            .header("X-API-Key", &self.signer.api_key)
            .bearer_auth(jwt))
    }

    // The rest are "nice to have" API endpoints

    // GET /v1/vault/accounts
    pub async fn vaults(&self) -> Result<Vec<VaultAccountResponse>> {
        self.get("vault/accounts").await
    }

    pub async fn vault(&self, account_id: &str) -> Result<VaultAccountResponse> {
        self.get(&format!("vault/accounts/{}", account_id)).await
    }

    pub async fn vault_wallet(&self, account_id: &str, asset_id: &str) -> Result<AssetResponse> {
        self.get(&format!("vault/accounts/{}/{}", account_id, asset_id))
            .await
    }

    pub async fn new_vault(&self, req: CreateVaultRequest) -> Result<CreateVaultResponse> {
        self.post("vault/accounts", req).await
    }

    pub async fn vault_addresses(
        &self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<Vec<DepositAddressResponse>> {
        self.get(&format!(
            "vault/accounts/{}/{}/addresses",
            account_id, asset_id
        ))
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;

    static CLIENT: Lazy<Fireblocks> = Lazy::new(|| {
        let key = {
            let fireblocks_key = std::env::var("FIREBLOCKS_API_SECRET_PATH")
                .expect("fireblocks api secret key not set");
            let rsa_pem = std::fs::read(fireblocks_key).unwrap();
            let key2 = EncodingKey::from_rsa_pem(include_bytes!(
                "/Users/Georgios/.fireblocks/fireblocks.key"
            ))
            .unwrap();
            let key = EncodingKey::from_rsa_pem(&rsa_pem[..]).unwrap();
            assert_eq!(key, key2);
            key
        };
        let api_key = std::env::var("FIREBLOCKS_API_KEY").expect("fireblocks api key not set");
        // Fireblocks::new_with_url(key, &api_key, "http://localhost:8080")
        Fireblocks::new(key, &api_key)
    });

    #[tokio::test]
    // TODO: FIgure out why POST requests fail when GET requested do not
    async fn v1_api() {
        // let _res = CLIENT.vaults().await.unwrap();
        // let _res = CLIENT.vault("1").await.unwrap();
        // let _res = CLIENT.vault_addresses("1", "ETH_TEST").await.unwrap();
        // let _res = CLIENT.vault_wallet("1", "ETH_TEST").await.unwrap();
        let res = CLIENT
            .new_vault(CreateVaultRequest {
                name: "test-acc".to_owned(),
                customer_ref_id: None,
                hidden_on_ui: false,
                auto_fuel: false,
            })
            .await
            .unwrap();
        dbg!(res);
    }
}
