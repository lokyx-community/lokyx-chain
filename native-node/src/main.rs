
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

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
            let hash = Self::calculate_hash(
                index,
                timestamp,
                &previous_hash,
                &data,
                nonce,
            );

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

        self.hash == expected
            && self.hash.starts_with(&"0".repeat(difficulty))
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
            if block.index != 0
                || block.previous_hash != "0".repeat(64)
            {
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
    let json = serde_json::to_string_pretty(chain)
        .map_err(|error| error.to_string())?;

    fs::write(path, json).map_err(|error| error.to_string())
}

fn load_chain(path: &str) -> Result<Vec<Block>, String> {
    let json = fs::read_to_string(path).map_err(|error| error.to_string())?;

    serde_json::from_str(&json).map_err(|error| error.to_string())
}

fn main() {
    let chain = if Path::new(CHAIN_FILE).exists() {
        match load_chain(CHAIN_FILE) {
            Ok(chain) if is_chain_valid(&chain, DIFFICULTY) => {
                println!("Loaded {} blocks from disk.", chain.len());
                chain
            }
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
            "LOKYX genesis prototype".to_string(),
            DIFFICULTY,
        );

        let next = Block::mine(
            1,
            1_735_689_600_001,
            genesis.hash.clone(),
            "First LOKYX test block".to_string(),
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

        println!("Created and saved {} blocks.", chain.len());
        chain
    };

    println!("LOKYX Rust storage prototype");

    for block in &chain {
        println!(
            "Block {} | hash: {} | valid: {}",
            block.index,
            block.hash,
            block.is_valid(DIFFICULTY)
        );
    }

    println!("Entire chain valid: {}", is_chain_valid(&chain, DIFFICULTY));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_block_chain() -> Vec<Block> {
        let first = Block::mine(
            0,
            1000,
            "0".repeat(64),
            "genesis test".into(),
            2,
        );

        let second = Block::mine(
            1,
            1001,
            first.hash.clone(),
            "second test".into(),
            2,
        );

        vec![first, second]
    }

    #[test]
    fn mined_block_has_valid_hash_and_proof_of_work() {
        let block = Block::mine(0, 1000, "0".repeat(64), "test".into(), 2);
        assert!(block.is_valid(2));
    }

    #[test]
    fn changed_data_invalidates_block() {
        let mut block =
            Block::mine(0, 1000, "0".repeat(64), "original".into(), 2);

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
        let path = std::env::temp_dir()
            .join("lokyx-storage-test.json");
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
}
