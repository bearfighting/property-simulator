use property_simulation_account::{
    Account, AccountCatalogue, AccountClass, AccountCode, AccountError, AccountRole,
};

/// Stable account roles used by the initialization screen and initial Slice set.
pub const PROPERTY_ROLE: &str = "property";
pub const CASH_ROLE: &str = "cash";
pub const MORTGAGE_ROLE: &str = "mortgage";
pub const EQUITY_ROLE: &str = "opening_equity";
pub const RENTAL_INCOME_ROLE: &str = "rental_income";
pub const OPERATING_EXPENSE_ROLE: &str = "operating_expense";
pub const REPAIR_EXPENSE_ROLE: &str = "repair_expense";
pub const INTEREST_EXPENSE_ROLE: &str = "interest_expense";

/// Build the fixed account catalogue used for a newly initialized simulation.
///
/// This initial chart is intentionally incomplete: expense/report categories
/// and the posting roles required by every Slice have not been fully decided.
/// Callers must not treat it as a complete accounting profile. Complete it when
/// the initial Slice posting accounts and report classifications are specified.
pub fn initialization_account_catalogue() -> Result<AccountCatalogue, AccountError> {
    let definitions = [
        (1000, "Property", AccountClass::Asset, PROPERTY_ROLE),
        (1010, "Cash", AccountClass::Asset, CASH_ROLE),
        (
            2000,
            "Mortgage payable",
            AccountClass::Liability,
            MORTGAGE_ROLE,
        ),
        (3000, "Opening equity", AccountClass::Equity, EQUITY_ROLE),
        (
            4000,
            "Rental income",
            AccountClass::Income,
            RENTAL_INCOME_ROLE,
        ),
        (
            5000,
            "Operating expenses",
            AccountClass::Expense,
            OPERATING_EXPENSE_ROLE,
        ),
        (
            5010,
            "Repair expenses",
            AccountClass::Expense,
            REPAIR_EXPENSE_ROLE,
        ),
        (
            5020,
            "Interest expenses",
            AccountClass::Expense,
            INTEREST_EXPENSE_ROLE,
        ),
    ];

    AccountCatalogue::new(
        definitions
            .into_iter()
            .map(|(code, name, class, role)| {
                Account::new(
                    AccountCode::new(code)?,
                    name,
                    class,
                    AccountRole::new(role)?,
                )
            })
            .collect::<Result<Vec<_>, AccountError>>()?,
    )
}
