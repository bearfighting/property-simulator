use property_simulation_month::Month;
use property_simulation_stop_condition::StopCondition;

/// A complete, storage-neutral version of stop conditions, including an empty set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopConditionSetDto {
    pub simulation_id: String,
    pub effective_from_month: Month,
    pub conditions: Vec<StopCondition>,
}
