
use crate::transaction::Transaction;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AccountState {
    pub balance: u64,
    pub nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    InvalidTransaction,
    IncorrectNonce {
        expected: u64,
        received: u64,
    },
    InsufficientFunds,
    BalanceOverflow,
    NonceOverflow,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    accounts: BTreeMap<String, AccountState>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates ledger state from explicit genesis allocations.
    /// This is a local initialization function, not a public mint endpoint.
    pub fn from_genesis_allocations(
        allocations: impl IntoIterator<Item = (String, u64)>,
    ) -> Self {
        let mut ledger = Self::new();

        for (address, balance) in allocations {
            ledger.accounts.insert(
                address,
                AccountState {
                    balance,
                    nonce: 0,
                },
            );
        }

        ledger
    }

    pub fn account(&self, address: &str) -> AccountState {
        self.accounts.get(address).copied().unwrap_or_default()
    }

    /// Validates and executes a transfer atomically.
    /// Failed transactions leave the ledger unchanged.
    pub fn apply_transaction(
        &mut self,
        transaction: &Transaction,
    ) -> Result<(), LedgerError> {
        if !transaction.is_valid() {
            return Err(LedgerError::InvalidTransaction);
        }

        let sender = self.account(&transaction.sender);
        let recipient = self.account(&transaction.recipient);

        if transaction.nonce != sender.nonce {
            return Err(LedgerError::IncorrectNonce {
                expected: sender.nonce,
                received: transaction.nonce,
            });
        }

        if sender.balance < transaction.amount {
            return Err(LedgerError::InsufficientFunds);
        }

        let recipient_balance = recipient
            .balance
            .checked_add(transaction.amount)
            .ok_or(LedgerError::BalanceOverflow)?;

        let next_nonce = sender
            .nonce
            .checked_add(1)
            .ok_or(LedgerError::NonceOverflow)?;

        // All checks happen before either account is modified.
        self.accounts.insert(
            transaction.sender.clone(),
            AccountState {
                balance: sender.balance - transaction.amount,
                nonce: next_nonce,
            },
        );

        self.accounts.insert(
            transaction.recipient.clone(),
            AccountState {
                balance: recipient_balance,
                nonce: recipient.nonce,
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    fn signed_transaction(
        sender: &Wallet,
        recipient: &Wallet,
        amount: u64,
        nonce: u64,
    ) -> Transaction {
        let sender_address = sender.address();
        let recipient_address = recipient.address();

        let message = Transaction::signing_message(
            &sender_address,
            &recipient_address,
            amount,
            nonce,
        );

        Transaction {
            sender: sender_address,
            recipient: recipient_address,
            amount,
            nonce,
            public_key: sender.public_key(),
            signature: sender.sign(&message),
        }
    }

    fn funded_ledger(sender: &Wallet, balance: u64) -> Ledger {
        Ledger::from_genesis_allocations([(
            sender.address(),
            balance,
        )])
    }

    #[test]
    fn valid_transfer_updates_both_balances() {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();
        let mut ledger = funded_ledger(&sender, 100);

        let transaction = signed_transaction(&sender, &recipient, 30, 0);

        assert_eq!(ledger.apply_transaction(&transaction), Ok(()));
        assert_eq!(ledger.account(&sender.address()).balance, 70);
        assert_eq!(ledger.account(&recipient.address()).balance, 30);
        assert_eq!(ledger.account(&sender.address()).nonce, 1);
    }

    #[test]
    fn overspending_is_rejected_without_changing_state() {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();
        let mut ledger = funded_ledger(&sender, 20);
        let before = ledger.clone();

        let transaction = signed_transaction(&sender, &recipient, 21, 0);

        assert_eq!(
            ledger.apply_transaction(&transaction),
            Err(LedgerError::InsufficientFunds)
        );
        assert_eq!(ledger, before);
    }

    #[test]
    fn incorrect_nonce_is_rejected() {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();
        let mut ledger = funded_ledger(&sender, 100);

        let transaction = signed_transaction(&sender, &recipient, 10, 1);

        assert_eq!(
            ledger.apply_transaction(&transaction),
            Err(LedgerError::IncorrectNonce {
                expected: 0,
                received: 1,
            })
        );
    }

    #[test]
    fn replayed_transaction_is_rejected() {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();
        let mut ledger = funded_ledger(&sender, 100);

        let transaction = signed_transaction(&sender, &recipient, 10, 0);

        assert_eq!(ledger.apply_transaction(&transaction), Ok(()));
        assert_eq!(
            ledger.apply_transaction(&transaction),
            Err(LedgerError::IncorrectNonce {
                expected: 1,
                received: 0,
            })
        );

        assert_eq!(ledger.account(&sender.address()).balance, 90);
        assert_eq!(ledger.account(&recipient.address()).balance, 10);
    }

    #[test]
    fn tampered_transaction_is_rejected() {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();
        let mut ledger = funded_ledger(&sender, 100);
        let before = ledger.clone();

        let mut transaction = signed_transaction(&sender, &recipient, 10, 0);
        transaction.amount = 50;

        assert_eq!(
            ledger.apply_transaction(&transaction),
            Err(LedgerError::InvalidTransaction)
        );
        assert_eq!(ledger, before);
    }

    #[test]
    fn recipient_balance_overflow_is_rejected_atomically() {
        let sender = Wallet::generate();
        let recipient = Wallet::generate();

        let mut ledger = Ledger::from_genesis_allocations([
            (sender.address(), 100),
            (recipient.address(), u64::MAX),
        ]);
        let before = ledger.clone();

        let transaction = signed_transaction(&sender, &recipient, 1, 0);

        assert_eq!(
            ledger.apply_transaction(&transaction),
            Err(LedgerError::BalanceOverflow)
        );
        assert_eq!(ledger, before);
    }
}
