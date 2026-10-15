use crate::wallet::verify_signature;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The network identifier is included in the signed transaction message.
pub const NETWORK_ID: &str = "lokyx-testnet-1";
const TRANSACTION_DOMAIN: &[u8] = b"LOKYX-TX-V2";

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
    /// Encodes a transaction message unambiguously.
    ///
    /// Length-prefixing the variable-length fields avoids delimiter collisions.
    pub fn signing_message(sender: &str, recipient: &str, amount: u64, nonce: u64) -> Vec<u8> {
        Self::signing_message_for_network(NETWORK_ID, sender, recipient, amount, nonce)
    }

    /// Builds the canonical signed message for a specific network.
    pub fn signing_message_for_network(
        network_id: &str,
        sender: &str,
        recipient: &str,
        amount: u64,
        nonce: u64,
    ) -> Vec<u8> {
        let mut message = Vec::with_capacity(
            TRANSACTION_DOMAIN.len()
                + 8
                + network_id.len()
                + 8
                + sender.len()
                + 8
                + recipient.len()
                + 16,
        );

        message.extend_from_slice(TRANSACTION_DOMAIN);
        Self::append_bytes(&mut message, network_id.as_bytes());
        Self::append_bytes(&mut message, sender.as_bytes());
        Self::append_bytes(&mut message, recipient.as_bytes());
        message.extend_from_slice(&amount.to_be_bytes());
        message.extend_from_slice(&nonce.to_be_bytes());

        message
    }

    fn append_bytes(output: &mut Vec<u8>, value: &[u8]) {
        let length = u64::try_from(value.len()).expect("field length must fit in a u64");
        output.extend_from_slice(&length.to_be_bytes());
        output.extend_from_slice(value);
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

    fn signed_transaction(amount: u64, nonce: u64) -> Transaction {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();

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
    fn signed_transaction_is_valid() {
        assert!(signed_transaction(25, 0).is_valid());
    }

    #[test]
    fn changed_amount_invalidates_signature() {
        let mut transaction = signed_transaction(25, 0);
        transaction.amount = 100;

        assert!(!transaction.is_valid());
    }

    #[test]
    fn zero_amount_is_rejected() {
        assert!(!signed_transaction(0, 0).is_valid());
    }

    #[test]
    fn transaction_id_is_64_hex_characters() {
        let id = signed_transaction(25, 0).transaction_id();

        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn network_identifier_changes_signed_message() {
        let first = Transaction::signing_message_for_network(
            "lokyx-testnet-1",
            "sender",
            "recipient",
            25,
            0,
        );
        let second = Transaction::signing_message_for_network(
            "lokyx-testnet-2",
            "sender",
            "recipient",
            25,
            0,
        );

        assert_ne!(first, second);
    }

    #[test]
    fn variable_length_fields_are_encoded_unambiguously() {
        let first = Transaction::signing_message("ab", "c", 25, 0);
        let second = Transaction::signing_message("a", "bc", 25, 0);

        assert_ne!(first, second);
    }
}
