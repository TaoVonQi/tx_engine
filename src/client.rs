use crate::{
    transaction::{Transaction, TransactionType},
    EngineError,
};

use serde::Serialize;
use std::{collections::HashMap, fmt::Display};

#[derive(Debug)]
pub struct Client {
    tx_map: HashMap<u32, Transaction>, // map: tx_id -> transaction
    summary: ClientSummary,
}

impl Client {
    pub fn new(client_id: u16) -> Self {
        Client {
            tx_map: HashMap::new(),
            summary: ClientSummary::new(client_id),
        }
    }

    pub fn deposit(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        // Ensure idempotence
        if !self.tx_map.contains_key(&tx.tx_id) {
            self.summary.deposit(tx)?;
            self.tx_map.insert(tx.tx_id, tx.clone());
            Ok(())
        } else {
            Err(EngineError::DuplicateTransaction(format!("{}", tx.tx_id)))
        }
    }

    pub fn withdraw(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        // Ensure idempotence
        if !self.tx_map.contains_key(&tx.tx_id) {
            self.summary.withdraw(tx)?;
            self.tx_map.insert(tx.tx_id, tx.clone());
            Ok(())
        } else {
            Err(EngineError::DuplicateTransaction(format!("{}", tx.tx_id)))
        }
    }

    pub fn dispute(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        // Fetch referenced transaction from client's tx map
        if let Some(disputed_tx) = self.tx_map.get_mut(&tx.tx_id) {
            self.summary.dispute(&disputed_tx)?;
            disputed_tx.disputed = true;

            Ok(())
        } else {
            Err(EngineError::DisputeError(format!(
                "Invalid TX ID: {} for client: {}",
                tx.tx_id, self.summary.client_id
            )))
        }
    }

    pub fn resolve(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        // Fetch referenced transaction from client's tx map
        if let Some(transaction) = self.tx_map.get_mut(&tx.tx_id) {
            self.summary.resolve(&transaction)?;
            transaction.resolved = true;

            Ok(())
        } else {
            Err(EngineError::ResolveError(format!(
                "Invalid TX ID: {} for client: {}",
                tx.tx_id, self.summary.client_id
            )))
        }
    }

    pub fn charge_back(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        // Fetch referenced transaction from client's tx map
        if let Some(transaction) = self.tx_map.get(&tx.tx_id) {
            self.summary.charge_back(&transaction)?;

            Ok(())
        } else {
            Err(EngineError::ChargeBackError(format!(
                "Invalid TX ID: {} for client: {}",
                tx.tx_id, self.summary.client_id
            )))
        }
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary)
    }
}

#[derive(Debug, Serialize)]
struct ClientSummary {
    client_id: u16,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}

impl ClientSummary {
    fn new(client_id: u16) -> Self {
        ClientSummary {
            client_id,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }

    fn deposit(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        if !self.locked {
            if let Some(amount) = tx.amount {
                if amount > 0.0 {
                    self.available += amount;
                    self.total += amount;
                } else {
                    return Err(EngineError::InvalidTransaction(format!(
                        "Tx ID: {} invalid amount",
                        tx.tx_id
                    )));
                }
            } else {
                return Err(EngineError::InvalidTransaction(format!(
                    "Tx ID: {}",
                    tx.tx_id
                )));
            }
        } else {
            return Err(EngineError::AccountLocked);
        }

        Ok(())
    }

    fn withdraw(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        if !self.locked {
            if let Some(amount) = tx.amount {
                if amount > 0.0 {
                    if self.available >= amount {
                        self.available -= amount;
                        self.total -= amount;
                    } else {
                        return Err(EngineError::InsufficientFunds);
                    }
                } else {
                    return Err(EngineError::InvalidTransaction(format!(
                        "Tx ID: {} invalid amount",
                        tx.tx_id
                    )));
                }
            } else {
                return Err(EngineError::InvalidTransaction(format!(
                    "Tx ID: {}",
                    tx.tx_id
                )));
            }
        } else {
            return Err(EngineError::AccountLocked);
        }

        Ok(())
    }

    fn dispute(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        // Assumed here that only deposit transactions can be disputed
        if disputed_tx.tx_type == TransactionType::Deposit {
            if let Some(amount) = disputed_tx.amount {
                // Ensure idempotence
                if !disputed_tx.disputed {
                    self.available -= amount;
                    self.held += amount;
                } else {
                    return Err(EngineError::DisputeError(format!(
                        "TX {} is already disputed",
                        disputed_tx.tx_id
                    )));
                }
            } else {
                return Err(EngineError::InvalidTransaction(format!(
                    "Tx ID: {}",
                    disputed_tx.tx_id
                )));
            }
        } else {
            return Err(EngineError::DisputeError(format!(
                "Attempt to dispute non deposit tx"
            )));
        }

        Ok(())
    }

    fn resolve(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        if let Some(amount) = disputed_tx.amount {
            // Only resolve transactions that where previously disputed.
            if disputed_tx.disputed {
                // Ensure idempotence
                if !disputed_tx.resolved {
                    self.available += amount;
                    self.held -= amount;
                } else {
                    return Err(EngineError::ResolveError(format!(
                        "TX {} is already resolved",
                        disputed_tx.tx_id
                    )));
                }
            } else {
                return Err(EngineError::ResolveError(format!(
                    "TX {} is undisputed",
                    disputed_tx.tx_id
                )));
            }
        } else {
            return Err(EngineError::InvalidTransaction(format!(
                "Tx ID: {}",
                disputed_tx.tx_id
            )));
        }

        Ok(())
    }

    fn charge_back(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        if !self.locked {
            // Allowing charge_backs to occur without being disputed to lock account for further
            // investigation
            if let Some(amount) = disputed_tx.amount {
                self.total -= amount;
                self.held -= amount;
                self.available = self.total - self.held;
                self.locked = true;
            } else {
                return Err(EngineError::InvalidTransaction(format!(
                    "Tx ID: {}",
                    disputed_tx.tx_id
                )));
            }
        } else {
            return Err(EngineError::AccountLocked);
        }

        Ok(())
    }
}

impl Display for ClientSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Client ID: {}, Available: {}, Held: {}, Total: {}, locked: {}",
            self.client_id, self.available, self.held, self.total, self.locked
        )
    }
}
