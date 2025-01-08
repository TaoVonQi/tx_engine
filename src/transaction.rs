use serde::Deserialize;
use std::fmt::Display;

use crate::EngineError;

const DEPOSIT: &str = "deposit";
const WITHDRAWAL: &str = "withdrawal";
const DISPUTE: &str = "dispute";
const RESOLVE: &str = "resolve";
const CHARGE_BACK: &str = "chargeback";

#[derive(Debug, PartialEq, Deserialize, Clone)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    ChargeBack,
}

impl Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionType::Deposit => write!(f, "{DEPOSIT}"),
            TransactionType::Withdrawal => write!(f, "{WITHDRAWAL}"),
            TransactionType::Dispute => write!(f, "{DISPUTE}"),
            TransactionType::Resolve => write!(f, "{RESOLVE}"),
            TransactionType::ChargeBack => write!(f, "{CHARGE_BACK}"),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    pub tx_type: String,

    #[serde(rename = "client")]
    pub client_id: u16,

    #[serde(rename = "tx")]
    pub tx_id: u32,

    #[serde(rename = "amount")]
    pub amount: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Transaction {
    pub tx_id: u32,
    pub client_id: u16,
    pub tx_type: TransactionType,
    pub amount: Option<f64>,
    pub disputed: bool,
    pub resolved: bool,
}

impl TryFrom<TransactionRecord> for Transaction {
    type Error = EngineError;

    fn try_from(value: TransactionRecord) -> Result<Self, Self::Error> {
        if let Some(tx_type) = match value.tx_type.as_str() {
            DEPOSIT => Some(TransactionType::Deposit),
            WITHDRAWAL => Some(TransactionType::Withdrawal),
            DISPUTE => Some(TransactionType::Dispute),
            RESOLVE => Some(TransactionType::Resolve),
            CHARGE_BACK => Some(TransactionType::ChargeBack),
            _ => None,
        } {
            Ok(Transaction {
                tx_type,
                client_id: value.client_id,
                tx_id: value.tx_id,
                amount: value.amount,
                disputed: false,
                resolved: false,
            })
        } else {
            Err(EngineError::InvalidTransaction(format!(
                "TX ID: {}, Type: {}",
                value.tx_id, value.tx_type
            )))
        }
    }
}
