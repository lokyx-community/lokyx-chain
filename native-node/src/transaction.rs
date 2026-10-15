use crate::wallet::verify_signature;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub nonce: u64,
    pub public_key: [u8; 32],
    pub signature: Vec<u8>,
}

impl Transaction {
    /// Produces the exact message bytes that the sender must sign.
    pub fn signing_message(sender: &str, recipient: &str, amount: u64, nonce: u64) -> Vec<u8> {
        format!("LOKYX-TX-V1|{}|{}|{}|{}", sender, recipient, amount, nonce).into_bytes()
    }

    /// Returns a deterministic 64-character SHA-256 transaction ID.
    pub fn transaction_id(&self) -> String {
        let message = Self::signing_message(&self.sender, &self.recipient, self.amount, self.nonce);

        let mut hasher = Sha256::new();
        hasher.update(&message);
        hasher.update(self.public_key);
        hasher.update(&self.signature);

        let digest = hasher.finalize();
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// Validates addresses, sender/public-key matching, and the signature.
    pub fn is_valid(&self) -> bool {
        if self.amount == 0 || self.sender == self.recipient {
            return false;
        }

        if !crate::wallet::validate_address(&self.sender)
            || !crate::wallet::validate_address(&self.recipient)
        {
            return false;
        }

        let expected_sender = match bech32::encode::<bech32::Bech32>(
            bech32::Hrp::parse("lkx").expect("valid address prefix"),
            &self.public_key,
        ) {
            Ok(address) => address,
            Err(_) => return false,
        };

        if expected_sender != self.sender {
            return false;
        }

        let message = Self::signing_message(&self.sender, &self.recipient, self.amount, self.nonce);

        verify_signature(&self.public_key, &message, &self.signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    fn create_signed_transaction(amount: u64) -> Transaction {
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
    fn signed_transaction_is_valid() {
        let transaction = create_signed_transaction(25);

        assert!(transaction.is_valid());
    }

    #[test]
    fn changed_amount_invalidates_signature() {
        let mut transaction = create_signed_transaction(25);
        transaction.amount = 100;

        assert!(!transaction.is_valid());
    }

    #[test]
    fn zero_amount_is_rejected() {
        let transaction = create_signed_transaction(0);

        assert!(!transaction.is_valid());
    }

    #[test]
    fn transaction_id_is_64_hex_characters() {
        let transaction = create_signed_transaction(25);
        let id = transaction.transaction_id();

        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|character| character.is_ascii_hexdigit()));
    }
}
