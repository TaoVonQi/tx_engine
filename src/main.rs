use csv::{Reader, StringRecord};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tx_engine::client::Client;
use tx_engine::{
    transaction::{Transaction, TransactionRecord, TransactionType},
    AppState, EngineError, EngineState,
};

async fn process_csv(path: String, state: EngineState) -> Result<(), EngineError> {
    let mut rdr = Reader::from_path(path)
        .map_err(|_| EngineError::CsvFileError(String::from("Invalid CSV file")))?;

    let mut client_map = state.client_map.write().await;

    for result in rdr.records() {
        let record = result.map_err(|e| {
            EngineError::InvalidTransaction(format!(
                "Failed to fetch transaction record. {}",
                e.to_string()
            ))
        })?;

        let trimmed_record: StringRecord = record.into_iter().map(|field| field.trim()).collect();

        // Making sure the entire file is valid.
        // Stop processing the rest of the records if any record failed to deserialize.
        let record: TransactionRecord = trimmed_record.deserialize(None).map_err(|e| {
            EngineError::InvalidTransaction(format!(
                "Failed to deserialize transaction record. {}",
                e.to_string()
            ))
        })?;

        let transaction = Transaction::try_from(record)?;

        // Insert a default client if none exists.
        let client = client_map
            .entry(transaction.client_id)
            .or_insert(Client::new(transaction.client_id));

        // Print any transaction error and process the remaining transactions.
        if let Err(e) = match transaction.tx_type {
            TransactionType::Deposit => client.deposit(&transaction),
            TransactionType::Withdrawal => client.withdraw(&transaction),
            TransactionType::Dispute => client.dispute(&transaction),
            TransactionType::Resolve => client.resolve(&transaction),
            TransactionType::ChargeBack => client.charge_back(&transaction),
        } {
            println!("{}", e);
        }
    }

    Ok(())
}

pub async fn on_process_csv(
    mut process_csv_reciever: mpsc::UnboundedReceiver<String>,
    state: EngineState,
) -> Result<(), EngineError> {
    loop {
        if let Some(path) = process_csv_reciever.recv().await {
            process_csv(path, state.clone()).await?;

            let client_map = state.client_map.read().await;

            for client in client_map.values() {
                println!("{}", client);
            }
            break;
        } else {
            println!("Warning: failed to handle csv processing event")
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), EngineError> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        let (process_csv_sender, process_csv_receiver) = mpsc::unbounded_channel::<String>();

        let state = Arc::new(AppState {
            client_map: RwLock::new(HashMap::new()),
        });

        process_csv_sender.send(args[1].clone()).map_err(|e| {
            EngineError::OtherError(format!(
                "Failed to trigger processing event\n{}",
                e.to_string()
            ))
        })?;

        tokio::spawn(on_process_csv(process_csv_receiver, state.clone()))
            .await
            .map_err(|e| EngineError::OtherError(e.to_string()))??;
    } else {
        println!("This program expects the csv filepath");
    }

    Ok(())
}
