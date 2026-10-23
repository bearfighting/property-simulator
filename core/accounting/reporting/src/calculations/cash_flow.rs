use super::validation::{validate_all, validate_period, zero};
use crate::{CashFlowMovement, CashFlowReport, ReportingError, ReportingInput};
use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_month::Month;
use std::collections::HashSet;

/// Derive raw cash-account movements for an inclusive period.
///
/// Callers supply which accounts represent cash. Categories and activity-level
/// attribution are deferred until account roles and journal provenance are defined.
pub fn cash_flow(
    input: ReportingInput<'_>,
    cash_accounts: &[AccountCode],
    start: Month,
    end: Month,
) -> Result<CashFlowReport, ReportingError> {
    validate_period(start, end)?;
    validate_all(input)?;

    let mut selected = HashSet::with_capacity(cash_accounts.len());
    for account in cash_accounts {
        if input.accounts().get(*account).is_none() {
            return Err(ReportingError::UnknownAccount(*account));
        }
        selected.insert(*account);
    }

    let zero = zero(input)?;
    let mut movements = Vec::new();
    let mut total = zero;

    for (journal_position, entry) in input.journal().iter().enumerate() {
        if entry.month() < start || entry.month() > end {
            continue;
        }

        for line in entry.lines() {
            if !selected.contains(&line.account()) {
                continue;
            }
            let amount = match line.side() {
                DebitCredit::Debit => line.amount(),
                DebitCredit::Credit => line.amount().checked_neg()?,
            };
            total = total.checked_add(amount)?;
            movements.push(CashFlowMovement::new(
                journal_position,
                entry.month(),
                line.account(),
                amount,
            ));
        }
    }

    Ok(CashFlowReport::new(start, end, movements, total))
}
