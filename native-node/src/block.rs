use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub previous_hash: String,
    pub data: String,
    pub nonce: u64,
    pub hash: String,
}

impl Block {
    pub fn calculate_hash(
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

    pub fn mine(
        index: u64,
        timestamp: u64,
        previous_hash: String,
        data: String,
        difficulty: usize,
    ) -> Self {
        assert!(difficulty <= 64, "Difficulty must not exceed 64");

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

    pub fn is_valid(&self, difficulty: usize) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn excessive_difficulty_is_rejected_by_validation() {
        let block = Block::mine(0, 1000, "0".repeat(64), "test".into(), 2);
        assert!(!block.is_valid(65));
    }
}
