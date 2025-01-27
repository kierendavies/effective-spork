use std::collections::{btree_map::Entry, BTreeMap};

use rust_decimal::Decimal;

use crate::{ClientId, TransactionId};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("transaction already disputed: {0}")]
    AlreadyDisputed(TransactionId),

    #[error("client does not match (tx: {tx}, expected: {expected}, found: {found})")]
    ClientMismatch {
        tx: TransactionId,
        expected: ClientId,
        found: ClientId,
    },

    #[error("duplicate transaction ID: {0}")]
    DuplicateTransactionId(TransactionId),

    #[error(
        "insufficient funds (client: {client}, available: {available}, requested: {requested})"
    )]
    InsufficientFunds {
        client: ClientId,
        available: Decimal,
        requested: Decimal,
    },

    #[error("account locked: {0}")]
    Locked(ClientId),

    #[error("transaction not disputed: {0}")]
    NotDisputed(TransactionId),

    #[error("transaction not found: {0}")]
    TransactionNotFound(TransactionId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DepositState {
    Ok,
    Dispute,
    Chargeback,
}

#[derive(Clone, Copy, Debug)]
struct Deposit {
    client: ClientId,
    amount: Decimal,
    state: DepositState,
}

#[derive(Clone, Copy, Debug)]
pub struct Account {
    pub total: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn available(&self) -> Decimal {
        self.total - self.held
    }
}

impl Default for Account {
    fn default() -> Self {
        Self {
            total: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        }
    }
}

#[derive(Debug)]
pub struct Engine {
    accounts: BTreeMap<ClientId, Account>,
    deposits: BTreeMap<TransactionId, Deposit>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            deposits: BTreeMap::new(),
        }
    }

    pub fn accounts(&self) -> impl Iterator<Item = (&ClientId, &Account)> {
        self.accounts.iter()
    }

    pub fn deposit(
        &mut self,
        client: ClientId,
        tx: TransactionId,
        amount: Decimal,
    ) -> Result<(), Error> {
        let account = self.accounts.entry(client).or_default();

        if account.locked {
            return Err(Error::Locked(client));
        }

        match self.deposits.entry(tx) {
            Entry::Vacant(entry) => {
                _ = entry.insert(Deposit {
                    client,
                    amount,
                    state: DepositState::Ok,
                });
            }
            Entry::Occupied(_) => return Err(Error::DuplicateTransactionId(tx)),
        }

        account.total += amount;

        Ok(())
    }

    pub fn withdraw(
        &mut self,
        client: ClientId,
        _tx: TransactionId,
        amount: Decimal,
    ) -> Result<(), Error> {
        let account = self.accounts.entry(client).or_default();

        if account.locked {
            return Err(Error::Locked(client));
        }

        if account.available() < amount {
            return Err(Error::InsufficientFunds {
                client,
                available: account.available(),
                requested: amount,
            });
        }

        account.total -= amount;

        Ok(())
    }

    pub fn dispute(&mut self, client: ClientId, tx: TransactionId) -> Result<(), Error> {
        let account = self.accounts.entry(client).or_default();

        let deposit = self
            .deposits
            .get_mut(&tx)
            .ok_or(Error::TransactionNotFound(tx))?;

        if deposit.client != client {
            return Err(Error::ClientMismatch {
                tx,
                expected: client,
                found: deposit.client,
            });
        }

        if deposit.state != DepositState::Ok {
            return Err(Error::AlreadyDisputed(tx));
        }

        // If `deposit.amount > account.total`? Should be fine, right?

        deposit.state = DepositState::Dispute;
        account.held += deposit.amount;

        Ok(())
    }

    pub fn resolve(&mut self, client: ClientId, tx: TransactionId) -> Result<(), Error> {
        let account = self.accounts.entry(client).or_default();

        let deposit = self
            .deposits
            .get_mut(&tx)
            .ok_or(Error::TransactionNotFound(tx))?;

        if deposit.client != client {
            return Err(Error::ClientMismatch {
                tx,
                expected: client,
                found: deposit.client,
            });
        }

        if deposit.state != DepositState::Dispute {
            return Err(Error::NotDisputed(tx));
        }

        deposit.state = DepositState::Ok;
        account.held -= deposit.amount;

        Ok(())
    }

    pub fn chargeback(&mut self, client: ClientId, tx: TransactionId) -> Result<(), Error> {
        let account = self.accounts.entry(client).or_default();

        let deposit = self
            .deposits
            .get_mut(&tx)
            .ok_or(Error::TransactionNotFound(tx))?;

        if deposit.client != client {
            return Err(Error::ClientMismatch {
                tx,
                expected: client,
                found: deposit.client,
            });
        }

        if deposit.state != DepositState::Dispute {
            return Err(Error::NotDisputed(tx));
        }

        deposit.state = DepositState::Chargeback;
        account.held -= deposit.amount;
        account.total -= deposit.amount;
        account.locked = true;

        Ok(())
    }
}
