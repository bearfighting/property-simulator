use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_journal_entry::{JournalEntry, JournalLine};
use property_simulation_money::{Currency, Money};
use property_simulation_month::Month;
use property_simulation_slice_catalogue::{SliceKind, SliceResult};
use property_simulation_snapshot::{Snapshot, SnapshotError};

#[test]
fn snapshot_rejects_journal_entries_from_another_month() {
    let currency = Currency::from_code("CAD").unwrap();
    let cash = AccountCode::new(100).unwrap();
    let income = AccountCode::new(400).unwrap();
    let amount = Money::parse("10", currency).unwrap();
    let entry = JournalEntry::new(
        Month::new(0),
        vec![
            JournalLine::new(cash, DebitCredit::Debit, amount).unwrap(),
            JournalLine::new(income, DebitCredit::Credit, amount).unwrap(),
        ],
    )
    .unwrap();
    let result = SliceResult::new(SliceKind::Rental, Vec::new(), Vec::new(), vec![entry]).unwrap();

    assert!(matches!(
        Snapshot::new(Month::new(1), vec![result]),
        Err(SnapshotError::JournalEntryMonthMismatch {
            snapshot_month,
            entry_month,
        }) if snapshot_month == Month::new(1) && entry_month == Month::new(0)
    ));
}
