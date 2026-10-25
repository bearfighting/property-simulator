use property_simulation_month::Month;

use crate::{DataAccessError, dto::*};

/// Read and write month-scoped event, accounting, and repair-instance facts.
#[allow(async_fn_in_trait)]
pub trait MonthlyFactsStore {
    async fn save_accounting_month(
        &self,
        simulation_id: &str,
        month: Month,
        events: &MonthlyEventsDto,
        entries: &[JournalEntryDto],
        new_repair_cost_instances: &[RepairCostInstanceDto],
    ) -> Result<(), DataAccessError>;

    async fn load_journal_entries(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<JournalEntryDto>, DataAccessError>;

    async fn load_journal_entries_for_month(
        &self,
        simulation_id: &str,
        month: Month,
    ) -> Result<Vec<JournalEntryDto>, DataAccessError>;

    async fn load_month_events(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<MonthlyEventsDto>, DataAccessError>;

    async fn load_month_events_for_month(
        &self,
        simulation_id: &str,
        month: Month,
    ) -> Result<MonthlyEventsDto, DataAccessError>;

    async fn load_repair_cost_instances(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RepairCostInstanceDto>, DataAccessError>;
}
