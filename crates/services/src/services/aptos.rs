use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use ts_rs::TS;

const TESTNET_NODE_URL: &str = "https://fullnode.testnet.aptoslabs.com/v1";
const TESTNET_FAUCET_URL: &str = "https://faucet.testnet.aptoslabs.com";
const GAS_UNIT_PRICE: u64 = 100;
const MAX_GAS_AMOUNT: u64 = 10000;

// VIBE Token Configuration
// VIBE is a fungible asset on Aptos - address will be set after deployment
// For devnet testing, we simulate VIBE using a conversion from APT
// 1 VIBE = $0.001 USD, 1 APT ≈ $10 USD (approximate), so 1 APT = 10,000 VIBE
const VIBE_TOKEN_ADDRESS: &str = "0x0"; // Placeholder - will be updated after VIBE contract deployment
const APT_TO_VIBE_RATE: u64 = 10_000; // 1 APT = 10,000 VIBE (assuming $10/APT and $0.001/VIBE)
const VIBE_DECIMALS: u8 = 8; // Same as APT for simplicity

#[derive(Debug, Clone)]
pub struct AptosService {
    client: Client,
    node_url: String,
    faucet_url: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AptosBalance {
    pub address: String,
    pub balance: u64,
    pub balance_apt: f64,
    pub sequence_number: u64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AptosTransaction {
    pub version: String,
    pub hash: String,
    pub sender: String,
    pub sequence_number: String,
    pub timestamp: String,
    pub tx_type: String,
    pub success: bool,
    pub gas_used: String,
    pub gas_unit_price: String,
    pub payload_function: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FaucetResponse {
    pub success: bool,
    pub message: String,
    pub tx_hashes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SendTransactionRequest {
    pub sender_private_key: String,
    pub sender_address: String,
    pub recipient_address: String,
    pub amount_apt: f64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SendTransactionResponse {
    pub success: bool,
    pub tx_hash: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EstimateGasResponse {
    pub gas_estimate: u64,
    pub gas_unit_price: u64,
    pub total_gas_apt: f64,
}

// VIBE Token Types
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VibeBalance {
    pub address: String,
    /// Balance in VIBE tokens (smallest unit)
    pub balance: u64,
    /// Balance in VIBE tokens (human readable)
    pub balance_vibe: f64,
    /// Equivalent APT value (for reference)
    pub equivalent_apt: f64,
    /// USD value (1 VIBE = $0.001)
    pub usd_value: f64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VibeTransferRequest {
    pub sender_private_key: String,
    pub sender_address: String,
    pub recipient_address: String,
    pub amount_vibe: u64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VibeTransferResponse {
    pub success: bool,
    pub tx_hash: String,
    pub amount_vibe: u64,
    pub message: String,
}

/// Result of generating a new Aptos wallet
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GeneratedWallet {
    /// The Aptos address (0x prefixed hex)
    pub address: String,
    /// The encrypted private key (base64 encoded nonce + ciphertext)
    pub private_key_encrypted: String,
    /// The public key (hex encoded, for verification)
    pub public_key: String,
}

/// Request to setup on-chain wallet for an agent
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SetupOnchainWalletRequest {
    pub wallet_id: String,
    /// Whether to automatically fund from testnet faucet
    #[ts(optional)]
    pub auto_fund: Option<bool>,
}

/// Response from setting up on-chain wallet
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SetupOnchainWalletResponse {
    pub success: bool,
    pub aptos_address: String,
    pub funded: bool,
    pub message: String,
}

// Internal API response types
#[derive(Debug, Deserialize)]
struct AccountResource {
    #[serde(rename = "type")]
    resource_type: String,
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct CoinStoreData {
    coin: CoinValue,
}

#[derive(Debug, Deserialize)]
struct CoinValue {
    value: String,
}

#[derive(Debug, Deserialize)]
struct AccountData {
    sequence_number: String,
}

#[derive(Debug, Deserialize)]
struct TransactionResponse {
    version: String,
    hash: String,
    sender: Option<String>,
    sequence_number: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "type")]
    tx_type: String,
    success: Option<bool>,
    gas_used: Option<String>,
    gas_unit_price: Option<String>,
    payload: Option<TransactionPayload>,
}

#[derive(Debug, Deserialize)]
struct TransactionPayload {
    function: Option<String>,
}

// For building transactions
#[derive(Debug, Serialize)]
struct TransactionPayloadRequest {
    #[serde(rename = "type")]
    payload_type: String,
    function: String,
    type_arguments: Vec<String>,
    arguments: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct TransactionRequest {
    sender: String,
    sequence_number: String,
    max_gas_amount: String,
    gas_unit_price: String,
    expiration_timestamp_secs: String,
    payload: TransactionPayloadRequest,
}

#[derive(Debug, Serialize)]
struct SignedTransactionRequest {
    #[serde(flatten)]
    transaction: TransactionRequest,
    signature: TransactionSignature,
}

#[derive(Debug, Serialize)]
struct TransactionSignature {
    #[serde(rename = "type")]
    sig_type: String,
    public_key: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct SubmitTransactionResponse {
    hash: String,
}

#[derive(Debug, Deserialize)]
struct LedgerInfo {
    ledger_timestamp: String,
}

impl AptosService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            node_url: TESTNET_NODE_URL.to_string(),
            faucet_url: TESTNET_FAUCET_URL.to_string(),
        }
    }

    pub fn testnet() -> Self {
        Self::new()
    }

    /// Get account balance and info from testnet
    /// Supports both legacy CoinStore and new Primary Fungible Store
    pub async fn get_balance(&self, address: &str) -> Result<AptosBalance> {
        // Normalize address (ensure 0x prefix and proper length)
        let address = Self::normalize_address(address);

        // Get account resources
        let url = format!("{}/accounts/{}/resources", self.node_url, address);
        let response = self.client.get(&url).send().await?;

        if response.status() == 404 {
            // Account doesn't exist on chain yet
            return Ok(AptosBalance {
                address: address.clone(),
                balance: 0,
                balance_apt: 0.0,
                sequence_number: 0,
            });
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get account resources: {}", error_text));
        }

        let resources: Vec<AccountResource> = response.json().await?;

        // Find APT coin balance from legacy CoinStore
        let mut balance: u64 = 0;
        let mut sequence_number: u64 = 0;

        for resource in &resources {
            if resource.resource_type == "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>" {
                if let Ok(coin_store) = serde_json::from_value::<CoinStoreData>(resource.data.clone()) {
                    balance = coin_store.coin.value.parse().unwrap_or(0);
                }
            }
            if resource.resource_type == "0x1::account::Account" {
                if let Ok(account_data) = serde_json::from_value::<AccountData>(resource.data.clone()) {
                    sequence_number = account_data.sequence_number.parse().unwrap_or(0);
                }
            }
        }

        // If no legacy CoinStore balance, check Primary Fungible Store (new Aptos standard)
        // APT metadata is at address 0xa
        if balance == 0 {
            if let Ok(fa_balance) = self.get_fungible_asset_balance(&address, "0xa").await {
                balance = fa_balance;
            }
        }

        // Convert octas to APT (1 APT = 10^8 octas)
        let balance_apt = balance as f64 / 100_000_000.0;

        Ok(AptosBalance {
            address,
            balance,
            balance_apt,
            sequence_number,
        })
    }

    /// Get fungible asset balance from Primary Fungible Store
    /// Uses the view function to query balance for any fungible asset
    async fn get_fungible_asset_balance(&self, owner: &str, metadata_address: &str) -> Result<u64> {
        let url = format!("{}/view", self.node_url);

        let request_body = serde_json::json!({
            "function": "0x1::primary_fungible_store::balance",
            "type_arguments": ["0x1::fungible_asset::Metadata"],
            "arguments": [owner, metadata_address]
        });

        let response = self.client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(0); // Return 0 if view fails (account might not have FA store)
        }

        // Response is an array with single u64 value as string
        let result: Vec<String> = response.json().await.unwrap_or_default();
        let balance = result.first()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(balance)
    }

    /// Fund account from testnet faucet
    pub async fn fund_from_faucet(&self, address: &str, amount_apt: Option<f64>) -> Result<FaucetResponse> {
        let address = Self::normalize_address(address);

        // Default to 1 APT if not specified (faucet typically gives 1 APT per request)
        let amount_octas = ((amount_apt.unwrap_or(1.0)) * 100_000_000.0) as u64;

        let url = format!(
            "{}/mint?address={}&amount={}",
            self.faucet_url, address, amount_octas
        );

        let response = self.client.post(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Faucet request failed ({}): {}",
                status,
                error_text
            ));
        }

        // Faucet returns array of transaction hashes
        let tx_hashes: Vec<String> = response.json().await.unwrap_or_default();

        Ok(FaucetResponse {
            success: true,
            message: format!("Funded {} APT to {}", amount_apt.unwrap_or(1.0), address),
            tx_hashes,
        })
    }

    /// Get recent transactions for an account
    pub async fn get_transactions(&self, address: &str, limit: Option<u32>) -> Result<Vec<AptosTransaction>> {
        let address = Self::normalize_address(address);
        let limit = limit.unwrap_or(25);

        let url = format!(
            "{}/accounts/{}/transactions?limit={}",
            self.node_url, address, limit
        );

        let response = self.client.get(&url).send().await?;

        if response.status() == 404 {
            // Account doesn't exist, return empty list
            return Ok(vec![]);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get transactions: {}", error_text));
        }

        let txns: Vec<TransactionResponse> = response.json().await?;

        let transactions: Vec<AptosTransaction> = txns
            .into_iter()
            .map(|tx| AptosTransaction {
                version: tx.version,
                hash: tx.hash,
                sender: tx.sender.unwrap_or_default(),
                sequence_number: tx.sequence_number.unwrap_or_default(),
                timestamp: tx.timestamp.unwrap_or_default(),
                tx_type: tx.tx_type,
                success: tx.success.unwrap_or(false),
                gas_used: tx.gas_used.unwrap_or_default(),
                gas_unit_price: tx.gas_unit_price.unwrap_or_default(),
                payload_function: tx.payload.and_then(|p| p.function),
            })
            .collect();

        Ok(transactions)
    }

    /// Check if account exists on chain
    pub async fn account_exists(&self, address: &str) -> Result<bool> {
        let address = Self::normalize_address(address);
        let url = format!("{}/accounts/{}", self.node_url, address);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    /// Send APT to another address
    pub async fn send_apt(
        &self,
        sender_private_key: &str,
        sender_address: &str,
        recipient_address: &str,
        amount_apt: f64,
    ) -> Result<SendTransactionResponse> {
        let sender = Self::normalize_address(sender_address);
        let recipient = Self::normalize_address(recipient_address);
        let amount_octas = (amount_apt * 100_000_000.0) as u64;

        // Parse the private key
        let private_key_bytes = Self::parse_hex_key(sender_private_key)?;
        let secret_key = SecretKey::from_bytes(&private_key_bytes)
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;
        let public_key = PublicKey::from(&secret_key);
        let keypair = Keypair { secret: secret_key, public: public_key };

        // Get sequence number
        let balance_info = self.get_balance(&sender).await?;
        let sequence_number = balance_info.sequence_number;

        // Get ledger timestamp for expiration
        let ledger_info = self.get_ledger_info().await?;
        let expiration = ledger_info + 600; // 10 minutes from now

        // Build the transaction payload
        let payload = TransactionPayloadRequest {
            payload_type: "entry_function_payload".to_string(),
            function: "0x1::aptos_account::transfer".to_string(),
            type_arguments: vec![],
            arguments: vec![
                serde_json::Value::String(recipient.clone()),
                serde_json::Value::String(amount_octas.to_string()),
            ],
        };

        let transaction = TransactionRequest {
            sender: sender.clone(),
            sequence_number: sequence_number.to_string(),
            max_gas_amount: MAX_GAS_AMOUNT.to_string(),
            gas_unit_price: GAS_UNIT_PRICE.to_string(),
            expiration_timestamp_secs: expiration.to_string(),
            payload,
        };

        // Get the signing message from the API
        let encode_url = format!("{}/transactions/encode_submission", self.node_url);
        let encode_response = self.client
            .post(&encode_url)
            .json(&transaction)
            .send()
            .await?;

        if !encode_response.status().is_success() {
            let error_text = encode_response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to encode transaction: {}", error_text));
        }

        let signing_message: String = encode_response.json().await?;
        let signing_bytes = Self::parse_hex_key(&signing_message)?;

        // Sign the message
        let signature: Signature = keypair.sign(&signing_bytes);
        let signature_hex = format!("0x{}", hex::encode(signature.to_bytes()));
        let public_key_hex = format!("0x{}", hex::encode(keypair.public.to_bytes()));

        // Build signed transaction
        let signed_tx = SignedTransactionRequest {
            transaction: TransactionRequest {
                sender: sender.clone(),
                sequence_number: sequence_number.to_string(),
                max_gas_amount: MAX_GAS_AMOUNT.to_string(),
                gas_unit_price: GAS_UNIT_PRICE.to_string(),
                expiration_timestamp_secs: expiration.to_string(),
                payload: TransactionPayloadRequest {
                    payload_type: "entry_function_payload".to_string(),
                    function: "0x1::aptos_account::transfer".to_string(),
                    type_arguments: vec![],
                    arguments: vec![
                        serde_json::Value::String(recipient.clone()),
                        serde_json::Value::String(amount_octas.to_string()),
                    ],
                },
            },
            signature: TransactionSignature {
                sig_type: "ed25519_signature".to_string(),
                public_key: public_key_hex,
                signature: signature_hex,
            },
        };

        // Submit the transaction
        let submit_url = format!("{}/transactions", self.node_url);
        let submit_response = self.client
            .post(&submit_url)
            .json(&signed_tx)
            .send()
            .await?;

        if !submit_response.status().is_success() {
            let error_text = submit_response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to submit transaction: {}", error_text));
        }

        let result: SubmitTransactionResponse = submit_response.json().await?;

        Ok(SendTransactionResponse {
            success: true,
            tx_hash: result.hash,
            message: format!("Sent {} APT to {}", amount_apt, recipient),
        })
    }

    /// Estimate gas for a transfer
    pub async fn estimate_gas(&self, sender_address: &str) -> Result<EstimateGasResponse> {
        // For simple transfers, gas is fairly predictable
        let gas_estimate = 500u64; // Typical for coin transfer
        let total_octas = gas_estimate * GAS_UNIT_PRICE;
        let total_apt = total_octas as f64 / 100_000_000.0;

        Ok(EstimateGasResponse {
            gas_estimate,
            gas_unit_price: GAS_UNIT_PRICE,
            total_gas_apt: total_apt,
        })
    }

    // ========================================
    // VIBE Token Methods
    // ========================================

    // VIBE Token contract address (deployed on testnet)
    const VIBE_CONTRACT: &'static str = "0x24cb561c64c32942eb8600d5135f0185c23bcd06cd8cf33422ce2f9b77d65388";

    /// Get VIBE token balance for an address
    /// Queries the actual VIBE token contract on Aptos testnet
    pub async fn get_vibe_balance(&self, address: &str) -> Result<VibeBalance> {
        let address = Self::normalize_address(address);

        // Query the VIBE token contract balance
        let vibe_raw = self.query_vibe_balance(&address).await.unwrap_or(0);

        // VIBE has 8 decimals (same as APT)
        let vibe_balance = vibe_raw / 100_000_000; // Convert from smallest unit to whole VIBE
        let vibe_balance_human = vibe_raw as f64 / 100_000_000.0;
        let usd_value = vibe_balance_human * 0.001; // 1 VIBE = $0.001

        // Calculate equivalent APT (for reference only)
        let equivalent_apt = vibe_balance_human / APT_TO_VIBE_RATE as f64;

        Ok(VibeBalance {
            address,
            balance: vibe_balance,
            balance_vibe: vibe_balance_human,
            equivalent_apt,
            usd_value,
        })
    }

    /// Query VIBE token balance from the deployed contract
    async fn query_vibe_balance(&self, owner: &str) -> Result<u64> {
        let url = format!("{}/view", self.node_url);

        let request_body = serde_json::json!({
            "function": format!("{}::vibe_token::balance", Self::VIBE_CONTRACT),
            "type_arguments": [],
            "arguments": [owner]
        });

        let response = self.client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(0); // Return 0 if view fails
        }

        // Response is an array with single u64 value as string
        let result: Vec<String> = response.json().await.unwrap_or_default();
        let balance = result.first()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(balance)
    }

    /// Transfer VIBE tokens to another address using the VIBE token contract
    pub async fn transfer_vibe(
        &self,
        sender_private_key: &str,
        sender_address: &str,
        recipient_address: &str,
        amount_vibe: u64,
    ) -> Result<VibeTransferResponse> {
        let sender = Self::normalize_address(sender_address);
        let recipient = Self::normalize_address(recipient_address);

        // Convert whole VIBE to smallest units (8 decimals)
        let amount_raw = amount_vibe * 100_000_000;

        // Parse the private key
        let private_key_bytes = Self::parse_hex_key(sender_private_key)?;
        let secret_key = SecretKey::from_bytes(&private_key_bytes)
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;
        let public_key = PublicKey::from(&secret_key);
        let keypair = Keypair { secret: secret_key, public: public_key };

        // Get sequence number
        let balance_info = self.get_balance(&sender).await?;
        let sequence_number = balance_info.sequence_number;

        // Get ledger timestamp for expiration
        let ledger_info = self.get_ledger_info().await?;
        let expiration = ledger_info + 600; // 10 minutes from now

        // Build the transaction payload for VIBE token transfer
        let payload = TransactionPayloadRequest {
            payload_type: "entry_function_payload".to_string(),
            function: format!("{}::vibe_token::transfer", Self::VIBE_CONTRACT),
            type_arguments: vec![],
            arguments: vec![
                serde_json::Value::String(recipient.clone()),
                serde_json::Value::String(amount_raw.to_string()),
            ],
        };

        let transaction = TransactionRequest {
            sender: sender.clone(),
            sequence_number: sequence_number.to_string(),
            max_gas_amount: MAX_GAS_AMOUNT.to_string(),
            gas_unit_price: GAS_UNIT_PRICE.to_string(),
            expiration_timestamp_secs: expiration.to_string(),
            payload,
        };

        // Get the signing message from the API
        let encode_url = format!("{}/transactions/encode_submission", self.node_url);
        let encode_response = self.client
            .post(&encode_url)
            .json(&transaction)
            .send()
            .await?;

        if !encode_response.status().is_success() {
            let error_text = encode_response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to encode VIBE transfer: {}", error_text));
        }

        let signing_message: String = encode_response.json().await?;
        let signing_bytes = Self::parse_hex_key(&signing_message)?;

        // Sign the message
        let signature: Signature = keypair.sign(&signing_bytes);
        let signature_hex = format!("0x{}", hex::encode(signature.to_bytes()));
        let public_key_hex = format!("0x{}", hex::encode(keypair.public.to_bytes()));

        // Build signed transaction
        let signed_tx = SignedTransactionRequest {
            transaction: TransactionRequest {
                sender: sender.clone(),
                sequence_number: sequence_number.to_string(),
                max_gas_amount: MAX_GAS_AMOUNT.to_string(),
                gas_unit_price: GAS_UNIT_PRICE.to_string(),
                expiration_timestamp_secs: expiration.to_string(),
                payload: TransactionPayloadRequest {
                    payload_type: "entry_function_payload".to_string(),
                    function: format!("{}::vibe_token::transfer", Self::VIBE_CONTRACT),
                    type_arguments: vec![],
                    arguments: vec![
                        serde_json::Value::String(recipient.clone()),
                        serde_json::Value::String(amount_raw.to_string()),
                    ],
                },
            },
            signature: TransactionSignature {
                sig_type: "ed25519_signature".to_string(),
                public_key: public_key_hex,
                signature: signature_hex,
            },
        };

        // Submit the transaction
        let submit_url = format!("{}/transactions", self.node_url);
        let submit_response = self.client
            .post(&submit_url)
            .json(&signed_tx)
            .send()
            .await?;

        if !submit_response.status().is_success() {
            let error_text = submit_response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to submit VIBE transfer: {}", error_text));
        }

        let result: SubmitTransactionResponse = submit_response.json().await?;

        Ok(VibeTransferResponse {
            success: true,
            tx_hash: result.hash,
            amount_vibe,
            message: format!("Transferred {} VIBE to {}", amount_vibe, recipient),
        })
    }

    /// Deduct VIBE for LLM usage (simulation)
    /// This method records the deduction and returns the transaction hash
    pub async fn deduct_vibe_for_llm(
        &self,
        sender_private_key: &str,
        sender_address: &str,
        treasury_address: &str,
        amount_vibe: u64,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<VibeTransferResponse> {
        // Verify sender has enough VIBE
        let balance = self.get_vibe_balance(sender_address).await?;
        if balance.balance < amount_vibe {
            return Err(anyhow!(
                "Insufficient VIBE balance. Required: {}, Available: {}",
                amount_vibe,
                balance.balance
            ));
        }

        // Transfer VIBE to treasury
        let result = self
            .transfer_vibe(
                sender_private_key,
                sender_address,
                treasury_address,
                amount_vibe,
            )
            .await?;

        Ok(VibeTransferResponse {
            success: result.success,
            tx_hash: result.tx_hash,
            amount_vibe,
            message: format!(
                "LLM usage charge: {} VIBE for {} ({} in, {} out tokens)",
                amount_vibe, model, input_tokens, output_tokens
            ),
        })
    }

    /// Convert APT amount to VIBE tokens
    pub fn apt_to_vibe(apt: f64) -> u64 {
        (apt * APT_TO_VIBE_RATE as f64) as u64
    }

    /// Convert VIBE tokens to APT amount
    pub fn vibe_to_apt(vibe: u64) -> f64 {
        vibe as f64 / APT_TO_VIBE_RATE as f64
    }

    /// Get the VIBE to USD conversion rate
    pub fn vibe_usd_rate() -> f64 {
        0.001 // 1 VIBE = $0.001
    }

    /// Get current ledger timestamp
    async fn get_ledger_info(&self) -> Result<u64> {
        let url = format!("{}", self.node_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get ledger info"));
        }

        let info: LedgerInfo = response.json().await?;
        let timestamp: u64 = info.ledger_timestamp.parse().unwrap_or(0) / 1_000_000; // Convert from microseconds
        Ok(timestamp)
    }

    /// Parse hex-encoded key (with or without 0x prefix)
    fn parse_hex_key(key: &str) -> Result<Vec<u8>> {
        let key = key.trim();
        let hex_str = if key.starts_with("0x") {
            &key[2..]
        } else {
            key
        };
        hex::decode(hex_str).map_err(|e| anyhow!("Invalid hex key: {}", e))
    }

    /// Normalize address to proper format
    fn normalize_address(address: &str) -> String {
        let addr = address.trim();
        if addr.starts_with("0x") {
            addr.to_string()
        } else {
            format!("0x{}", addr)
        }
    }

    // ========================================
    // Wallet Generation Methods
    // ========================================

    /// Generate a new Aptos wallet with encrypted private key
    ///
    /// The private key is encrypted using AES-256-GCM with a key derived from
    /// the provided encryption secret.
    pub fn generate_wallet(encryption_secret: &str) -> Result<GeneratedWallet> {
        // Generate random bytes for the private key
        let mut rng = rand::thread_rng();
        let mut secret_bytes = [0u8; 32];
        rng.fill_bytes(&mut secret_bytes);

        // Create the keypair
        let secret_key = SecretKey::from_bytes(&secret_bytes)
            .map_err(|e| anyhow!("Failed to create secret key: {}", e))?;
        let public_key = PublicKey::from(&secret_key);

        // Derive Aptos address from public key (SHA3-256 of public key bytes)
        let address = Self::derive_address_from_public_key(&public_key);

        // Encrypt the private key
        let private_key_encrypted = Self::encrypt_private_key(&secret_bytes, encryption_secret)?;

        Ok(GeneratedWallet {
            address,
            private_key_encrypted,
            public_key: format!("0x{}", hex::encode(public_key.to_bytes())),
        })
    }

    /// Derive Aptos address from an Ed25519 public key
    ///
    /// Aptos uses SHA3-256 hash of (public_key || 0x00) where 0x00 is the
    /// signature scheme identifier for Ed25519
    fn derive_address_from_public_key(public_key: &PublicKey) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update(public_key.as_bytes());
        hasher.update(&[0x00]); // Ed25519 signature scheme identifier
        let hash = hasher.finalize();
        format!("0x{}", hex::encode(hash))
    }

    /// Encrypt a private key using AES-256-GCM
    ///
    /// Returns base64-encoded string containing: nonce (12 bytes) + ciphertext
    fn encrypt_private_key(private_key: &[u8], encryption_secret: &str) -> Result<String> {
        // Derive a 256-bit key from the secret using SHA3-256
        let mut hasher = Sha3_256::new();
        hasher.update(encryption_secret.as_bytes());
        let key_bytes = hasher.finalize();

        // Create AES-256-GCM cipher
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

        // Generate random nonce
        let mut rng = rand::thread_rng();
        let mut nonce_bytes = [0u8; 12];
        rng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, private_key)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Combine nonce + ciphertext and encode as base64
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &combined,
        ))
    }

    /// Decrypt a private key that was encrypted with encrypt_private_key
    pub fn decrypt_private_key(encrypted: &str, encryption_secret: &str) -> Result<Vec<u8>> {
        // Derive key from secret
        let mut hasher = Sha3_256::new();
        hasher.update(encryption_secret.as_bytes());
        let key_bytes = hasher.finalize();

        // Decode base64
        let combined = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encrypted)
            .map_err(|e| anyhow!("Invalid base64: {}", e))?;

        if combined.len() < 12 {
            return Err(anyhow!("Encrypted data too short"));
        }

        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Create cipher and decrypt
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))
    }

    /// Get the decrypted private key as a hex string for use in transactions
    pub fn get_private_key_hex(encrypted: &str, encryption_secret: &str) -> Result<String> {
        let decrypted = Self::decrypt_private_key(encrypted, encryption_secret)?;
        Ok(format!("0x{}", hex::encode(&decrypted)))
    }
}

impl Default for AptosService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_normalize_address() {
        let addr1 = AptosService::normalize_address("0x123");
        assert_eq!(addr1, "0x123");

        let addr2 = AptosService::normalize_address("456");
        assert_eq!(addr2, "0x456");
    }
}
