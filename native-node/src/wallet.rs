use bech32::{Bech32, Hrp};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::TryRng;

#[derive(Debug)]
pub struct Wallet {
    signing_key: SigningKey,
}

impl Wallet {
    pub fn generate() -> Self {
        let mut secret = [0u8; 32];
        rand::rngs::SysRng
            .try_fill_bytes(&mut secret)
            .expect("Secure operating-system randomness unavailable");

        Self {
            signing_key: SigningKey::from_bytes(&secret),
        }
    }

    pub fn address(&self) -> String {
        let public_key = self.signing_key.verifying_key();
        bech32::encode::<Bech32>(
            Hrp::parse("lkx").expect("valid address prefix"),
            public_key.as_bytes(),
        )
        .expect("public key address encoding should succeed")
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    pub fn public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }
}

pub fn verify_signature(public_key: &[u8; 32], message: &[u8], signature_bytes: &[u8]) -> bool {
    let Ok(key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };

    let Ok(signature) = Signature::from_slice(signature_bytes) else {
        return false;
    };

    key.verify(message, &signature).is_ok()
}

pub fn validate_address(address: &str) -> bool {
    match bech32::decode(address) {
        Ok((hrp, data)) => hrp.as_str() == "lkx" && data.len() == 32,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_address_uses_lkx_prefix() {
        let wallet = Wallet::generate();
        let address = wallet.address();

        assert!(address.starts_with("lkx1"));
        assert!(validate_address(&address));
    }

    #[test]
    fn signature_verifies_for_original_message() {
        let wallet = Wallet::generate();
        let message = b"LOKYX wallet test";
        let signature = wallet.sign(message);
        let public_key = wallet.public_key();

        assert!(verify_signature(&public_key, message, &signature));
        assert!(!verify_signature(
            &public_key,
            b"changed message",
            &signature
        ));
    }

    #[test]
    fn altered_address_is_rejected() {
        let wallet = Wallet::generate();
        let mut address = wallet.address();
        address.push('q');

        assert!(!validate_address(&address));
    }
}
