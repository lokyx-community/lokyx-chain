mod transaction;
mod wallet;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use transaction::Transaction;

const DIFFICULTY: usize = 3;
const CHAIN_FILE: &str = "chain.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Block {
    index: u64,
    timestamp: u64,
    previous_hash: String,
    data: String,
    nonce: u64,
    hash: String,
}

impl Block {
    fn calculate_hash(
        index: u64,
        timestamp: u64,
        previous_hash: &str,
        data: &str,
        nonce: u64,
    ) -> String {
        let input = format!(
            "{}|{}|{}|{}|{}",
            index, timestamp, previous_hash, data, nonce
        );

        let digest = Sha256::digest(input.as_bytes());
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn mine(
        index: u64,
        timestamp: u64,
        previous_hash: String,
        data: String,
        difficulty: usize,
    ) -> Self {
        assert!(difficulty <= 64);

        let target = "0".repeat(difficulty);
        let mut nonce = 0_u64;

        loop {
            let hash = Self::calculate_hash(index, timestamp, &previous_hash, &data, nonce);

            if hash.starts_with(&target) {
                return Self {
                    index,
                    timestamp,
                    previous_hash,
                    data,
                    nonce,
                    hash,
                };
            }

            nonce = nonce.checked_add(1).expect("Nonce space exhausted");
        }
    }

    fn is_valid(&self, difficulty: usize) -> bool {
        if difficulty > 64 {
            return false;
        }

        let expected = Self::calculate_hash(
            self.index,
            self.timestamp,
            &self.previous_hash,
            &self.data,
            self.nonce,
        );

        self.hash == expected && self.hash.starts_with(&"0".repeat(difficulty))
    }
}

fn is_chain_valid(chain: &[Block], difficulty: usize) -> bool {
    if chain.is_empty() || difficulty > 64 {
        return false;
    }

    for (i, block) in chain.iter().enumerate() {
        if !block.is_valid(difficulty) {
            return false;
        }

        if i == 0 {
            if block.index != 0 || block.previous_hash != "0".repeat(64) {
                return false;
            }
        } else {
            let previous = &chain[i - 1];

            if previous.index.checked_add(1) != Some(block.index)
                || block.previous_hash != previous.hash
            {
                return false;
            }
        }
    }

    true
}

fn save_chain(chain: &[Block], path: &str) -> Result<(), String> {
    let json = serde_json::to_string_pretty(chain).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn load_chain(path: &str) -> Result<Vec<Block>, String> {
    let json = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&json).map_err(|error| error.to_string())
}

#[derive(Clone)]
struct AppState {
    chain: Arc<Vec<Block>>,
    pending_transactions: Arc<Mutex<Vec<Transaction>>>,
}

#[derive(Serialize)]
struct NodeStatus {
    name: &'static str,
    network_id: &'static str,
    height: usize,
    chain_valid: bool,
    pending_transactions: usize,
}

async fn status(State(state): State<AppState>) -> Json<NodeStatus> {
    let pending_count = match state.pending_transactions.lock() {
        Ok(pending) => pending.len(),
        Err(_) => 0,
    };

    Json(NodeStatus {
        name: "LOKYX Rust Node",
        network_id: "lokyx-testnet-1",
        height: state.chain.len(),
        chain_valid: is_chain_valid(&state.chain, DIFFICULTY),
        pending_transactions: pending_count,
    })
}

async fn get_chain(State(state): State<AppState>) -> Json<Vec<Block>> {
    Json((*state.chain).clone())
}

async fn get_pending_transactions(
    State(state): State<AppState>,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    let pending = state
        .pending_transactions
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(pending.clone()))
}

async fn submit_transaction(
    State(state): State<AppState>,
    Json(transaction): Json<Transaction>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    if !transaction.is_valid() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid transaction, address, or signature"
            })),
        ));
    }

    let tx_id = transaction.transaction_id();

    let mut pending = state.pending_transactions.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Pending transaction storage unavailable"
            })),
        )
    })?;

    if pending
        .iter()
        .any(|existing| existing.transaction_id() == tx_id)
    {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "Transaction already pending",
                "transaction_id": tx_id
            })),
        ));
    }

    pending.push(transaction);

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "message": "Transaction accepted into the local pending pool",
            "transaction_id": tx_id
        })),
    ))
}

#[tokio::main]
async fn main() {
    let chain = if Path::new(CHAIN_FILE).exists() {
        match load_chain(CHAIN_FILE) {
            Ok(chain) if is_chain_valid(&chain, DIFFICULTY) => chain,
            Ok(_) => {
                eprintln!("Saved chain is invalid. Stopping.");
                return;
            }
            Err(error) => {
                eprintln!("Could not load saved chain: {error}");
                return;
            }
        }
    } else {
        let genesis = Block::mine(
            0,
            1_735_689_600_000,
            "0".repeat(64),
            "LOKYX genesis prototype".into(),
            DIFFICULTY,
        );

        let next = Block::mine(
            1,
            1_735_689_600_001,
            genesis.hash.clone(),
            "First LOKYX test block".into(),
            DIFFICULTY,
        );

        let chain = vec![genesis, next];

        if !is_chain_valid(&chain, DIFFICULTY) {
            eprintln!("New chain failed validation.");
            return;
        }

        if let Err(error) = save_chain(&chain, CHAIN_FILE) {
            eprintln!("Could not save chain: {error}");
            return;
        }

        chain
    };

    let state = AppState {
        chain: Arc::new(chain),
        pending_transactions: Arc::new(Mutex::new(Vec::new())),
    };

    let app = Router::new()
        .route("/status", get(status))
        .route("/chain", get(get_chain))
        .route("/transactions", post(submit_transaction))
        .route("/transactions/pending", get(get_pending_transactions))
        .with_state(state);

    let port: u16 = std::env::var("LOKYX_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(7070);

    let address = SocketAddr::from(([127, 0, 0, 1], port));

    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Could not bind to {address}: {error}");
            return;
        }
    };

    println!("LOKYX Rust node listening at http://{address}");
    println!("Status:       http://{address}/status");
    println!("Chain:        http://{address}/chain");
    println!("Submit:       POST http://{address}/transactions");
    println!("Pending:      http://{address}/transactions/pending");

    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("Node server failed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    fn two_block_chain() -> Vec<Block> {
        let first = Block::mine(0, 1000, "0".repeat(64), "genesis test".into(), 2);
        let second = Block::mine(1, 1001, first.hash.clone(), "second test".into(), 2);
        vec![first, second]
    }

    fn signed_transaction(amount: u64) -> Transaction {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();
        let sender_address = sender.address();
        let recipient_address = recipient.address();

        let message = Transaction::signing_message(&sender_address, &recipient_address, amount, 0);

        Transaction {
            sender: sender_address,
            recipient: recipient_address,
            amount,
            nonce: 0,
            public_key: sender.public_key(),
            signature: sender.sign(&message),
        }
    }

    #[test]
    fn mined_block_has_valid_hash_and_proof_of_work() {
        let block = Block::mine(0, 1000, "0".repeat(64), "test".into(), 2);
        assert!(block.is_valid(2));
    }

    #[test]
    fn changed_data_invalidates_block() {
        let mut block = Block::mine(0, 1000, "0".repeat(64), "original".into(), 2);
        block.data = "modified".into();
        assert!(!block.is_valid(2));
    }

    #[test]
    fn next_block_links_to_previous_hash() {
        let chain = two_block_chain();
        assert_eq!(chain[1].previous_hash, chain[0].hash);
        assert!(is_chain_valid(&chain, 2));
    }

    #[test]
    fn chain_can_be_saved_and_loaded() {
        let chain = two_block_chain();
        let path = std::env::temp_dir().join("lokyx-storage-test.json");
        let path = path.to_string_lossy().to_string();

        save_chain(&chain, &path).unwrap();
        let loaded = load_chain(&path).unwrap();

        assert_eq!(loaded, chain);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn broken_chain_is_rejected() {
        let mut chain = two_block_chain();
        chain[1].previous_hash = "incorrect-link".into();
        assert!(!is_chain_valid(&chain, 2));
    }

    #[test]
    fn empty_chain_is_rejected() {
        assert!(!is_chain_valid(&[], 2));
    }

    #[test]
    fn api_transaction_validation_accepts_valid_signature() {
        let transaction = signed_transaction(25);
        assert!(transaction.is_valid());
    }

    #[test]
    fn api_transaction_validation_rejects_modified_amount() {
        let mut transaction = signed_transaction(25);
        transaction.amount = 100;
        assert!(!transaction.is_valid());
    }

    #[test]
    fn api_transaction_validation_rejects_zero_amount() {
        let transaction = signed_transaction(0);
        assert!(!transaction.is_valid());
    }
}
