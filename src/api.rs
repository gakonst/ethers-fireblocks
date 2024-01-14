// TODO: This file can be extracted to a separate crate.
use crate::{
    jwtclient::JwtSigner,
    types::{
        AssetResponse, CreateTransactionResponse, CreateVaultRequest, CreateVaultResponse,
        DepositAddressResponse, TransactionArguments, TransactionDetails, VaultAccountResponse, VaultAccountPaginatedResponse, AccountDetails,
    },
    FireblocksError, Result,
};

use jsonwebtoken::EncodingKey;
use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Serialize};

const FIREBLOCKS_API: &str = "https://sandbox-api.fireblocks.io";
const VERSION: &str = "v1";

#[derive(Debug, Clone)]
pub struct FireblocksClient {
    pub signer: JwtSigner,
    client: Client,
    url: String,
    version: String,
}

// This impl block contains the necessary API calls for interacting with Ethereum
impl FireblocksClient {
    pub fn new(key: EncodingKey, api_key: &str, api_url_override: Option<&str>) -> Self {
        let api_url = match api_url_override {
            Some(url) => url,
            None => FIREBLOCKS_API
        };
        Self::new_with_url(key, api_key, api_url)
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

    pub async fn get_account_details(&self, asset_id: &str, account_id: &str) -> Result<AccountDetails> {
        self.get(&format!("vault/accounts/{}/{}", account_id, asset_id)).await
    }

    pub async fn transaction(&self, txid: &str) -> Result<TransactionDetails> {
        self.get(&format!("transactions/{}", txid)).await
    }
}

// This impl block contains the underlying GET/POST helpers for authing to fireblocks
impl FireblocksClient {
    async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let path = format!("/{}/{}", self.version, path);
        let req = self.client.get(&format!("{}{}", self.url, path));
        self.send(&path, req, ()).await
    }

    async fn post<S: Serialize, R: DeserializeOwned>(&self, path: &str, body: S) -> Result<R> {
        let path = format!("/{}/{}", self.version, path);
        let req = self
            .client
            .post(&format!("{}{}", self.url, path))
            .json(&body);
        self.send(&path, req, body).await
    }

    async fn send<S: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        req: RequestBuilder,
        body: S,
    ) -> Result<R> {
        let req = self.authed(path, req, body)?;
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
}

// This impl block contains the rest of "nice to have" endpoints
impl FireblocksClient {
    pub async fn vaults(&self) -> Result<VaultAccountPaginatedResponse> {
        self.get("vault/accounts_paged").await
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

    // this section implements method useful in tests
    impl FireblocksClient {
        pub fn url(&self) -> &str {
            &self.url
        }
    }

    #[tokio::test]
    async fn v1_api() {
        let fireblocks_key = std::env::var("FIREBLOCKS_API_SECRET_PATH").unwrap();
        let api_key = std::env::var("FIREBLOCKS_API_KEY").expect("fireblocks api key not set");

        let rsa_pem = std::fs::read(fireblocks_key).unwrap();
        let key = EncodingKey::from_rsa_pem(&rsa_pem[..]).unwrap();
        let client = FireblocksClient::new(key, &api_key, None);

        assert_eq!(client.url(), FIREBLOCKS_API);

        let _res = client.vaults().await.unwrap();
        let _res = client.vault("0").await.unwrap();
        let _res = client.vault_addresses("0", "ETH_TEST3").await.unwrap();
        let _res = client.vault_wallet("0", "ETH_TEST3").await.unwrap();
        let _res = client
            // Creating a vault does not require approval?
            .new_vault(CreateVaultRequest {
                name: "test-acc".to_owned(),
                customer_ref_id: None,
                hidden_on_ui: false,
                auto_fuel: false,
            })
            .await
            .unwrap();
    }

    // test api url
}
