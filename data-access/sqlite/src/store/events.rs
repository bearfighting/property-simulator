use std::collections::BTreeMap;

use property_simulation_data_access::dto::MonthlyEventsDto;
use property_simulation_month::Month;
use sqlx::{Row, Sqlite, Transaction};

use crate::{SqliteDataAccessError, mapping, store::SqliteDataAccess};

pub(super) async fn insert_month_events(
    transaction: &mut Transaction<'_, Sqlite>,
    events: &MonthlyEventsDto,
) -> Result<(), SqliteDataAccessError> {
    if events.month.is_initial() {
        return if events.events.is_empty() {
            Ok(())
        } else {
            Err(SqliteDataAccessError::InvalidData("event month"))
        };
    }
    for (ordinal, event) in events.events.iter().enumerate() {
        sqlx::query(
            "INSERT INTO month_events (simulation_id, month, event_ordinal, event_kind) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&events.simulation_id)
        .bind(i64::from(events.month.number()))
        .bind(
            i64::try_from(ordinal)
                .map_err(|_| SqliteDataAccessError::InvalidData("event ordinal"))?,
        )
        .bind(mapping::event_kind(*event))
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

impl SqliteDataAccess {
    /// Load recorded events by month, preserving duplicate events and their stored order.
    pub(crate) async fn load_month_events(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<MonthlyEventsDto>, SqliteDataAccessError> {
        let rows = sqlx::query(
            "SELECT month, event_kind FROM month_events \
             WHERE simulation_id = ? ORDER BY month, event_ordinal",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        let mut events_by_month: BTreeMap<Month, Vec<_>> = BTreeMap::new();
        for row in rows {
            let month = mapping::month(row.try_get("month")?)?;
            let kind: String = row.try_get("event_kind")?;
            events_by_month
                .entry(month)
                .or_default()
                .push(mapping::event(&kind)?);
        }
        Ok(events_by_month
            .into_iter()
            .map(|(month, events)| MonthlyEventsDto {
                simulation_id: simulation_id.to_owned(),
                month,
                events,
            })
            .collect())
    }

    pub(crate) async fn load_month_events_for_month(
        &self,
        simulation_id: &str,
        month: Month,
    ) -> Result<MonthlyEventsDto, SqliteDataAccessError> {
        let rows = sqlx::query(
            "SELECT event_kind FROM month_events \
             WHERE simulation_id = ? AND month = ? ORDER BY event_ordinal",
        )
        .bind(simulation_id)
        .bind(i64::from(month.number()))
        .fetch_all(&self.pool)
        .await?;
        let events = rows
            .iter()
            .map(|row| {
                let kind: String = row.try_get("event_kind")?;
                mapping::event(&kind)
            })
            .collect::<Result<Vec<_>, SqliteDataAccessError>>()?;
        Ok(MonthlyEventsDto {
            simulation_id: simulation_id.to_owned(),
            month,
            events,
        })
    }
}
