use client::Client;
use std::{collections::HashMap, fmt::Display, sync::Arc};
use tokio::sync::RwLock;

pub mod client;
pub mod transaction;

pub type EngineState = Arc<AppState>;

#[derive(Debug, PartialEq)]
pub enum EngineError {
    InsufficientFunds,
    InvalidTransaction(String),
    DuplicateTransaction(String),
    AccountLocked,
    DisputeError(String),
    ResolveError(String),
    ChargeBackError(String),
    CsvFileError(String),
    OutputError(String),
    OtherError(String),
}

impl Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::InsufficientFunds => write!(f, "Insufficient funds"),
            EngineError::InvalidTransaction(msg) => write!(f, "Invalid Tx: {msg}"),
            EngineError::DuplicateTransaction(msg) => write!(f, "Duplicate Tx: {msg}"),
            EngineError::AccountLocked => write!(f, "Account Locked"),
            EngineError::DisputeError(msg) => write!(f, "Dispute Error: {msg}"),
            EngineError::ResolveError(msg) => write!(f, "Resolve Error: {msg}"),
            EngineError::ChargeBackError(msg) => write!(f, "Chargeback Error: {msg}"),
            EngineError::CsvFileError(msg) => write!(f, "CSV Error: {msg}"),
            EngineError::OutputError(msg) => write!(f, "Output Error: {msg}"),
            EngineError::OtherError(msg) => write!(f, "{msg}"),
        }
    }
}

pub struct AppState {
    pub client_map: RwLock<HashMap<u16, Client>>, // map: client_id -> client
}
