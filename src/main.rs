use std::{env, io};

use anyhow::Context;
use derive_more::Display;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::engine::Engine;

mod engine;

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
struct ClientId(u16);

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
struct TransactionId(u32);

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
struct TransactionRecord {
    r#type: TransactionType,
    client: ClientId,
    tx: TransactionId,
    amount: Option<Decimal>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AccountRecord {
    client: ClientId,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

fn main() -> Result<(), anyhow::Error> {
    let mut args = env::args_os();
    _ = args.next();
    let path = args
        .next()
        .context("missing argument: path to transactions")?;

    let mut engine = Engine::new();

    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)?;

    for transaction_res in csv_reader.deserialize::<TransactionRecord>() {
        let transaction = transaction_res?;
        println!("{transaction:?}");

        let res = match transaction.r#type {
            TransactionType::Deposit => engine.deposit(
                transaction.client,
                transaction.tx,
                transaction.amount.context("missing amount")?,
            ),
            TransactionType::Withdrawal => engine.withdraw(
                transaction.client,
                transaction.tx,
                transaction.amount.context("missing amount")?,
            ),
            TransactionType::Dispute => engine.dispute(transaction.client, transaction.tx),
            TransactionType::Resolve => engine.resolve(transaction.client, transaction.tx),
            TransactionType::Chargeback => engine.chargeback(transaction.client, transaction.tx),
        };

        match res {
            Ok(()) => (),

            Err(
                fatal @ (engine::Error::ClientMismatch { .. }
                | engine::Error::DuplicateTransactionId(_)),
            ) => return Err(fatal.into()),

            Err(nonfatal) => eprintln!("warning: {nonfatal}"),
        }
    }

    let mut csv_writer = csv::Writer::from_writer(io::stdout().lock());

    for (&client, account) in engine.accounts() {
        let account_record = AccountRecord {
            client,
            available: account.available(),
            held: account.held,
            total: account.total,
            locked: account.locked,
        };

        csv_writer.serialize(account_record)?;
    }

    Ok(())
}
