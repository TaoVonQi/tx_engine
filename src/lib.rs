use client::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod client;
pub mod transaction;

pub type EngineState = Arc<AppState>;

#[derive(Debug)]
enum EngineError {
    InsufficientFunds,
    InvalidTransaction,
    AccountLocked,
    DisputeError(String),
    ResolveError(String),
    ChargeBackError,
    OtherError(String),
}

pub struct AppState {
    pub client_map: RwLock<HashMap<u16, Client>>, // map: client_id -> client
}
