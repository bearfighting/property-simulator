use property_simulation_month::Month;

/// Inputs resolved for one month's repair occurrence decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairOccurrenceSliceInput {
    pub month: Month,
    /// Complete months since simulation start or the latest settled repair cost.
    pub months_since_repair_cost: u64,
}
