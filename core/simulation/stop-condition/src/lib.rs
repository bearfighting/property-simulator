//! Typed automatic stop conditions and their pure boundary evaluation.

use property_simulation_month::Month;
use std::num::NonZeroU32;

/// A condition that prevents the next simulation month from starting once satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopCondition {
    /// Stop after this many calculated months have completed.
    AfterMonths(NonZeroU32),

    /// Stop once all configured loan balances are paid off.
    AllLoansPaidOff,
}

/// Return whether any configured stop condition is satisfied at this boundary.
///
/// `completed_month` is the latest completed period, or Month 0 immediately
/// after initialization. The caller supplies the loan payoff fact so this
/// crate does not depend on Ledger or Loan Slice representations. An empty
/// condition list does not stop the simulation.
pub fn should_stop(
    conditions: &[StopCondition],
    completed_month: Month,
    all_loans_paid_off: bool,
) -> bool {
    conditions.iter().any(|condition| match condition {
        StopCondition::AfterMonths(months) => u32::from(completed_month.number()) >= months.get(),
        StopCondition::AllLoansPaidOff => all_loans_paid_off,
    })
}
