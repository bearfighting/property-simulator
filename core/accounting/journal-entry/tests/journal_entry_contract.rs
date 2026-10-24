use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_journal_entry::{JournalEntry, JournalEntryError, JournalLine};
use property_simulation_money::{Currency, Money};
use property_simulation_month::Month;

fn money(value: &str) -> Money {
    Money::parse(value, Currency::from_code("CAD").unwrap()).unwrap()
}

#[test]
fn journal_lines_round_half_up_and_entries_must_balance() {
    let debit = JournalLine::new(
        AccountCode::new(100).unwrap(),
        DebitCredit::Debit,
        money("1.005"),
    )
    .unwrap();
    let credit = JournalLine::new(
        AccountCode::new(200).unwrap(),
        DebitCredit::Credit,
        money("1.00"),
    )
    .unwrap();

    assert_eq!(debit.amount(), money("1.01"));
    assert!(matches!(
        JournalEntry::new(Month::new(0), vec![debit, credit]),
        Err(JournalEntryError::Unbalanced { .. })
    ));
}

#[test]
fn journal_lines_reject_amounts_that_round_to_zero() {
    assert!(matches!(
        JournalLine::new(
            AccountCode::new(100).unwrap(),
            DebitCredit::Debit,
            money("0.004")
        ),
        Err(JournalEntryError::NonPositiveAmount)
    ));
}
