use super::validation::{
    apply_entry, validate_entry, validate_month_order, validated_opening_balances,
};
use crate::{AuditReport, ReportingError, ReportingInput};

/// Audit supplied account, opening balance, and journal facts.
pub fn audit(input: ReportingInput<'_>) -> Result<AuditReport, ReportingError> {
    let (mut balances, opening_count) = validated_opening_balances(input)?;
    let mut previous_month = None;
    let mut line_count = 0;

    for entry in input.journal() {
        validate_entry(input, entry)?;
        validate_month_order(&mut previous_month, entry)?;
        line_count += entry.lines().len();
        apply_entry(&mut balances, entry)?;
    }

    Ok(AuditReport::new(
        input.accounts().len(),
        opening_count,
        input.journal().len(),
        line_count,
    ))
}
