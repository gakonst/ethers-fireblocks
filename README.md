# ethers-fireblocks

 Provides [ethers](https://github.com/gakonst/ethers-rs)-compatible Signer and Middleware
 implementations for the Fireblocks API.

 ```rust
use ethers_core::types::{Address, TransactionRequest};
use ethers_fireblocks::{Config, FireblocksMiddleware, FireblocksSigner};
use ethers_providers::{Middleware, Provider};
use std::convert::TryFrom;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wallet_id = "1"; // Our wallet id
    let chain_id = Some(3); // Ropsten
    let cfg = Config::new(
        &std::env::var("FIREBLOCKS_SECRET_PATH").expect("fireblocks secret not set"),
        &std::env::var("FIREBLOCKS_API_KEY").expect("fireblocks api key not set"),
        wallet_id,
        chain_id,
    )?;

    // Create the signer (it can also be used with ethers_signers::Wallet)
    let mut signer = FireblocksSigner::new(cfg).await;

    // Associate the wallet id to the address you're calling
    // (this will be no longer required in the future)
    let address: Address = "cbe74e21b070a979b9d6426b11e876d4cb618daf".parse()?;
    let address_id = std::env::var("EXTERNAL_WALLET_ID").expect("external wallet id not set");
    signer.add_account(address_id, address);

    // Instantiate an Ethers provider
    let provider = Provider::try_from("http://localhost:8545")?;
    // Wrap the provider with the fireblocks middleware
    let provider = FireblocksMiddleware::new(provider, signer);

    // Any state altering transactions issued will be signed using
    // Fireblocks. Wait for your push notification and approve on your phone...
    let tx = TransactionRequest::new().to(address);
    let pending_tx = provider.send_transaction(tx, None).await?;
    // Everything else follows the normal ethers-rs APIs
    // e.g. we can get the receipt after 6 confs
    let receipt = pending_tx.confirmations(6).await?;

    Ok(())
}
 ```
