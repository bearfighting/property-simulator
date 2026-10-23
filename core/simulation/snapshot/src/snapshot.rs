use property_simulation_month::Month;

/// **Incomplete:** this currently stores only a Month and event tags. It does
/// not yet retain the actual inputs, effects, journal entries, or continuation
/// state produced during that month. Earlier events remain in their monthly
/// snapshots; this is not an active-event queue.
///
/// Complete the type after representative Slice results are defined, ensuring
/// it can persist the effective inputs, events/effects, Ledger increment, and
/// the state needed to continue the following month without inventing history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// The unique period key for this snapshot within one simulation.
    pub month: Month,
    pub events: Vec<crate::Event>,
    // TODO(incomplete): add the committed JournalEntry values and other monthly
    // outputs once Slice result and persistence ownership are settled.
}
