use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub previous_hash: String,
    pub transactions: Vec<Transaction>,
    pub nonce: u64,
    pub hash: String,
}

impl Block {
    /// Calculates a deterministic hash over the complete block contents.
    pub fn calculate_hash(
        index: u64,
        timestamp: u64,
        previous_hash: &str,
        transactions: &[Transaction],
        nonce: u64,
    ) -> String {
        let mut hasher = Sha256::new();

        hasher.update(b"LOKYX-BLOCK-V2");
        hasher.update(index.to_be_bytes());
        hasher.update(timestamp.to_be_bytes());

        Self::hash_bytes(&mut hasher, previous_hash.as_bytes());

        let transaction_count =
            u64::try_from(transactions.len()).expect("transaction count must fit in a u64");
        hasher.update(transaction_count.to_be_bytes());

        for transaction in transactions {
            Self::hash_bytes(&mut hasher, transaction.sender.as_bytes());
            Self::hash_bytes(&mut hasher, transaction.recipient.as_bytes());
            hasher.update(transaction.amount.to_be_bytes());
            hasher.update(transaction.nonce.to_be_bytes());
            hasher.update(transaction.public_key);
            Self::hash_bytes(&mut hasher, &transaction.signature);
        }

        hasher.update(nonce.to_be_bytes());

        let digest = hasher.finalize();
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
        let length = u64::try_from(bytes.len()).expect("field length must fit in a u64");

        hasher.update(length.to_be_bytes());
        hasher.update(bytes);
    }

    /// Mines a block containing an ordered list of transactions.
    pub fn mine(
        index: u64,
        timestamp: u64,
        previous_hash: String,
        transactions: Vec<Transaction>,
        difficulty: usize,
    ) -> Self {
        assert!(difficulty <= 64, "Difficulty must not exceed 64");

        let target = "0".repeat(difficulty);
        let mut nonce = 0_u64;

        loop {
            let hash = Self::calculate_hash(index, timestamp, &previous_hash, &transactions, nonce);

            if hash.starts_with(&target) {
                return Self {
                    index,
                    timestamp,
                    previous_hash,
                    transactions,
                    nonce,
                    hash,
                };
            }

            nonce = nonce.checked_add(1).expect("Nonce space exhausted");
        }
    }

    /// Checks the stored hash and proof-of-work target.
    pub fn is_valid(&self, difficulty: usize) -> bool {
        if difficulty > 64 {
            return false;
        }

        let expected = Self::calculate_hash(
            self.index,
            self.timestamp,
            &self.previous_hash,
            &self.transactions,
            self.nonce,
        );

        self.hash == expected && self.hash.starts_with(&"0".repeat(difficulty))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mined_block_has_valid_hash_and_proof_of_work() {
        let block = Block::mine(0, 1000, "0".repeat(64), Vec::new(), 2);

        assert!(block.is_valid(2));
    }

    #[test]
    fn changing_transactions_invalidates_block() {
        let mut block = Block::mine(0, 1000, "0".repeat(64), Vec::new(), 2);

        block.transactions.push(Transaction {
            sender: "sender".into(),
            recipient: "recipient".into(),
            amount: 10,
            nonce: 0,
            public_key: [0; 32],
            signature: Vec::new(),
        });

        assert!(!block.is_valid(2));
    }

    #[test]
    fn excessive_difficulty_is_rejected() {
        let block = Block::mine(0, 1000, "0".repeat(64), Vec::new(), 2);

        assert!(!block.is_valid(65));
    }
}
