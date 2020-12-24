use serde::{Deserialize, Serialize};
// TODO: Make all fields public

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultAccountResponse {
    id: String,
    name: String,
    #[serde(rename = "hiddenOnUI")]
    hidden_on_ui: bool,
    assets: Vec<AssetResponse>,
    #[serde(rename = "customerRefId")]
    customer_ref_id: Option<String>,
    #[serde(rename = "autoFuel")]
    auto_fuel: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaultRequest {
    pub name: String,
    #[serde(rename = "hiddenOnUI")]
    pub hidden_on_ui: bool,
    #[serde(rename = "customerRefId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_ref_id: Option<String>,
    // Field order matters :(
    #[serde(rename = "autoFuel")]
    pub auto_fuel: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaultResponse {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetResponse {
    id: String,
    total: String,
    /// DEPRECATED
    balance: Option<String>,
    #[serde(rename = "lockedAmount")]
    locked_amount: Option<String>,
    available: Option<String>,
    pending: Option<String>,
    // This should be Option<EosOpts>
    // enum ProtocolOpts {
    //  EOS { cpu, staked network , ... }
    // }
    #[serde(rename = "selfStakedCPU")]
    self_staked_cpu: Option<String>,
    #[serde(rename = "selfStakedNetwork")]
    self_staked_network: Option<String>,
    #[serde(rename = "pendingRefundCPU")]
    pending_refund_cpu: Option<String>,
    #[serde(rename = "pendingRefundNetwork")]
    pending_refund_network: Option<String>,
    #[serde(rename = "totalStakedCPU")]
    total_staked_cpu: Option<String>,
    #[serde(rename = "totalStakedNetwork")]
    total_staked_network: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
// TODO: Figure out how to deserialize empty as None.
pub struct DepositAddressResponse {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub address: String,
    pub tag: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "legacyAddress")]
    pub legacy_address: Option<String>,
    #[serde(rename = "customerRefId")]
    pub customer_ref_id: Option<String>,
    #[serde(rename = "addressFormat")]
    pub address_format: Option<String>,
}

// The APIs feel a bit weird: In trying to create a unified API, it might be good
// to combine these options in enums
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionArguments {
    #[serde(rename = "assetId")]
    pub asset_id: String,

    pub operation: TransactionOperation,

    pub source: TransferPeerPath,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationTransferPeerPath>,

    pub amount: String,

    pub extra_parameters: Option<ExtraParameters>,

    // pub fee: String,

    // #[serde(rename = "feeLevel")]
    // pub fee_level: FeeLevel,
    // #[serde(rename = "failOnLowFee")]
    // pub fail_on_low_fee: bool,

    // #[serde(rename = "maxFee")]
    // pub max_fee: String,
    #[serde(rename = "gasPrice")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<String>,
    #[serde(rename = "gasLimit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,

    // #[serde(rename = "cpuStaking")]
    // pub cpu_staking: usize,
    // #[serde(rename = "networkStaking")]
    // pub network_staking: usize,
    // #[serde(rename = "autoStaking")]
    // pub auto_staking: bool,
    // #[serde(rename = "customerRefId")]
    // pub customer_ref_id: String,
    // #[serde(rename = "replaceTxByHash")]
    // pub replace_tx_by_hash: String,
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtraParameters {
    ContractCallData(String),
    RawMessageData(RawMessageData),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FeeLevel {
    HIGH,
    MEDIUM,
    LOW,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferPeerPath {
    #[serde(rename = "type")]
    pub peer_type: Option<PeerType>,
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationTransferPeerPath {
    #[serde(rename = "type")]
    pub peer_type: PeerType,
    pub id: String,
    #[serde(rename = "oneTimeAddress")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_time_address: Option<OneTimeAddress>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OneTimeAddress {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransactionOperation {
    TRANSFER,
    RAW,
    CONTRACT_CALL,

    MINT,
    BURN,
    SUPPLY_TO_COMPOUND,
    REDEEM_FROM_COMPOUND,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PeerType {
    VAULT_ACCOUNT,
    EXCHANGE_ACCOUNT,
    INTERNAL_WALLET,
    EXTERNAL_WALLET,
    UNKNOWN,
    NETWORK_CONNECTION,
    FIAT_ACCOUNT,
    COMPOUND,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransactionResponse {
    pub id: String,
    pub status: TransactionStatus,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransactionStatus {
    SUBMITTED,
    QUEUED,
    PENDING_SIGNATURE,
    PENDING_AUTHORIZATION,
    PENDING_3RD_PARTY_MANUAL_APPROVAL,
    PENDING_3RD_PARTY,
    /**
     * @deprecated
     */
    PENDING,
    BROADCASTING,
    CONFIRMING,
    /**
     * @deprecated Replaced by "COMPLETED"
     */
    CONFIRMED,
    COMPLETED,
    PENDING_AML_SCREENING,
    PARTIALLY_COMPLETED,
    CANCELLING,
    CANCELLED,
    REJECTED,
    FAILED,
    TIMEOUT,
    BLOCKED,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    pub id: String,
    pub asset_id: String,

    pub tx_hash: String,
    pub status: TransactionStatus,
    pub sub_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMessageData {
    pub messages: Vec<UnsignedMessage>,
    // algorithm todo
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsignedMessage {
    pub content: Vec<u8>,
    // rest of bip derivation paths todo
}

// pub struct TransactionResponse {
//     id: string;
//     assetId: string;
//     source: {
//         id: string;
//         type: PeerType;
//         name?: string;
//         subType?: string;
//     };
//     destination: {
//         id: string;
//         type: PeerType;
//         name?: string;
//         subType?: string;
//     };
//     amount: number;
//     /**
//      * @deprecated Replaced by "networkFee"
//      */
//     fee?: number;
//     networkFee: number;
//     amountUSD: number;
//     netAmount: number;
//     createdAt: number;
//     lastUpdated: number;
//     status: TransactionStatus;
//     txHash: string;
//     numOfConfirmations?: number;
//     subStatus?: string;
//     signedBy: string[];
//     createdBy: string;
//     rejectedBy: string;
//     destinationAddress: string;
//     destinationAddressDescription?: string;
//     destinationTag: string;
//     addressType: string;
//     note: string;
//     exchangeTxId: string;
//     requestedAmount: number;
//     serviceFee?: number;
//     feeCurrency: string;
//     amlScreeningResult?: {
//         provider?: string;
//         payload: any;
//         screeningStatus: string;
//         bypassReason: string;
//         timestamp: number;
//     };
//     signedMessages?: SignedMessageResponse[];
// }
