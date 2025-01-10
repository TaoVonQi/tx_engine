use crate::{
    transaction::{Transaction, TransactionType},
    EngineError,
};

use serde::ser::{Serialize, SerializeStruct};
use std::{collections::HashMap, fmt::Display};

#[derive(Debug)]
pub struct Client {
    tx_map: HashMap<u32, Transaction>, // map: tx_id -> transaction
    pub summary: ClientSummary,
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
        if self.tx_map.contains_key(&tx.tx_id) {
            return Err(EngineError::DuplicateTransaction(format!("{}", tx.tx_id)));
        }

        self.summary.deposit(tx)?;
        self.tx_map.insert(tx.tx_id, tx.clone());

        Ok(())
    }

    pub fn withdraw(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        // Ensure idempotence
        if self.tx_map.contains_key(&tx.tx_id) {
            return Err(EngineError::DuplicateTransaction(format!("{}", tx.tx_id)));
        }

        self.summary.withdraw(tx)?;
        self.tx_map.insert(tx.tx_id, tx.clone());

        Ok(())
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

#[derive(Debug)]
pub struct ClientSummary {
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

    pub fn get_client_id(&self) -> u16 {
        self.client_id
    }

    pub fn validate_tx_get_amount(&self, tx: &Transaction) -> Result<f64, EngineError> {
        if self.locked {
            return Err(EngineError::AccountLocked);
        }

        if tx.amount.is_none() {
            return Err(EngineError::InvalidTransaction(format!(
                "Tx ID: {}",
                tx.tx_id
            )));
        }

        let amount = tx.amount.unwrap();

        if amount <= 0.0 {
            return Err(EngineError::InvalidTransaction(format!(
                "Tx ID: {} invalid amount",
                tx.tx_id
            )));
        }

        Ok(amount)
    }

    fn deposit(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        let amount = self.validate_tx_get_amount(tx)?;

        self.available += amount;
        self.total += amount;

        Ok(())
    }

    fn withdraw(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        let amount = self.validate_tx_get_amount(tx)?;

        if self.available < amount {
            return Err(EngineError::InsufficientFunds);
        }

        self.available -= amount;
        self.total -= amount;

        Ok(())
    }

    fn dispute(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        let amount = self.validate_tx_get_amount(disputed_tx)?;

        // Assuming here that only deposit transactions can be disputed
        if disputed_tx.tx_type != TransactionType::Deposit {
            return Err(EngineError::DisputeError(format!(
                "Attempt to dispute non deposit tx"
            )));
        }

        // Ensure idempotence
        if disputed_tx.disputed {
            return Err(EngineError::DisputeError(format!(
                "TX {} is already disputed",
                disputed_tx.tx_id
            )));
        }

        self.available -= amount;
        self.held += amount;

        Ok(())
    }

    fn resolve(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        let amount = self.validate_tx_get_amount(disputed_tx)?;

        // Only resolve transactions that where previously disputed.
        if !disputed_tx.disputed {
            return Err(EngineError::ResolveError(format!(
                "TX {} is undisputed",
                disputed_tx.tx_id
            )));
        }

        // Ensure idempotence
        if disputed_tx.resolved {
            return Err(EngineError::ResolveError(format!(
                "TX {} is already resolved",
                disputed_tx.tx_id
            )));
        }

        self.available += amount;
        self.held -= amount;

        Ok(())
    }

    fn charge_back(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        let amount = self.validate_tx_get_amount(disputed_tx)?;

        // Only chargeback transactions that where previously disputed.
        if !disputed_tx.disputed {
            return Err(EngineError::ChargeBackError(format!(
                "TX {} is undisputed",
                disputed_tx.tx_id
            )));
        }

        // Can not chargeback transactions that are already resolved.
        if disputed_tx.resolved {
            return Err(EngineError::ChargeBackError(format!(
                "TX {} is already resolved",
                disputed_tx.tx_id
            )));
        }

        self.total -= amount;
        self.held -= amount;
        self.locked = true;

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

impl Serialize for ClientSummary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // 5 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("ClientSummary", 5)?;
        state.serialize_field("client", &self.client_id)?;
        state.serialize_field(" available", &format!(" {:.4}", &self.available))?;
        state.serialize_field(" held", &format!(" {:.4}", &self.held))?;
        state.serialize_field(" total", &format!(" {:.4}", &self.total))?;
        state.serialize_field(" locked", &format!(" {}", &self.locked))?;
        state.end()
    }
}
