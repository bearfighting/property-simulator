use property_simulation_month::Month;
use property_simulation_slice_catalogue::Event;

/// Events produced during one simulated month, in calculation collection order.
///
/// Monetary values are read from Ledger projections; this DTO retains event
/// occurrences and their order, including repeated events of the same kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyEventsDto {
    pub simulation_id: String,
    pub month: Month,
    pub events: Vec<Event>,
}
