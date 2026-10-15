use crate::block::Block;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn two_block_chain() -> Vec<Block> {
        let first = Block::mine(0, 1000, "0".repeat(64), "genesis test".into(), 2);
        let second = Block::mine(1, 1001, first.hash.clone(), "second test".into(), 2);

        vec![first, second]
    }

    #[test]
    fn valid_chain_is_accepted() {
        let chain = two_block_chain();
        assert!(is_chain_valid(&chain, 2));
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
}
