use crate::{
    block::Block,
    ledger::{Ledger, LedgerError},
};
use std::collections::HashSet;

/// Validates proof-of-work, the genesis block, block indexes,
/// and links between consecutive blocks.
pub fn is_chain_valid(chain: &[Block], difficulty: usize) -> bool {
    if chain.is_empty() || difficulty > 64 {
        return false;
    }

    for (index, block) in chain.iter().enumerate() {
        if !block.is_valid(difficulty) {
            return false;
        }

        if index == 0 {
            if block.index != 0 || block.previous_hash != "0".repeat(64) {
                return false;
            }
        } else {
            let previous = &chain[index - 1];

            if previous.index.checked_add(1) != Some(block.index)
                || block.previous_hash != previous.hash
            {
                return false;
            }
        }
    }

    true
}

/// Validates the chain structure and replays every transaction in order.
///
/// The supplied ledger represents the trusted initial account state.
/// A failed transaction invalidates the chain.
pub fn validate_chain_state(
    chain: &[Block],
    difficulty: usize,
    initial_ledger: &Ledger,
) -> Result<Ledger, ChainStateError> {
    if !is_chain_valid(chain, difficulty) {
        return Err(ChainStateError::InvalidStructure);
    }

    let mut ledger = initial_ledger.clone();
    let mut seen_transaction_ids = HashSet::new();

    for block in chain {
        for transaction in &block.transactions {
            let transaction_id = transaction.transaction_id();

            if !seen_transaction_ids.insert(transaction_id) {
                return Err(ChainStateError::DuplicateTransaction);
            }

            ledger
                .apply_transaction(transaction)
                .map_err(ChainStateError::InvalidTransaction)?;
        }
    }

    Ok(ledger)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainStateError {
    InvalidStructure,
    DuplicateTransaction,
    InvalidTransaction(LedgerError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{transaction::Transaction, wallet::Wallet};

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

    #[test]
    fn valid_chain_is_accepted() {
        assert!(is_chain_valid(&two_block_chain(), 2));
    }

    #[test]
    fn broken_previous_hash_is_rejected() {
        let mut chain = two_block_chain();
        chain[1].previous_hash = "incorrect-link".into();

        assert!(!is_chain_valid(&chain, 2));
    }

    #[test]
    fn empty_chain_is_rejected() {
        assert!(!is_chain_valid(&[], 2));
    }

    #[test]
    fn incorrect_genesis_index_is_rejected() {
        let mut chain = two_block_chain();
        chain[0].index = 1;

        assert!(!is_chain_valid(&chain, 2));
    }

    #[test]
    fn empty_transaction_blocks_replay_successfully() {
        let chain = two_block_chain();
        let initial_ledger = Ledger::new();

        assert!(validate_chain_state(&chain, 2, &initial_ledger).is_ok());
    }

    #[test]
    fn valid_funded_transfer_replays_successfully() {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();

        let transaction = signed_transaction(&sender, &recipient, 30, 0);

        let mut chain = two_block_chain();
        chain[1] = Block::mine(1, 1001, chain[0].hash.clone(), vec![transaction], 2);

        let initial_ledger = Ledger::from_genesis_allocations([(sender.address(), 100)]);

        let result = validate_chain_state(&chain, 2, &initial_ledger).unwrap();

        assert_eq!(result.account(&sender.address()).balance, 70);
        assert_eq!(result.account(&recipient.address()).balance, 30);
    }

    #[test]
    fn unfunded_transfer_invalidates_chain_state() {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();

        let transaction = signed_transaction(&sender, &recipient, 30, 0);

        let mut chain = two_block_chain();
        chain[1] = Block::mine(1, 1001, chain[0].hash.clone(), vec![transaction], 2);

        let result = validate_chain_state(&chain, 2, &Ledger::new());

        assert_eq!(
            result,
            Err(ChainStateError::InvalidTransaction(
                LedgerError::InsufficientFunds
            ))
        );
    }

    #[test]
    fn duplicate_transaction_id_invalidates_chain_state() {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();

        let transaction = signed_transaction(&sender, &recipient, 30, 0);

        let mut chain = two_block_chain();
        chain[1] = Block::mine(
            1,
            1001,
            chain[0].hash.clone(),
            vec![transaction.clone(), transaction],
            2,
        );

        let initial_ledger = Ledger::from_genesis_allocations([(sender.address(), 100)]);

        assert_eq!(
            validate_chain_state(&chain, 2, &initial_ledger),
            Err(ChainStateError::DuplicateTransaction)
        );
    }
}
