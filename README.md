# tx_engine

This is a simple toy payments engine that reads a series of transactions from CSV files, updates client accounts, handles deposit, withdraws, disputes and chargebacks, and then outputs the state of clients accounts as a CSV to stdout.

The project is designed to handle events that trigger when a CSV file becomes available. Each event will lock the client account's state as it finishes processing the batch of transactions within the CSV file.

The CSV file is not loaded at once, instead transactions within a CSV file are streamed in chronological order. If any of the records failed to deserialize according to the transaction's schema, the program will abort inidcating the problem. Logical errors however, like insufficient funds will be output to stderr. And finally the final state of clients accounts will be output to stdout.

### The engine makes the following assumptions:

* Only deposit transactions can be disputed.
* A transaction can only be disputed once.
* A transaction can not be resolved without being previously disputed.
* A transaction can only be resolved once.
* A transaction can not be charged back without being previously disputed.
* A transaction can not be charged back if it is already resolved.
* If the account is locked, the engine will not accept any transactions for that account.


### Safety concern:

When a client deposits and withdraws funds before disputing. ie: when the available funds at the time of dispute is less than the disputed transaction's amount; an insufficient funds error will occur. see unit test: "test_dispute" 


### To run this engine with the provided sample data: 

```
cargo run -- sample.csv > output.csv
```
