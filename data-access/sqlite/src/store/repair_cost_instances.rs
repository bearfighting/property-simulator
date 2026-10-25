use property_simulation_data_access::dto::RepairCostInstanceDto;

use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};

impl SqliteDataAccess {
    /// Load generated repair cost instances, including their due and settlement months.
    pub async fn load_repair_cost_instances(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<RepairCostInstanceDto>, SqliteDataAccessError> {
        let currency = self.simulation_currency(simulation_id).await?;
        let rows = sqlx::query(
            "SELECT simulation_id, occurrence_month, due_month, maximum_amount_coefficient, \
                    maximum_amount_scale, settled_month \
             FROM repair_cost_instances WHERE simulation_id = ? ORDER BY occurrence_month",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| mapping::repair_cost_instance(row, currency))
            .collect()
    }
}
