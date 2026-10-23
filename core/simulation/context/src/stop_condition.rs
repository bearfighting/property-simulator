use std::num::NonZeroU32;

/// A configured condition that stops the simulation when satisfied.
///
/// Conditions are checked at the start of a month using the latest completed
/// Snapshot. `AfterMonths(1)` therefore permits calculation of Month 0 and
/// prevents Month 1 from starting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopCondition {
    /// Stop after this many months have completed.
    AfterMonths(NonZeroU32),

    /// Stop once all configured loan balances are paid off.
    AllLoansPaidOff,
}
