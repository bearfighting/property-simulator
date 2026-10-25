use super::validation::{apply_selected_lines, validate_all, validate_period, zero};
use crate::{IncomeStatement, IncomeStatementLine, ReportingError, ReportingInput};
use property_simulation_account::{AccountClass, DebitCredit};
use property_simulation_month::Month;
use std::collections::BTreeMap;

/// Derive income and expense account activity over the inclusive month interval.
pub fn income_statement(
    input: ReportingInput<'_>,
    start: Month,
    end: Month,
) -> Result<IncomeStatement, ReportingError> {
    validate_period(start, end)?;
    validate_all(input)?;

    let zero = zero(input)?;
    let mut activity = BTreeMap::new();
    for account in input.accounts().iter() {
        if matches!(
            account.class(),
            AccountClass::Income | AccountClass::Expense
        ) {
            activity.insert(account.code(), zero);
        }
    }

    for entry in input.journal() {
        let month = entry.month();
        if month < start || month > end {
            continue;
        }
        apply_selected_lines(input, &mut activity, entry, |class| {
            matches!(class, AccountClass::Income | AccountClass::Expense)
        })?;
    }

    let mut income = Vec::new();
    let mut expenses = Vec::new();
    let mut income_total = zero;
    let mut expense_total = zero;

    let mut accounts = input.accounts().iter().collect::<Vec<_>>();
    accounts.sort_by_key(|account| account.code());

    for account in accounts {
        let Some(net_debit) = activity.get(&account.code()).copied() else {
            continue;
        };
        let amount = match account.class().normal_balance() {
            DebitCredit::Debit => net_debit,
            DebitCredit::Credit => net_debit.checked_neg()?,
        };
        let line =
            IncomeStatementLine::new(account.code(), account.class().normal_balance(), amount);

        match account.class() {
            AccountClass::Income => {
                income_total = income_total.checked_add(amount)?;
                income.push(line);
            }
            AccountClass::Expense => {
                expense_total = expense_total.checked_add(amount)?;
                expenses.push(line);
            }
            _ => {}
        }
    }

    let net_income = income_total.checked_sub(expense_total)?;
    Ok(IncomeStatement::new(
        start,
        end,
        income,
        expenses,
        income_total,
        expense_total,
        net_income,
    ))
}
