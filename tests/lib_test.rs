use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tx_proc::*;

// todo try using a single struct as argument to make call sites more readable?
fn assert_account_data(
    proc: &TxProc,
    client_id: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
) {
    let summary = proc
        .account_summary(client_id)
        .expect(&format!("summary for client {client_id} not found"));
    assert_eq!(summary.available_amount(), available);
    assert_eq!(summary.held_amount(), held);
    assert_eq!(summary.total_amount(), total);
    assert_eq!(summary.is_locked(), locked);
}

#[test]
fn repeated_transaction() {
    const CLIENT_1: u16 = 1;
    const CLIENT_2: u16 = 2;
    const DEPOSIT_1: u32 = 3;

    let proc = &mut TxProc::default();

    // deposit
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, DEPOSIT_1),
        dec!(3),
    ));

    // available & total are increased
    assert_account_data(proc, CLIENT_1, dec!(3), dec!(0), dec!(3), false);

    // repeated transaction on same client
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, DEPOSIT_1),
        dec!(3),
    ));

    // amounts do not change
    assert_account_data(proc, CLIENT_1, dec!(3), dec!(0), dec!(3), false);

    // repeated transaction on different client
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_2, DEPOSIT_1),
        dec!(3),
    ));

    // no amount deposited for client 2
    assert_account_data(proc, CLIENT_2, dec!(0), dec!(0), dec!(0), false);

    // no amount change for client 1
    assert_account_data(proc, CLIENT_1, dec!(3), dec!(0), dec!(3), false);

    assert_eq!(proc.summary_iterator().count(), 2);
}

#[test]
fn multiple_clients() {
    const CLIENT_1: u16 = 1;
    const CLIENT_2: u16 = 2;

    let proc = &mut TxProc::default();

    // deposit on client 1
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, 2),
        dec!(3),
    ));

    // available & total are increased for client 1
    assert_account_data(proc, CLIENT_1, dec!(3), dec!(0), dec!(3), false);

    // deposit on client 2
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_2, 3),
        dec!(4),
    ));

    // available & total are increased for client 2
    assert_account_data(proc, CLIENT_2, dec!(4), dec!(0), dec!(4), false);

    // no amount change for client 1
    assert_account_data(proc, CLIENT_1, dec!(3), dec!(0), dec!(3), false);

    assert_eq!(proc.summary_iterator().count(), 2);
}

#[test]
fn deposit() {
    const CLIENT_1: u16 = 1;

    let proc = &mut TxProc::default();

    // negative deposit amount
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, 2),
        dec!(-3),
    ));

    // amounts do not change
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // valid deposit
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, 2),
        dec!(3),
    ));

    // available & total are increased
    assert_account_data(proc, CLIENT_1, dec!(3), dec!(0), dec!(3), false);
}

#[test]
fn withdrawals() {
    const CLIENT_1: u16 = 1;

    let proc = &mut TxProc::default();

    // deposit
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, 2),
        dec!(3),
    ));

    // available & total are increased
    assert_account_data(proc, CLIENT_1, dec!(3), dec!(0), dec!(3), false);

    // withdrawal with insufficient available amount
    proc.submit_tx_record(TxRecord::WITHDRAWAL(
        TxRecordMetadata::new(CLIENT_1, 3),
        dec!(4),
    ));

    // amounts do not change
    assert_account_data(proc, CLIENT_1, dec!(3), dec!(0), dec!(3), false);

    // invalid negative withdrawal
    proc.submit_tx_record(TxRecord::WITHDRAWAL(
        TxRecordMetadata::new(CLIENT_1, 4),
        dec!(-1),
    ));

    // amounts do not change
    assert_account_data(proc, CLIENT_1, dec!(3), dec!(0), dec!(3), false);

    // valid withdrawal
    proc.submit_tx_record(TxRecord::WITHDRAWAL(
        TxRecordMetadata::new(CLIENT_1, 4),
        dec!(3),
    ));

    // available & total are decreased
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    assert_eq!(proc.summary_iterator().count(), 1);
}

#[test]
fn dispute() {
    const CLIENT_1: u16 = 1;
    const CLIENT_2: u16 = 2;

    const NON_EXISTENT_TX: u32 = 2;
    const DEPOSIT_1: u32 = 3;
    const WITHDRAWAL_1: u32 = 4;

    let proc = &mut TxProc::default();

    // dispute a non-existent transaction
    proc.submit_tx_record(TxRecord::DISPUTE(TxRecordMetadata::new(
        CLIENT_1,
        NON_EXISTENT_TX,
    )));

    // no change in amounts
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // deposit
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, DEPOSIT_1),
        dec!(2),
    ));

    // available & total are increased
    assert_account_data(proc, CLIENT_1, dec!(2), dec!(0), dec!(2), false);

    // withdrawal
    proc.submit_tx_record(TxRecord::WITHDRAWAL(
        TxRecordMetadata::new(CLIENT_1, WITHDRAWAL_1),
        dec!(2),
    ));

    // available & total are decreased
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // dispute the withdrawal
    proc.submit_tx_record(TxRecord::DISPUTE(TxRecordMetadata::new(
        CLIENT_1,
        WITHDRAWAL_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // dispute the deposit but with wrong client 2
    proc.submit_tx_record(TxRecord::DISPUTE(TxRecordMetadata::new(
        CLIENT_2, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);
    assert_account_data(proc, CLIENT_2, dec!(0), dec!(0), dec!(0), false);

    // dispute the deposit
    proc.submit_tx_record(TxRecord::DISPUTE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // available is decreased by the deposit amount
    // held is increased by the deposit amount
    // total does not change
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(2), dec!(0), false);

    // duplicated dispute
    proc.submit_tx_record(TxRecord::DISPUTE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(2), dec!(0), false);
}

#[test]
fn resolve() {
    const CLIENT_1: u16 = 1;
    const CLIENT_2: u16 = 2;
    const DEPOSIT_1: u32 = 3;
    const WITHDRAWAL_1: u32 = 4;

    let proc = &mut TxProc::default();

    // resolve a non-existent transaction
    proc.submit_tx_record(TxRecord::RESOLVE(TxRecordMetadata::new(CLIENT_1, 2)));

    // no change in amounts
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // deposit
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, DEPOSIT_1),
        dec!(2),
    ));

    // available & total are increased
    assert_account_data(proc, CLIENT_1, dec!(2), dec!(0), dec!(2), false);

    // withdrawal
    proc.submit_tx_record(TxRecord::WITHDRAWAL(
        TxRecordMetadata::new(CLIENT_1, WITHDRAWAL_1),
        dec!(2),
    ));

    // available & total are decreased
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // resolve the withdrawal
    proc.submit_tx_record(TxRecord::RESOLVE(TxRecordMetadata::new(
        CLIENT_1,
        WITHDRAWAL_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // resolve the non-disputed deposit
    proc.submit_tx_record(TxRecord::RESOLVE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // dispute the deposit
    proc.submit_tx_record(TxRecord::DISPUTE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // available is decreased by the deposit amount
    // held is increased by the deposit amount
    // total does not change
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(2), dec!(0), false);

    // resolve the deposit but with wrong client 2
    proc.submit_tx_record(TxRecord::RESOLVE(TxRecordMetadata::new(
        CLIENT_2, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(2), dec!(0), false);
    assert_account_data(proc, CLIENT_2, dec!(0), dec!(0), dec!(0), false);

    // resolve the disputed deposit
    proc.submit_tx_record(TxRecord::RESOLVE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // available is increased by the deposit amount
    // held is decreased by the deposit amount
    // total does not change
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // duplicated resolve
    proc.submit_tx_record(TxRecord::RESOLVE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // dispute a second time the deposit
    proc.submit_tx_record(TxRecord::DISPUTE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // available is decreased by the deposit amount
    // held is increased by the deposit amount
    // total does not change
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(2), dec!(0), false);

    // resolve a second time the disputed deposit
    proc.submit_tx_record(TxRecord::RESOLVE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // available is increased by the deposit amount
    // held is decreased by the deposit amount
    // total does not change
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);
}

#[test]
fn chargeback() {
    const CLIENT_1: u16 = 1;
    const CLIENT_2: u16 = 2;
    const DEPOSIT_1: u32 = 3;
    const DEPOSIT_2: u32 = 5;
    const WITHDRAWAL_1: u32 = 4;
    const WITHDRAWAL_2: u32 = 6;

    let proc = &mut TxProc::default();

    // chargeback a non-existent transaction
    proc.submit_tx_record(TxRecord::CHARGEBACK(TxRecordMetadata::new(CLIENT_1, 2)));

    // no change in amounts
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // deposit
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, DEPOSIT_1),
        dec!(2),
    ));

    // available & total are increased
    assert_account_data(proc, CLIENT_1, dec!(2), dec!(0), dec!(2), false);

    // withdrawal
    proc.submit_tx_record(TxRecord::WITHDRAWAL(
        TxRecordMetadata::new(CLIENT_1, WITHDRAWAL_1),
        dec!(2),
    ));

    // available & total are decreased
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // chargeback the withdrawal
    proc.submit_tx_record(TxRecord::CHARGEBACK(TxRecordMetadata::new(
        CLIENT_1,
        WITHDRAWAL_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // chargeback the non-disputed deposit
    proc.submit_tx_record(TxRecord::CHARGEBACK(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(0), dec!(0), dec!(0), false);

    // dispute the deposit
    proc.submit_tx_record(TxRecord::DISPUTE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // available is decreased by the deposit amount
    // held is increased by the deposit amount
    // total does not change
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(2), dec!(0), false);

    // chargeback the disputed deposit but on wrong client 2
    proc.submit_tx_record(TxRecord::CHARGEBACK(TxRecordMetadata::new(
        CLIENT_2, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(2), dec!(0), false);
    assert_account_data(proc, CLIENT_2, dec!(0), dec!(0), dec!(0), false);

    // chargeback the disputed deposit
    proc.submit_tx_record(TxRecord::CHARGEBACK(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // available does not change
    // held is decreased by the deposit amount
    // total is decreased by the deposit amount
    // account is locked
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(0), dec!(-2), true);

    // duplicated chargeback
    proc.submit_tx_record(TxRecord::CHARGEBACK(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(0), dec!(-2), true);

    // try resolve the chargeback deposit
    proc.submit_tx_record(TxRecord::RESOLVE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(0), dec!(-2), true);

    // try dispute again on chargeback deposit
    proc.submit_tx_record(TxRecord::DISPUTE(TxRecordMetadata::new(
        CLIENT_1, DEPOSIT_1,
    )));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(0), dec!(-2), true);

    // deposit on locked account
    proc.submit_tx_record(TxRecord::DEPOSIT(
        TxRecordMetadata::new(CLIENT_1, DEPOSIT_2),
        dec!(2),
    ));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(0), dec!(-2), true);

    // withdrawal on locked account
    proc.submit_tx_record(TxRecord::WITHDRAWAL(
        TxRecordMetadata::new(CLIENT_1, WITHDRAWAL_2),
        dec!(2),
    ));

    // nothing happens
    assert_account_data(proc, CLIENT_1, dec!(-2), dec!(0), dec!(-2), true);
}
