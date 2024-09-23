use csv::{ReaderBuilder, Trim};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self};

use tx_proc::*;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("invalid argument count {}", args.len())
    }

    let file_path = &args[1];

    let file = File::open(file_path).expect("failed to open file");

    let tx_proc = process_csv(file);

    write_account_csv_to_stdout(&tx_proc);
}

// todo if needed, it's possible to move CSV encode/decode into the library
fn process_csv(reader: impl Read) -> TxProc {
    #[derive(Deserialize)]
    struct CsvLineInput {
        r#type: String,
        client: u16,
        tx: u32,
        amount: Decimal,
    }

    // big files are not a problem because
    // the csv crate uses a BufReader of size 8 * (1 << 10) bytes = 8 KiB
    let mut rdr = ReaderBuilder::new().trim(Trim::All).from_reader(reader);

    let mut tx_proc = TxProc::default();

    for record in rdr.deserialize::<CsvLineInput>() {
        match record {
            Ok(csv_record) => {
                tx_proc.submit_tx_record(match csv_record.r#type.as_str() {
                    "deposit" => TxRecord::DEPOSIT(
                        TxRecordMetadata::new(csv_record.client, csv_record.tx),
                        csv_record.amount,
                    ),
                    "withdrawal" => TxRecord::WITHDRAWAL(
                        TxRecordMetadata::new(csv_record.client, csv_record.tx),
                        csv_record.amount,
                    ),
                    "dispute" => {
                        TxRecord::DISPUTE(TxRecordMetadata::new(csv_record.client, csv_record.tx))
                    }
                    "resolve" => {
                        TxRecord::RESOLVE(TxRecordMetadata::new(csv_record.client, csv_record.tx))
                    }
                    "chargeback" => TxRecord::CHARGEBACK(TxRecordMetadata::new(
                        csv_record.client,
                        csv_record.tx,
                    )),
                    _ => {
                        // print error and skip record
                        eprintln!("unknown type: {}", csv_record.r#type);
                        continue;
                    }
                })
            }
            Err(e) => {
                // print error and skip record
                eprintln!("csv error: {}", e);
            }
        }
    }

    tx_proc
}

fn write_account_csv_to_stdout(proc: &TxProc) {
    let mut wtr = csv::Writer::from_writer(io::stdout());

    #[derive(Serialize)]
    struct CsvLineOutput {
        client: u16,
        available: Decimal,
        held: Decimal,
        total: Decimal,
        locked: bool,
    }

    for (client_id, account_summary) in proc.summary_iterator() {
        match wtr.serialize(CsvLineOutput {
            client: *client_id,
            available: account_summary.available_amount(),
            held: account_summary.held_amount(),
            total: account_summary.total_amount(),
            locked: account_summary.is_locked(),
        }) {
            Ok(_) => {}
            // todo this case is not documented nor tested
            // todo is skipping the record ok or should the process completely fail ?
            Err(e) => {
                eprintln!("error while serializing record {}", e)
            }
        }
    }

    wtr.flush()
        .expect("error while trying to flush csv to stdout");
}
