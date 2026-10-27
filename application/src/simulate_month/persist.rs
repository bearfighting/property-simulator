use property_simulation_data_access::{
    DataAccess,
    dto::{JournalEntryDto, MonthlyEventsDto},
};

use super::{MonthSimulationError, MonthSimulationOutput, prepare::PreparedMonth};

pub(super) async fn persist_month<D: DataAccess>(
    data_access: &D,
    simulation_id: &str,
    prepared: PreparedMonth,
) -> Result<MonthSimulationOutput, MonthSimulationError> {
    let entries = prepared
        .entries
        .iter()
        .enumerate()
        .map(|(offset, entry)| {
            let sequence = prepared
                .next_journal_sequence
                .checked_add(u32::try_from(offset).map_err(|_| {
                    MonthSimulationError::InvalidData("too many monthly journal entries".into())
                })?)
                .ok_or_else(|| {
                    MonthSimulationError::InvalidData("journal sequence overflow".into())
                })?;
            Ok(JournalEntryDto::from_domain(
                simulation_id.to_owned(),
                sequence,
                entry,
            ))
        })
        .collect::<Result<Vec<_>, MonthSimulationError>>()?;

    data_access
        .save_accounting_month(
            simulation_id,
            prepared.month,
            &MonthlyEventsDto {
                simulation_id: simulation_id.to_owned(),
                month: prepared.month,
                events: prepared.events,
            },
            &entries,
            &prepared.new_repair_instances,
        )
        .await?;
    Ok(prepared.output)
}
