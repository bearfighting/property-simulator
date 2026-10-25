use super::validation::{apply_entry, validate_entry, validate_month_order, zero_balances};
use crate::{AuditReport, ReportingError, ReportingInput};

/// Audit supplied account catalogue and journal facts.
pub fn audit(input: ReportingInput<'_>) -> Result<AuditReport, ReportingError> {
    let mut balances = zero_balances(input)?;
    let mut previous_month = None;
    let mut line_count = 0;
    let mut opening_entry_count = 0;

    for entry in input.journal() {
        validate_entry(input, entry)?;
        validate_month_order(&mut previous_month, entry)?;
        if entry.month().is_initial() {
            opening_entry_count += 1;
        }
        line_count += entry.lines().len();
        apply_entry(&mut balances, entry)?;
    }

    Ok(AuditReport::new(
        input.accounts().len(),
        opening_entry_count,
        input.journal().len(),
        line_count,
    ))
}
