use property_simulation_account::{
    Account, AccountCatalogue, AccountClass, AccountCode, AccountRole, DebitCredit,
};
use property_simulation_journal_entry::{JournalEntry, JournalLine};
use property_simulation_ledger::{Ledger, LedgerError, OpeningBalance};
use property_simulation_money::{Currency, Money};
use property_simulation_month::Month;

fn money(value: &str) -> Money {
    Money::parse(value, Currency::from_code("CAD").unwrap()).unwrap()
}

fn code(value: u32) -> AccountCode {
    AccountCode::new(value).unwrap()
}

fn accounts() -> AccountCatalogue {
    AccountCatalogue::new(vec![
        Account::new(
            code(100),
            "Cash",
            AccountClass::Asset,
            AccountRole::new("cash").unwrap(),
        )
        .unwrap(),
        Account::new(
            code(300),
            "Equity",
            AccountClass::Equity,
            AccountRole::new("equity").unwrap(),
        )
        .unwrap(),
        Account::new(
            code(500),
            "Repairs",
            AccountClass::Expense,
            AccountRole::new("repair-expense").unwrap(),
        )
        .unwrap(),
    ])
    .unwrap()
}

fn ledger() -> Ledger {
    Ledger::new(
        accounts(),
        Currency::from_code("CAD").unwrap(),
        vec![
            OpeningBalance::new(code(100), DebitCredit::Debit, money("100")).unwrap(),
            OpeningBalance::new(code(300), DebitCredit::Credit, money("100")).unwrap(),
        ],
    )
    .unwrap()
}

#[test]
fn failed_append_leaves_journal_and_balances_unchanged() {
    let mut ledger = ledger();
    let invalid_entry = JournalEntry::new(
        Month::new(0),
        vec![
            JournalLine::new(code(999), DebitCredit::Debit, money("20")).unwrap(),
            JournalLine::new(code(100), DebitCredit::Credit, money("20")).unwrap(),
        ],
    )
    .unwrap();

    assert!(matches!(
        ledger.append(invalid_entry),
        Err(LedgerError::UnknownAccount(account)) if account == code(999)
    ));
    assert!(ledger.journal().is_empty());
    assert_eq!(ledger.balance(code(100)).unwrap(), money("100"));
}

#[test]
fn valid_append_updates_signed_balances_and_preserves_month_order() {
    let mut ledger = ledger();
    let entry = JournalEntry::new(
        Month::new(0),
        vec![
            JournalLine::new(code(500), DebitCredit::Debit, money("20")).unwrap(),
            JournalLine::new(code(100), DebitCredit::Credit, money("20")).unwrap(),
        ],
    )
    .unwrap();
    ledger.append(entry).unwrap();

    assert_eq!(ledger.balance(code(100)).unwrap(), money("80"));
    assert_eq!(ledger.balance(code(500)).unwrap(), money("20"));
    assert_eq!(ledger.journal().len(), 1);
}
