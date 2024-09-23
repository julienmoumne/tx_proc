use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

// todo evaluate whether using newtype structs improves safety without making the code harder to read
// e.g: HeldAmount(Decimal), AvailableAmount(Decimal), Locked(bool)
#[derive(Default)]
pub struct AccountSummary {
    held_amount: Decimal,
    available_amount: Decimal,
    locked: bool,
}

impl AccountSummary {
    pub fn total_amount(&self) -> Decimal {
        self.held_amount() + self.available_amount()
    }
    pub fn available_amount(&self) -> Decimal {
        self.available_amount
    }

    pub fn held_amount(&self) -> Decimal {
        self.held_amount
    }
    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

#[derive(Default)]
struct TxRecordFlags {
    disputed: bool,
    chargedback: bool,
}

#[derive(Default)]
pub struct TxProc {
    account_summaries: HashMap<u16, AccountSummary>,
    transactions: HashMap<u32, (TxRecord, TxRecordFlags)>,
}

impl TxProc {
    // todo if needed, it's possible to return a Result
    pub fn submit_tx_record(&mut self, record: TxRecord) {
        // because try_insert is nightly-only, see https://github.com/rust-lang/rust/issues/82766
        fn record_transaction_if_new(
            transactions: &mut HashMap<u32, (TxRecord, TxRecordFlags)>,
            record: TxRecord,
        ) -> Result<(), ()> {
            match transactions.get(&record.record_metadata().tx_id) {
                Some(_) => Err(()),
                None => {
                    transactions.insert(
                        record.record_metadata().tx_id,
                        (record, TxRecordFlags::default()),
                    );
                    Ok(())
                }
            }
        }

        let summary = self
            .account_summaries
            .entry(record.record_metadata().client_id)
            .or_default();

        if summary.is_locked() {
            return;
        }

        match record {
            TxRecord::DEPOSIT(_, amount) => {
                if amount < dec!(0) {
                    return;
                }

                match record_transaction_if_new(&mut self.transactions, record) {
                    Ok(_) => {}
                    // if the transaction has already been processed, we don't process it
                    Err(_) => return,
                }

                summary.available_amount += amount;
            }
            TxRecord::WITHDRAWAL(_, amount) => {
                if amount < dec!(0) {
                    return;
                }

                match record_transaction_if_new(&mut self.transactions, record) {
                    Ok(_) => {}
                    // if the transaction has already been processed, we don't process it
                    Err(_) => return,
                }

                if amount <= summary.available_amount {
                    summary.available_amount -= amount;
                }
            }
            TxRecord::DISPUTE(dispute_metadata) => {
                if let Some((TxRecord::DEPOSIT(deposit_metadata, amount), tx_record_flags)) =
                    self.transactions.get_mut(&dispute_metadata.tx_id)
                {
                    if deposit_metadata.client_id != dispute_metadata.client_id {
                        return;
                    }

                    if tx_record_flags.disputed || tx_record_flags.chargedback {
                        return;
                    }

                    tx_record_flags.disputed = true;

                    summary.available_amount -= *amount;
                    summary.held_amount += *amount;
                }
            }
            TxRecord::RESOLVE(resolve_metadata) => {
                if let Some((TxRecord::DEPOSIT(deposit_metadata, amount), tx_record_flags)) =
                    self.transactions.get_mut(&resolve_metadata.tx_id)
                {
                    if deposit_metadata.client_id != resolve_metadata.client_id {
                        return;
                    }

                    if !tx_record_flags.disputed || tx_record_flags.chargedback {
                        return;
                    }

                    tx_record_flags.disputed = false;

                    summary.available_amount += *amount;
                    summary.held_amount -= *amount;
                }
            }
            TxRecord::CHARGEBACK(chargeback_metadata) => {
                if let Some((TxRecord::DEPOSIT(deposit_metadata, amount), tx_record_flags)) =
                    self.transactions.get_mut(&chargeback_metadata.tx_id)
                {
                    if deposit_metadata.client_id != chargeback_metadata.client_id {
                        return;
                    }

                    if !tx_record_flags.disputed || tx_record_flags.chargedback {
                        return;
                    }

                    tx_record_flags.disputed = false;
                    tx_record_flags.chargedback = true;

                    summary.held_amount -= *amount;
                    summary.locked = true;
                }
            }
        }
    }

    pub fn account_summary(&self, client_id: u16) -> Option<&AccountSummary> {
        self.account_summaries.get(&client_id)
    }

    pub fn summary_iterator(&self) -> impl Iterator<Item = (&u16, &AccountSummary)> {
        self.account_summaries.iter()
    }
}

pub enum TxRecord {
    DEPOSIT(TxRecordMetadata, Decimal),
    WITHDRAWAL(TxRecordMetadata, Decimal),
    DISPUTE(TxRecordMetadata),
    RESOLVE(TxRecordMetadata),
    CHARGEBACK(TxRecordMetadata),
}

impl TxRecord {
    pub fn record_metadata(&self) -> &TxRecordMetadata {
        match self {
            TxRecord::DEPOSIT(metadata, _) => metadata,
            TxRecord::WITHDRAWAL(metadata, _) => metadata,
            TxRecord::DISPUTE(metadata) => metadata,
            TxRecord::RESOLVE(metadata) => metadata,
            TxRecord::CHARGEBACK(metadata) => metadata,
        }
    }
}

pub struct TxRecordMetadata {
    client_id: u16,
    tx_id: u32,
}

impl TxRecordMetadata {
    pub fn new(client_id: u16, tx_id: u32) -> TxRecordMetadata {
        TxRecordMetadata { client_id, tx_id }
    }
}
