use property_simulation_stop_condition::StopCondition;

/// All automatic stop conditions configured for one simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopConditionsDto {
    pub simulation_id: String,
    pub conditions: Vec<StopCondition>,
}
