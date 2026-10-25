use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_data_access::dto::{
    JournalEntryDto, JournalLineDto, MonthlyEventsDto, RepairCostInstanceDto,
};
use property_simulation_money::Currency;
use property_simulation_month::Month;
use property_simulation_slice_catalogue::Event;
use sqlx::Row;

use crate::{
    SqliteDataAccessError, mapping,
    store::{SqliteDataAccess, events::insert_month_events, simulation::ensure_month},
};

impl SqliteDataAccess {
    /// Save one month's events, JournalEntries, and repair instance lifecycle changes atomically.
    ///
    pub async fn save_accounting_month(
        &self,
        simulation_id: &str,
        month: Month,
        events: &MonthlyEventsDto,
        entries: &[JournalEntryDto],
        new_repair_cost_instances: &[RepairCostInstanceDto],
    ) -> Result<(), SqliteDataAccessError> {
        if events.simulation_id != simulation_id || events.month != month {
            return Err(SqliteDataAccessError::InvalidData("event month"));
        }
        let currency = self.simulation_currency(simulation_id).await?;
        for entry in entries {
            if entry.simulation_id != simulation_id || entry.month != month {
                return Err(SqliteDataAccessError::InvalidData("journal entry month"));
            }
            entry.domain_entry(currency)?;
        }
        let occurred_repairs = events
            .events
            .iter()
            .filter(|event| **event == Event::RepairOccurred)
            .count();
        if occurred_repairs != new_repair_cost_instances.len() {
            return Err(SqliteDataAccessError::InvalidData(
                "repair occurrence instances",
            ));
        }
        for instance in new_repair_cost_instances {
            instance.validate()?;
            if instance.simulation_id != simulation_id
                || instance.occurrence_month != month
                || instance.settled_month.is_some()
            {
                return Err(SqliteDataAccessError::InvalidData(
                    "repair cost instance month",
                ));
            }
            instance.maximum_amount.to_domain(currency)?;
        }
        let settlements_due = events
            .events
            .iter()
            .filter(|event| **event == Event::RepairCostSettled)
            .count();

        let mut transaction = self.pool.begin().await?;
        ensure_month(&mut transaction, simulation_id, month.number()).await?;

        for instance in new_repair_cost_instances {
            sqlx::query(
                "INSERT INTO repair_cost_instances \
                 (simulation_id, occurrence_month, due_month, maximum_amount_coefficient, \
                  maximum_amount_scale, settled_month) VALUES (?, ?, ?, ?, ?, NULL)",
            )
            .bind(&instance.simulation_id)
            .bind(i64::from(instance.occurrence_month.number()))
            .bind(i64::from(instance.due_month.number()))
            .bind(instance.maximum_amount.coefficient.to_string())
            .bind(i64::from(instance.maximum_amount.scale))
            .execute(&mut *transaction)
            .await?;
        }

        let due_instances: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM repair_cost_instances \
             WHERE simulation_id = ? AND due_month = ? AND settled_month IS NULL",
        )
        .bind(simulation_id)
        .bind(i64::from(month.number()))
        .fetch_one(&mut *transaction)
        .await?;
        if usize::try_from(due_instances).ok() != Some(settlements_due) {
            return Err(SqliteDataAccessError::InvalidData(
                "repair cost settlement events",
            ));
        }

        insert_month_events(&mut transaction, events).await?;
        if due_instances > 0 {
            sqlx::query(
                "UPDATE repair_cost_instances SET settled_month = ? \
                 WHERE simulation_id = ? AND due_month = ? AND settled_month IS NULL",
            )
            .bind(i64::from(month.number()))
            .bind(simulation_id)
            .bind(i64::from(month.number()))
            .execute(&mut *transaction)
            .await?;
        }
        for entry in entries {
            let result = sqlx::query(
                "INSERT INTO journal_entries (simulation_id, month, sequence) VALUES (?, ?, ?)",
            )
            .bind(simulation_id)
            .bind(i64::from(month.number()))
            .bind(i64::from(entry.sequence))
            .execute(&mut *transaction)
            .await?;
            let entry_id = result.last_insert_rowid();
            for (line_number, line) in entry.lines.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO journal_lines \
                     (journal_entry_id, simulation_id, line_number, account_code, side, amount_coefficient, amount_scale) \
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(entry_id)
                .bind(simulation_id)
                .bind(i64::try_from(line_number).map_err(|_| SqliteDataAccessError::InvalidData("line number"))?)
                .bind(i64::from(line.account_code.value()))
                .bind(match line.side {
                    DebitCredit::Debit => "debit",
                    DebitCredit::Credit => "credit",
                })
                .bind(line.amount.coefficient.to_string())
                .bind(i64::from(line.amount.scale))
                .execute(&mut *transaction)
                .await?;
            }
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Load all journal entries in append order and validate them through Core.
    pub async fn load_journal_entries(
        &self,
        simulation_id: &str,
    ) -> Result<Vec<JournalEntryDto>, SqliteDataAccessError> {
        let currency = self.simulation_currency(simulation_id).await?;
        let headers = sqlx::query(
            "SELECT id, simulation_id, month, sequence FROM journal_entries \
             WHERE simulation_id = ? ORDER BY sequence",
        )
        .bind(simulation_id)
        .fetch_all(&self.pool)
        .await?;
        let mut entries = Vec::with_capacity(headers.len());
        for header in headers {
            let id: i64 = header.try_get("id")?;
            let month_value: i64 = header.try_get("month")?;
            let sequence_value: i64 = header.try_get("sequence")?;
            let month = mapping::month(month_value)?;
            let rows = sqlx::query(
                "SELECT account_code, side, amount_coefficient, amount_scale \
                 FROM journal_lines WHERE journal_entry_id = ? ORDER BY line_number",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await?;
            let lines = rows
                .iter()
                .map(|row| line_dto(row, currency))
                .collect::<Result<Vec<_>, _>>()?;
            let entry = JournalEntryDto {
                simulation_id: header.try_get("simulation_id")?,
                month,
                sequence: mapping::u32_value(sequence_value, "sequence")?,
                lines,
            };
            entry.domain_entry(currency)?;
            entries.push(entry);
        }
        Ok(entries)
    }
}

fn line_dto(
    row: &sqlx::sqlite::SqliteRow,
    currency: Currency,
) -> Result<JournalLineDto, SqliteDataAccessError> {
    let account_code: i64 = row.try_get("account_code")?;
    let account_code = mapping::u32_value(account_code, "account_code")?;
    let account_code = AccountCode::new(account_code)
        .map_err(|_| SqliteDataAccessError::InvalidData("account_code"))?;
    let side: String = row.try_get("side")?;
    let side = match side.as_str() {
        "debit" => DebitCredit::Debit,
        "credit" => DebitCredit::Credit,
        _ => return Err(SqliteDataAccessError::InvalidData("journal line side")),
    };
    let coefficient: String = row.try_get("amount_coefficient")?;
    let coefficient = coefficient
        .parse::<i128>()
        .map_err(|_| SqliteDataAccessError::InvalidData("amount_coefficient"))?;
    let scale: i64 = row.try_get("amount_scale")?;
    let scale =
        u8::try_from(scale).map_err(|_| SqliteDataAccessError::InvalidData("amount_scale"))?;
    let amount = property_simulation_data_access::dto::MoneyDto {
        coefficient,
        scale,
        currency,
    };
    Ok(JournalLineDto {
        account_code,
        side,
        amount,
    })
}
