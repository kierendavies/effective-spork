use std::env;

use anyhow::{Context, Error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Transaction {
    r#type: TransactionType,
    client: u16,
    tx: u32,
    amount: Decimal,
}

fn main() -> Result<(), Error> {
    let mut args = env::args_os();
    _ = args.next();
    let path = args
        .next()
        .context("missing argument: path to transactions")?;

    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)?;

    for transaction_res in csv_reader.deserialize::<Transaction>() {
        let transaction = transaction_res?;
        println!("{transaction:?}");
    }

    Ok(())
}
