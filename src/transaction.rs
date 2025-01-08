use serde::Deserialize;
use std::fmt::Display;

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
            TransactionType::Deposit => write!(f, "deposit"),
            TransactionType::Withdrawal => write!(f, "withdrawal"),
            TransactionType::Dispute => write!(f, "dispute"),
            TransactionType::Resolve => write!(f, "resolve"),
            TransactionType::ChargeBack => write!(f, "chargeback"),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Transaction {
    pub id: u32,
    pub tx_type: TransactionType,
    pub client: u16,
    pub amount: Option<f64>,
    pub disputed: bool,
    pub resolved: bool,
}
