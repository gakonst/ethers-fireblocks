# <h1 align="center"> ethers-fireblocks </h1>

 Provides [ethers](https://github.com/gakonst/ethers-rs)-compatible Signer and Middleware
 implementations for the [Fireblocks Vaults API](https://fireblocks.com).

## Documentation

Clone the repository and run `cd ethers-fireblocks/ && cargo doc --open`

## Add ethers-fireblocks to your repository

```toml
[dependencies]

ethers-fireblocks = { git = "https://github.com/gakonst/ethers-fireblocks" }
```

To use the example, you must have the following env vars set:

 ```
export FIREBLOCKS_API_SECRET_PATH=<path to your fireblocks.key>
export FIREBLOCKS_API_KEY=<your fireblocks api key>
export FIREBLOCKS_SOURCE_VAULT_ACCOUNT=<the vault id being used for sending txs>
```

## Example Usage

 ```rust
use ethers_core::types::{Address, TransactionRequest};
use ethers_fireblocks::{Config, FireblocksMiddleware, FireblocksSigner};
use ethers_providers::{Middleware, Provider};
use std::convert::TryFrom;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let chain_id = 3; // Ropsten
    let cfg = Config::new(
        &std::env::var("FIREBLOCKS_API_SECRET_PATH").expect("fireblocks secret not set"),
        &std::env::var("FIREBLOCKS_API_KEY").expect("fireblocks api key not set"),
        &std::env::var("FIREBLOCKS_SOURCE_VAULT_ACCOUNT").expect("fireblocks source vault account not set"),
        chain_id,
    )?;

    // Create the signer (it can also be used with ethers_signers::Wallet)
    let mut signer = FireblocksSigner::new(cfg).await;

    // Instantiate an Ethers provider
    let provider = Provider::try_from("http://localhost:8545")?;
    // Wrap the provider with the fireblocks middleware
    let provider = FireblocksMiddleware::new(provider, signer);

    // Any state altering transactions issued will be signed using
    // Fireblocks. Wait for your push notification and approve on your phone...
    let address: Address = "cbe74e21b070a979b9d6426b11e876d4cb618daf".parse()?;
    let tx = TransactionRequest::new().to(address);
    let pending_tx = provider.send_transaction(tx, None).await?;
    // Everything else follows the normal ethers-rs APIs
    // e.g. we can get the receipt after 6 confs
    let receipt = pending_tx.confirmations(6).await?;

    Ok(())
}
 ```

 ## Sandbox environment

Fireblocks sandbox api is available at `https://sandbox-api.fireblocks.io` in contrast with test and production api available at `https://api.fireblocks.io`. By default `FireblocksSigner` connects to production url. You can override this behaviour (i.e. to connect to sandbox) by setting env var:
```
export FIREBLOCKS_API_URL_OVERRIDE="https://sandbox-api.fireblocks.io"
```
