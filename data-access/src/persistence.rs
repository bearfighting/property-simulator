//! Storage-neutral persistence operations over the facts represented by the schema.

mod monthly_facts;
mod settings;
mod simulation;

pub use monthly_facts::MonthlyFactsStore;
pub use settings::SettingsStore;
pub use simulation::SimulationStore;

/// Combined persistence capability for callers that need every data category.
/// Individual work units can depend on a smaller store trait directly.
pub trait DataAccess: SimulationStore + SettingsStore + MonthlyFactsStore {}

impl<T> DataAccess for T where T: SimulationStore + SettingsStore + MonthlyFactsStore {}
