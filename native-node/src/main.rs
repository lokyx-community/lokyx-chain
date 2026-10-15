mod block;
mod chain;
mod ledger;
mod transaction;
mod wallet;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use block::Block;
use chain::validate_chain_state;
use ledger::{Ledger, LedgerError};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use transaction::{NETWORK_ID, Transaction};

const DIFFICULTY: usize = 3;
const CHAIN_FILE: &str = "chain.json";

fn save_chain(chain: &[Block], path: &str) -> Result<(), String> {
    let json = serde_json::to_string_pretty(chain).map_err(|error| error.to_string())?;

    fs::write(path, json).map_err(|error| error.to_string())
}

fn load_chain(path: &str) -> Result<Vec<Block>, String> {
    let json = fs::read_to_string(path).map_err(|error| error.to_string())?;

    serde_json::from_str(&json).map_err(|error| {
        format!(
            "Could not parse the saved chain using the current block format. \
             The file may use the previous format. Preserve it and migrate it \
             deliberately. Details: {error}"
        )
    })
}

/// Validates block structure and replays transactions against
/// the current protocol's initial ledger.
///
/// Genesis allocations have not been configured, so this ledger
/// is deliberately empty.
fn is_chain_state_valid(chain: &[Block]) -> bool {
    validate_chain_state(chain, DIFFICULTY, &Ledger::new()).is_ok()
}

#[derive(Debug)]
enum PendingValidationError {
    InvalidChain,
    DuplicateTransaction,
    Ledger(LedgerError),
}

/// Reconstructs the ledger from the chain, replays pending transactions,
/// and validates the candidate against the resulting account state.
///
/// This function does not modify the chain or pending transaction list.
fn validate_pending_transaction(
    chain: &[Block],
    pending: &[Transaction],
    candidate: &Transaction,
) -> Result<(), PendingValidationError> {
    let mut ledger = validate_chain_state(chain, DIFFICULTY, &Ledger::new())
        .map_err(|_| PendingValidationError::InvalidChain)?;

    let mut seen_ids = HashSet::new();

    // Record all on-chain transaction IDs first.
    for block in chain {
        for transaction in &block.transactions {
            seen_ids.insert(transaction.transaction_id());
        }
    }

    // Check every pending ID before replaying any transaction. This makes
    // duplicate detection deterministic even if a pending transaction would
    // independently fail ledger validation (for example, for insufficient funds).
    for transaction in pending {
        if !seen_ids.insert(transaction.transaction_id()) {
            return Err(PendingValidationError::DuplicateTransaction);
        }
    }

    let candidate_id = candidate.transaction_id();
    if seen_ids.contains(&candidate_id) {
        return Err(PendingValidationError::DuplicateTransaction);
    }

    // Replay pending transactions in order so later transactions see the
    // balances and nonces produced by earlier transactions.
    for transaction in pending {
        ledger
            .apply_transaction(transaction)
            .map_err(PendingValidationError::Ledger)?;
    }

    // Validate against the state after all existing pending transactions.
    ledger
        .apply_transaction(candidate)
        .map_err(PendingValidationError::Ledger)?;

    Ok(())
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
        network_id: NETWORK_ID,
        height: state.chain.len(),
        chain_valid: is_chain_state_valid(&state.chain),
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

    let transaction_id = transaction.transaction_id();

    let mut pending = state.pending_transactions.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Pending transaction storage unavailable"
            })),
        )
    })?;

    match validate_pending_transaction(&state.chain, &pending, &transaction) {
        Ok(()) => {}
        Err(PendingValidationError::InvalidChain) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "The current chain failed state validation"
                })),
            ));
        }
        Err(PendingValidationError::DuplicateTransaction) => {
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Transaction already exists on-chain or is pending",
                    "transaction_id": transaction_id
                })),
            ));
        }
        Err(PendingValidationError::Ledger(error)) => {
            let status = match error {
                LedgerError::InvalidTransaction => StatusCode::BAD_REQUEST,
                LedgerError::IncorrectNonce { .. }
                | LedgerError::InsufficientFunds
                | LedgerError::BalanceOverflow
                | LedgerError::NonceOverflow => StatusCode::CONFLICT,
            };

            return Err((
                status,
                Json(serde_json::json!({
                    "error": format!("Transaction rejected: {error:?}"),
                    "transaction_id": transaction_id
                })),
            ));
        }
    }

    pending.push(transaction);

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "message": "Transaction accepted into the local pending pool",
            "transaction_id": transaction_id
        })),
    ))
}

fn create_initial_chain() -> Result<Vec<Block>, String> {
    let genesis = Block::mine(0, 1_735_689_600_000, "0".repeat(64), Vec::new(), DIFFICULTY);

    let next = Block::mine(
        1,
        1_735_689_600_001,
        genesis.hash.clone(),
        Vec::new(),
        DIFFICULTY,
    );

    let chain = vec![genesis, next];

    if !is_chain_state_valid(&chain) {
        return Err("New chain failed structural or state validation.".into());
    }

    Ok(chain)
}

#[tokio::main]
async fn main() {
    let chain = if Path::new(CHAIN_FILE).exists() {
        match load_chain(CHAIN_FILE) {
            Ok(chain) if is_chain_state_valid(&chain) => chain,
            Ok(_) => {
                eprintln!(
                    "Saved chain failed structural or ledger-state validation. \
                     Stopping without modifying it."
                );
                return;
            }
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        }
    } else {
        let chain = match create_initial_chain() {
            Ok(chain) => chain,
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        };

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
    use crate::{chain::is_chain_valid, wallet::Wallet};

    fn two_block_chain() -> Vec<Block> {
        let first = Block::mine(0, 1000, "0".repeat(64), Vec::new(), 2);

        let second = Block::mine(1, 1001, first.hash.clone(), Vec::new(), 2);

        vec![first, second]
    }

    fn signed_transaction(
        sender: &Wallet,
        recipient: &Wallet,
        amount: u64,
        nonce: u64,
    ) -> Transaction {
        let sender_address = sender.address();
        let recipient_address = recipient.address();

        let message =
            Transaction::signing_message(&sender_address, &recipient_address, amount, nonce);

        Transaction {
            sender: sender_address,
            recipient: recipient_address,
            amount,
            nonce,
            public_key: sender.public_key(),
            signature: sender.sign(&message),
        }
    }

    fn signed_unfunded_transaction(amount: u64) -> Transaction {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();
        signed_transaction(&sender, &recipient, amount, 0)
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
        let path =
            std::env::temp_dir().join(format!("lokyx-storage-test-{}.json", std::process::id()));
        let path = path.to_string_lossy().to_string();

        save_chain(&chain, &path).unwrap();
        let loaded = load_chain(&path).unwrap();

        assert_eq!(loaded, chain);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn api_transaction_validation_accepts_valid_signature() {
        assert!(signed_unfunded_transaction(25).is_valid());
    }

    #[test]
    fn api_transaction_validation_rejects_modified_amount() {
        let mut transaction = signed_unfunded_transaction(25);
        transaction.amount = 100;

        assert!(!transaction.is_valid());
    }

    #[test]
    fn api_transaction_validation_rejects_zero_amount() {
        assert!(!signed_unfunded_transaction(0).is_valid());
    }

    #[test]
    fn initial_chain_contains_transaction_based_blocks() {
        let chain = create_initial_chain().unwrap();

        assert_eq!(chain.len(), 2);
        assert!(chain.iter().all(|block| block.transactions.is_empty()));
        assert!(is_chain_valid(&chain, DIFFICULTY));
        assert!(is_chain_state_valid(&chain));
    }

    #[test]
    fn chain_state_validation_accepts_empty_initial_chain() {
        let chain = create_initial_chain().unwrap();

        assert!(is_chain_state_valid(&chain));
    }

    #[test]
    fn pending_validation_rejects_unfunded_transaction() {
        let chain = create_initial_chain().unwrap();
        let transaction = signed_unfunded_transaction(25);

        assert!(matches!(
            validate_pending_transaction(&chain, &[], &transaction),
            Err(PendingValidationError::Ledger(
                LedgerError::InsufficientFunds
            ))
        ));
    }

    #[test]
    fn pending_validation_rejects_duplicate_pending_transaction() {
        let chain = create_initial_chain().unwrap();
        let transaction = signed_unfunded_transaction(25);

        assert!(matches!(
            validate_pending_transaction(&chain, &[transaction.clone()], &transaction),
            Err(PendingValidationError::DuplicateTransaction)
        ));
    }
}
