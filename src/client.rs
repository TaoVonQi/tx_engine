use crate::{
    transaction::{Transaction, TransactionType},
    EngineError,
};

use serde::Serialize;
use std::collections::HashMap;

pub struct Client {
    tx_map: HashMap<u32, Transaction>, // map: tx_id -> transaction
    summary: ClientSummary,
}

impl Client {
    fn new(client_id: u16) -> Self {
        Client {
            tx_map: HashMap::new(),
            summary: ClientSummary::new(client_id),
        }
    }

    fn deposit(&mut self, tx: Transaction) -> Result<(), EngineError> {
        self.summary.deposit(&tx)?;
        self.tx_map.insert(tx.id, tx);
        Ok(())
    }

    fn withdraw(&mut self, tx: Transaction) -> Result<(), EngineError> {
        self.summary.withdraw(&tx)?;
        self.tx_map.insert(tx.id, tx);
        Ok(())
    }

    fn dispute(&mut self, tx: Transaction) -> Result<(), EngineError> {
        if let Some(disputed_tx) = self.tx_map.get_mut(&tx.id) {
            self.summary.dispute(&disputed_tx)?;
            disputed_tx.disputed = true;

            self.tx_map.insert(tx.id, tx);
            Ok(())
        } else {
            Err(EngineError::DisputeError(format!(
                "Invalid TX ID: {}",
                tx.id
            )))
        }
    }

    fn resolve(&mut self, tx: Transaction) -> Result<(), EngineError> {
        if let Some(transaction) = self.tx_map.get_mut(&tx.id) {
            self.summary.resolve(&transaction)?;
            transaction.resolved = true;

            self.tx_map.insert(tx.id, tx);
            Ok(())
        } else {
            Err(EngineError::ResolveError(format!(
                "Infalid TX ID: {}",
                tx.id
            )))
        }
    }

    fn charge_back(&mut self, tx: Transaction) -> Result<(), EngineError> {
        if let Some(transaction) = self.tx_map.get(&tx.id) {
            self.summary.charge_back(&transaction)?;
            self.tx_map.insert(tx.id, tx);
            Ok(())
        } else {
            Err(EngineError::ChargeBackError)
        }
    }
}

#[derive(Debug, Serialize)]
struct ClientSummary {
    id: u16,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}

impl ClientSummary {
    fn new(client_id: u16) -> Self {
        ClientSummary {
            id: client_id,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }

    fn deposit(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        if !self.locked {
            if let Some(amount) = tx.amount {
                self.available += amount;
                self.total += amount;
            } else {
                return Err(EngineError::InvalidTransaction);
            }
        } else {
            return Err(EngineError::AccountLocked);
        }

        Ok(())
    }

    fn withdraw(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        if !self.locked {
            if let Some(amount) = tx.amount {
                if self.available >= amount {
                    self.available -= amount;
                    self.total -= amount;
                } else {
                    return Err(EngineError::InsufficientFunds);
                }
            } else {
                return Err(EngineError::InvalidTransaction);
            }
        } else {
            return Err(EngineError::AccountLocked);
        }

        Ok(())
    }

    fn dispute(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        if disputed_tx.tx_type == TransactionType::Deposit {
            if let Some(amount) = disputed_tx.amount {
                if !disputed_tx.disputed {
                    self.available -= amount;
                    self.held += amount;
                } else {
                    return Err(EngineError::DisputeError(format!(
                        "TX {} is already disputed",
                        disputed_tx.id
                    )));
                }
            } else {
                return Err(EngineError::InvalidTransaction);
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
            if disputed_tx.disputed {
                if !disputed_tx.resolved {
                    self.available += amount;
                    self.held -= amount;
                } else {
                    return Err(EngineError::ResolveError(format!(
                        "TX {} is already resolved",
                        disputed_tx.id
                    )));
                }
            } else {
                return Err(EngineError::ResolveError(format!(
                    "TX {} is undisputed",
                    disputed_tx.id
                )));
            }
        } else {
            return Err(EngineError::InvalidTransaction);
        }

        Ok(())
    }

    fn charge_back(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        if let Some(amount) = disputed_tx.amount {
            self.total -= amount;
            self.held -= amount;
            self.available = self.total - self.held;
            self.locked = true;
        } else {
            return Err(EngineError::InvalidTransaction);
        }

        Ok(())
    }
}
