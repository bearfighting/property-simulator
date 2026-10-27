//! Typed automatic stop conditions and their pure boundary evaluation.

use property_simulation_month::Month;
use std::num::NonZeroU16;

/// A condition that prevents the next simulation month from starting once satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopCondition {
    /// Stop after this many calculated months have completed.
    AfterMonths(NonZeroU16),

    /// Stop once all configured loan balances are paid off.
    AllLoansPaidOff,
}

/// Return the configured stop conditions satisfied at this boundary.
///
/// `completed_month` is the latest completed period, or Month 0 immediately
/// after initialization. The caller supplies the loan payoff fact so this
/// crate does not depend on Ledger or Loan Slice representations. The returned
/// conditions preserve their configured order. An empty condition list returns
/// an empty vector.
pub fn satisfied_conditions(
    conditions: &[StopCondition],
    completed_month: Month,
    all_loans_paid_off: bool,
) -> Vec<StopCondition> {
    conditions
        .iter()
        .copied()
        .filter(|condition| match condition {
            StopCondition::AfterMonths(months) => completed_month.number() >= months.get(),
            StopCondition::AllLoansPaidOff => all_loans_paid_off,
        })
        .collect()
}

/// Return whether any configured stop condition is satisfied at this boundary.
///
/// This convenience predicate is derived from [`satisfied_conditions`].
pub fn should_stop(
    conditions: &[StopCondition],
    completed_month: Month,
    all_loans_paid_off: bool,
) -> bool {
    !satisfied_conditions(conditions, completed_month, all_loans_paid_off).is_empty()
}
