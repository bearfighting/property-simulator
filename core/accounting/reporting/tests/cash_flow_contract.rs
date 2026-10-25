use property_simulation_account::{
    Account, AccountCatalogue, AccountClass, AccountCode, AccountRole, DebitCredit,
};
use property_simulation_journal_entry::{JournalEntry, JournalLine};
use property_simulation_money::{Currency, Money};
use property_simulation_month::Month;
use property_simulation_reporting::{
    CashFlowAccountClassification, CashFlowCategory, ReportingInput, cash_flow,
};

fn code(value: u32) -> AccountCode {
    AccountCode::new(value).unwrap()
}

fn money(value: &str) -> Money {
    Money::parse(value, Currency::from_code("CAD").unwrap()).unwrap()
}

fn account(code: u32, name: &str, class: AccountClass, role: &str) -> Account {
    Account::new(
        self::code(code),
        name,
        class,
        AccountRole::new(role).unwrap(),
    )
    .unwrap()
}

#[test]
fn cash_flow_reconciles_classified_activity_and_excludes_opening_cash() {
    let currency = Currency::from_code("CAD").unwrap();
    let accounts = AccountCatalogue::new(vec![
        account(100, "Cash", AccountClass::Asset, "cash"),
        account(300, "Opening Equity", AccountClass::Equity, "equity"),
        account(400, "Rental Income", AccountClass::Income, "rental-income"),
        account(500, "Repairs", AccountClass::Expense, "repair-expense"),
    ])
    .unwrap();
    let journal = vec![
        JournalEntry::initial(vec![
            JournalLine::new(code(100), DebitCredit::Debit, money("100")).unwrap(),
            JournalLine::new(code(300), DebitCredit::Credit, money("100")).unwrap(),
        ])
        .unwrap(),
        JournalEntry::new(
            Month::new(1),
            vec![
                JournalLine::new(code(500), DebitCredit::Debit, money("20")).unwrap(),
                JournalLine::new(code(100), DebitCredit::Credit, money("20")).unwrap(),
            ],
        )
        .unwrap(),
        JournalEntry::new(
            Month::new(2),
            vec![
                JournalLine::new(code(100), DebitCredit::Debit, money("50")).unwrap(),
                JournalLine::new(code(400), DebitCredit::Credit, money("50")).unwrap(),
            ],
        )
        .unwrap(),
    ];
    let classifications = [
        CashFlowAccountClassification::new(code(400), CashFlowCategory::Operating, "Rental income"),
        CashFlowAccountClassification::new(code(500), CashFlowCategory::Operating, "Repairs"),
    ];

    let report = cash_flow(
        ReportingInput::new(&accounts, currency, &journal),
        &[code(100)],
        &classifications,
        Month::new(1),
        Month::new(2),
    )
    .unwrap();

    assert_eq!(report.total(), money("30"));
    assert_eq!(report.movements().len(), 2);
    assert_eq!(report.movements()[0].amount(), money("-20"));
    assert_eq!(report.movements()[1].amount(), money("50"));
    assert_eq!(report.category_totals().len(), 1);
    assert_eq!(
        report.category_totals()[0].category(),
        CashFlowCategory::Operating
    );
    assert_eq!(report.category_totals()[0].amount(), money("30"));
}
