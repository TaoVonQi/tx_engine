use csv::Reader;
use std::collections::HashMap;
use std::sync::Arc;
use std::{error::Error, io};
use tokio::sync::{mpsc, RwLock};
use tx_engine::{transaction::Transaction, AppState, EngineState};

fn example() -> Result<(), Box<dyn Error>> {
    let mut rdr = Reader::from_reader(io::stdin());
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let record: Transaction = result?;
        println!("{:?}", record);
    }
    Ok(())
}

pub async fn on_process_csv(
    mut process_csv_reciever: mpsc::UnboundedReceiver<String>,
    state: EngineState,
) {
    loop {
        let path = process_csv_reciever.recv().await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    let (process_csv_sender, process_csv_receiver) = mpsc::unbounded_channel::<String>();

    let state = Arc::new(AppState {
        client_map: RwLock::new(HashMap::new()),
    });

    tokio::spawn(on_process_csv(process_csv_receiver, state.clone()));
}
