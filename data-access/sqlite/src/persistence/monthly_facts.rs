use property_simulation_data_access::{DataAccessError, MonthlyFactsStore, dto::*};
use property_simulation_month::Month;

use crate::store::SqliteDataAccess;

impl MonthlyFactsStore for SqliteDataAccess {
    async fn save_accounting_month(
        &self,
        simulation_id: &str,
        month: Month,
        events: &MonthlyEventsDto,
        entries: &[JournalEntryDto],
        new_repair_cost_instances: &[RepairCostInstanceDto],
    ) -> Result<(), DataAccessError> {
        SqliteDataAccess::save_accounting_month(
            self,
            simulation_id,
            month,
            events,
            entries,
            new_repair_cost_instances,
        )
        .await
        .map_err(Into::into)
    }

    async fn load_journal_entries(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<JournalEntryDto>, DataAccessError> {
        SqliteDataAccess::load_journal_entries(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn load_journal_entries_for_month(
        &self,
        simulation_id: &str,
        month: Month,
    ) -> Result<Vec<JournalEntryDto>, DataAccessError> {
        SqliteDataAccess::load_journal_entries_for_month(self, simulation_id, month)
            .await
            .map_err(Into::into)
    }

    async fn load_month_events(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<MonthlyEventsDto>, DataAccessError> {
        SqliteDataAccess::load_month_events(self, simulation_id)
            .await
            .map_err(Into::into)
    }

    async fn load_month_events_for_month(
        &self,
        simulation_id: &str,
        month: Month,
    ) -> Result<MonthlyEventsDto, DataAccessError> {
        SqliteDataAccess::load_month_events_for_month(self, simulation_id, month)
            .await
            .map_err(Into::into)
    }

    async fn load_repair_cost_instances(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RepairCostInstanceDto>, DataAccessError> {
        SqliteDataAccess::load_repair_cost_instances(self, simulation_id)
            .await
            .map_err(Into::into)
    }
}
