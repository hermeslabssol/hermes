//! [`Account`] and the in-memory [`AccountsDb`].

use hermes_primitives::{Lamports, Pubkey};
use std::collections::HashMap;

/// An SVM account: the only place state lives in Hermes.
///
/// Mirrors Solana's account model. The `owner` program is the sole authority
/// that may debit `lamports` or mutate `data`; the runtime enforces this.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Account {
    /// Balance in lamports.
    pub lamports: Lamports,
    /// Opaque account data, interpreted by the owner program.
    pub data: Vec<u8>,
    /// Program that owns (and may modify) this account.
    pub owner: Pubkey,
    /// Whether this account holds an executable program.
    pub executable: bool,
}

impl Account {
    /// Create a data-carrying account owned by `owner`.
    #[must_use]
    pub fn new(lamports: Lamports, data: Vec<u8>, owner: Pubkey) -> Self {
        Self {
            lamports,
            data,
            owner,
            executable: false,
        }
    }

    /// Create an empty (zero-data) account owned by `owner`.
    #[must_use]
    pub fn empty(lamports: Lamports, owner: Pubkey) -> Self {
        Self::new(lamports, Vec::new(), owner)
    }

    /// Create an executable program account owned by `owner` (typically the
    /// loader / system program on Solana).
    #[must_use]
    pub fn program(owner: Pubkey) -> Self {
        Self {
            lamports: Lamports::ZERO,
            data: Vec::new(),
            owner,
            executable: true,
        }
    }
}

impl core::fmt::Debug for Account {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Account")
            .field("lamports", &self.lamports.get())
            .field("data_len", &self.data.len())
            .field("owner", &self.owner.to_base58())
            .field("executable", &self.executable)
            .finish()
    }
}

/// An in-memory account store keyed by [`Pubkey`].
///
/// On a real validator this would be backed by AccountsDB-on-disk; for Hermes
/// devnet a map is sufficient and keeps the runtime hermetic for tests.
#[derive(Debug, Default, Clone)]
pub struct AccountsDb {
    accounts: HashMap<Pubkey, Account>,
}

impl AccountsDb {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or overwrite an account.
    pub fn store(&mut self, key: Pubkey, account: Account) {
        self.accounts.insert(key, account);
    }

    /// Borrow an account by key.
    #[must_use]
    pub fn load(&self, key: &Pubkey) -> Option<&Account> {
        self.accounts.get(key)
    }

    /// Mutably borrow an account by key.
    pub fn load_mut(&mut self, key: &Pubkey) -> Option<&mut Account> {
        self.accounts.get_mut(key)
    }

    /// True if `key` has an account.
    #[must_use]
    pub fn contains(&self, key: &Pubkey) -> bool {
        self.accounts.contains_key(key)
    }

    /// Number of stored accounts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    /// True if the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    /// Lamport balance of `key`, or zero if absent.
    #[must_use]
    pub fn balance(&self, key: &Pubkey) -> Lamports {
        self.accounts
            .get(key)
            .map(|a| a.lamports)
            .unwrap_or(Lamports::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_load() {
        let mut db = AccountsDb::new();
        let key = Pubkey::new([1u8; 32]);
        db.store(key, Account::empty(Lamports::new(100), Pubkey::SYSTEM));
        assert_eq!(db.balance(&key).get(), 100);
        assert!(db.contains(&key));
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn load_mut_mutates() {
        let mut db = AccountsDb::new();
        let key = Pubkey::new([2u8; 32]);
        db.store(key, Account::empty(Lamports::new(10), Pubkey::SYSTEM));
        db.load_mut(&key).unwrap().lamports = Lamports::new(50);
        assert_eq!(db.balance(&key).get(), 50);
    }

    #[test]
    fn absent_account_zero_balance() {
        let db = AccountsDb::new();
        assert_eq!(db.balance(&Pubkey::new([9u8; 32])).get(), 0);
        assert!(db.is_empty());
    }
}
