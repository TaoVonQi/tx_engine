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

    fn validate_tx(
        &self,
        tx: &Transaction,
        expected_tx_type: TransactionType,
    ) -> Result<(), EngineError> {
        if tx.client_id != self.summary.client_id {
            return Err(EngineError::InvalidTransaction(format!(
                "tx client ID mismatch {}",
                tx.tx_id
            )));
        }

        if tx.tx_type != expected_tx_type {
            return Err(EngineError::OtherError(format!(
                "Expected {} transaction, ID: {}",
                expected_tx_type, tx.tx_id
            )));
        }

        Ok(())
    }

    pub fn deposit(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        self.validate_tx(tx, TransactionType::Deposit)?;

        // Ensure idempotence
        if self.tx_map.contains_key(&tx.tx_id) {
            return Err(EngineError::DuplicateTransaction(format!("{}", tx.tx_id)));
        }

        self.summary.deposit(tx)?;
        self.tx_map.insert(tx.tx_id, tx.clone());

        Ok(())
    }

    pub fn withdraw(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        self.validate_tx(tx, TransactionType::Withdrawal)?;

        // Ensure idempotence
        if self.tx_map.contains_key(&tx.tx_id) {
            return Err(EngineError::DuplicateTransaction(format!("{}", tx.tx_id)));
        }

        self.summary.withdraw(tx)?;
        self.tx_map.insert(tx.tx_id, tx.clone());

        Ok(())
    }

    pub fn dispute(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        self.validate_tx(tx, TransactionType::Dispute)?;

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
        self.validate_tx(tx, TransactionType::Resolve)?;

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
        self.validate_tx(tx, TransactionType::ChargeBack)?;

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

    pub fn validate_and_get_amount(&self, tx: &Transaction) -> Result<f64, EngineError> {
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
        let amount = self.validate_and_get_amount(tx)?;

        self.available += amount;
        self.total += amount;

        Ok(())
    }

    fn withdraw(&mut self, tx: &Transaction) -> Result<(), EngineError> {
        let amount = self.validate_and_get_amount(tx)?;

        if self.available < amount {
            return Err(EngineError::InsufficientFunds);
        }

        self.available -= amount;
        self.total -= amount;

        Ok(())
    }

    fn dispute(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        let amount = self.validate_and_get_amount(disputed_tx)?;

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

        if self.available < amount {
            return Err(EngineError::InsufficientFunds);
        }

        self.available -= amount;
        self.held += amount;

        Ok(())
    }

    fn resolve(&mut self, disputed_tx: &Transaction) -> Result<(), EngineError> {
        let amount = self.validate_and_get_amount(disputed_tx)?;

        // Only resolve transactions that were previously disputed.
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
        let amount = self.validate_and_get_amount(disputed_tx)?;

        // Only chargeback transactions that were previously disputed.
        if !disputed_tx.disputed {
            return Err(EngineError::ChargeBackError(format!(
                "TX {} is undisputed",
                disputed_tx.tx_id
            )));
        }

        // Do not chargeback transactions that are already resolved.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mismatch_tx_id() -> Result<(), EngineError> {
        let mut client = Client::new(1);

        let transaction = Transaction {
            tx_id: 1,
            client_id: 2,
            tx_type: TransactionType::Deposit,
            amount: Some(1.0),
            disputed: false,
            resolved: false,
        };

        if let Err(_) = client.deposit(&transaction) {
            Ok(())
        } else {
            Err(EngineError::OtherError(
                "Should fail inconsistent transaction for client".to_string(),
            ))
        }
    }

    #[test]
    fn test_duplicate() -> Result<(), EngineError> {
        let mut client = Client::new(1);

        let tx = Transaction {
            tx_id: 1,
            client_id: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(1.0),
            disputed: false,
            resolved: false,
        };

        client.deposit(&tx)?;

        let result = client.deposit(&tx);

        assert_eq!(
            result,
            Err(EngineError::DuplicateTransaction(format!("{}", tx.tx_id)))
        );

        Ok(())
    }

    #[test]
    fn test_insuffiecient_funds() -> Result<(), EngineError> {
        let mut client = Client::new(1);

        let deposit_tx = Transaction {
            tx_id: 1,
            client_id: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(1.0),
            disputed: false,
            resolved: false,
        };

        let mut withdraw_tx = Transaction {
            tx_id: 2,
            client_id: 1,
            tx_type: TransactionType::Withdrawal,
            amount: Some(2.0),
            disputed: false,
            resolved: false,
        };

        client.deposit(&deposit_tx)?;

        let result = client.withdraw(&withdraw_tx);
        assert_eq!(result, Err(EngineError::InsufficientFunds));

        withdraw_tx.amount = Some(1.0);
        client.withdraw(&withdraw_tx)?;

        Ok(())
    }

    #[test]
    fn test_dispute() -> Result<(), EngineError> {
        let mut client = Client::new(1);

        let mut deposit_tx = Transaction {
            tx_id: 1,
            client_id: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(1.0),
            disputed: false,
            resolved: false,
        };

        let mut dispute_tx = Transaction {
            tx_id: 1,
            client_id: 1,
            tx_type: TransactionType::Dispute,
            amount: None,
            disputed: false,
            resolved: false,
        };

        client.deposit(&deposit_tx)?;
        client.dispute(&dispute_tx)?;

        assert_eq!(client.summary.available, 0.0);
        assert_eq!(client.summary.held, 1.0);
        assert_eq!(client.summary.total, 1.0);
        assert_eq!(client.summary.locked, false);
        assert_eq!(client.tx_map.get(&1).unwrap().disputed, true);

        let result = client.dispute(&dispute_tx);

        assert_eq!(
            result,
            Err(EngineError::DisputeError(format!(
                "TX {} is already disputed",
                dispute_tx.tx_id
            )))
        );

        deposit_tx.tx_id = 2;
        dispute_tx.tx_id = 2;

        let withdraw_tx = Transaction {
            tx_id: 3,
            client_id: 1,
            tx_type: TransactionType::Withdrawal,
            amount: Some(1.0),
            disputed: false,
            resolved: false,
        };

        client.deposit(&deposit_tx)?;
        client.withdraw(&withdraw_tx)?;

        // What happens in this scenario?
        // When a dispute happens but the funds have already been withdrawn, should the account
        // be locked?

        let result = client.dispute(&dispute_tx);

        assert_eq!(client.summary.available, 0.0);
        assert_eq!(client.summary.held, 1.0);
        assert_eq!(client.summary.total, 1.0);
        assert_eq!(client.summary.locked, false);
        assert_eq!(client.tx_map.get(&2).unwrap().disputed, false);

        assert_eq!(result, Err(EngineError::InsufficientFunds));

        Ok(())
    }

    #[test]
    fn test_resolve() -> Result<(), EngineError> {
        let mut client = Client::new(1);

        let deposit_tx = Transaction {
            tx_id: 1,
            client_id: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(1.0),
            disputed: false,
            resolved: false,
        };

        let withdraw_tx = Transaction {
            tx_id: 2,
            client_id: 1,
            tx_type: TransactionType::Withdrawal,
            amount: Some(0.05),
            disputed: false,
            resolved: false,
        };

        let deposit_tx2 = Transaction {
            tx_id: 3,
            client_id: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(1.0),
            disputed: false,
            resolved: false,
        };

        let dispute_tx = Transaction {
            tx_id: 3,
            client_id: 1,
            tx_type: TransactionType::Dispute,
            amount: None,
            disputed: false,
            resolved: false,
        };

        let mut resolve_tx = Transaction {
            tx_id: 3,
            client_id: 1,
            tx_type: TransactionType::Resolve,
            amount: None,
            disputed: false,
            resolved: false,
        };

        client.deposit(&deposit_tx)?;
        client.withdraw(&withdraw_tx)?;

        assert_eq!(client.summary.available, 0.95);
        assert_eq!(client.summary.held, 0.0);
        assert_eq!(client.summary.total, 0.950);
        assert_eq!(client.summary.locked, false);

        client.deposit(&deposit_tx2)?;
        client.dispute(&dispute_tx)?;

        assert_eq!(client.summary.available, 0.95);
        assert_eq!(client.summary.held, 1.0);
        assert_eq!(client.summary.total, 1.95);
        assert_eq!(client.summary.locked, false);
        assert_eq!(client.tx_map.get(&3).unwrap().disputed, true);
        assert_eq!(client.tx_map.get(&3).unwrap().resolved, false);

        client.resolve(&resolve_tx)?;

        assert_eq!(client.summary.available, 1.95);
        assert_eq!(client.summary.held, 0.0);
        assert_eq!(client.summary.total, 1.95);
        assert_eq!(client.summary.locked, false);
        assert_eq!(client.tx_map.get(&3).unwrap().disputed, true);
        assert_eq!(client.tx_map.get(&3).unwrap().resolved, true);

        let result = client.resolve(&resolve_tx);

        assert_eq!(
            result,
            Err(EngineError::ResolveError(format!(
                "TX {} is already resolved",
                resolve_tx.tx_id
            )))
        );

        resolve_tx.tx_id = 1;
        let result = client.resolve(&resolve_tx);

        assert_eq!(
            result,
            Err(EngineError::ResolveError(format!(
                "TX {} is undisputed",
                resolve_tx.tx_id
            )))
        );

        assert_eq!(client.tx_map.get(&1).unwrap().disputed, false);
        assert_eq!(client.tx_map.get(&1).unwrap().resolved, false);

        Ok(())
    }

    #[test]
    fn test_chargeback() -> Result<(), EngineError> {
        let mut client = Client::new(1);

        let deposit_tx = Transaction {
            tx_id: 1,
            client_id: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(1.0),
            disputed: false,
            resolved: false,
        };

        let withdraw_tx = Transaction {
            tx_id: 2,
            client_id: 1,
            tx_type: TransactionType::Withdrawal,
            amount: Some(0.05),
            disputed: false,
            resolved: false,
        };

        let deposit_tx2 = Transaction {
            tx_id: 3,
            client_id: 1,
            tx_type: TransactionType::Deposit,
            amount: Some(1.0),
            disputed: false,
            resolved: false,
        };

        let mut dispute_tx = Transaction {
            tx_id: 3,
            client_id: 1,
            tx_type: TransactionType::Dispute,
            amount: None,
            disputed: false,
            resolved: false,
        };

        let resolve_tx = Transaction {
            tx_id: 3,
            client_id: 1,
            tx_type: TransactionType::Resolve,
            amount: None,
            disputed: false,
            resolved: false,
        };

        let mut chargeback_tx = Transaction {
            tx_id: 3,
            client_id: 1,
            tx_type: TransactionType::ChargeBack,
            amount: None,
            disputed: false,
            resolved: false,
        };

        client.deposit(&deposit_tx)?;
        client.withdraw(&withdraw_tx)?;

        client.deposit(&deposit_tx2)?;
        client.dispute(&dispute_tx)?;

        assert_eq!(client.summary.available, 0.95);
        assert_eq!(client.summary.held, 1.0);
        assert_eq!(client.summary.total, 1.95);
        assert_eq!(client.summary.locked, false);
        assert_eq!(client.tx_map.get(&3).unwrap().disputed, true);
        assert_eq!(client.tx_map.get(&3).unwrap().resolved, false);

        client.resolve(&resolve_tx)?;

        assert_eq!(client.summary.available, 1.95);
        assert_eq!(client.summary.held, 0.0);
        assert_eq!(client.summary.total, 1.95);
        assert_eq!(client.summary.locked, false);
        assert_eq!(client.tx_map.get(&3).unwrap().disputed, true);
        assert_eq!(client.tx_map.get(&3).unwrap().resolved, true);

        let result = client.charge_back(&chargeback_tx);

        assert_eq!(
            result,
            Err(EngineError::ChargeBackError(format!(
                "TX {} is already resolved",
                chargeback_tx.tx_id
            )))
        );

        chargeback_tx.tx_id = 1;
        let result = client.charge_back(&chargeback_tx);

        assert_eq!(
            result,
            Err(EngineError::ChargeBackError(format!(
                "TX {} is undisputed",
                chargeback_tx.tx_id
            )))
        );

        dispute_tx.tx_id = 1;
        client.dispute(&dispute_tx)?;
        client.charge_back(&chargeback_tx)?;

        assert_eq!(client.summary.available, 0.95);
        assert_eq!(client.summary.held, 0.0);
        assert_eq!(client.summary.total, 0.95);
        assert_eq!(client.tx_map.get(&3).unwrap().disputed, true);
        assert_eq!(client.tx_map.get(&3).unwrap().resolved, true);
        assert_eq!(client.tx_map.get(&1).unwrap().disputed, true);
        assert_eq!(client.tx_map.get(&1).unwrap().resolved, false);
        assert_eq!(client.summary.locked, true);

        Ok(())
    }
}
