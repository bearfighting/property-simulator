use std::{num::NonZeroU16, num::NonZeroU32, str::FromStr};

use property_simulation_data_access::dto::{
    FixedExpenseSettingsDto, LoanSettingsDto, MoneyDto, RentalSettingsDto, RepairCostInstanceDto,
    RepairCostSettingsDto, RepairOccurrenceSettingsDto, SimulationDto, SimulationMonthDto,
    SimulationRootKeyDto, StopConditionsDto,
};
use property_simulation_money::{Currency, Money};
use property_simulation_month::Month;
use property_simulation_slice_catalogue::Event;
use property_simulation_slice_catalogue::{
    FixedExpenseMethod, FixedExpenseParameters, LoanParameters, RentalParameters,
    RepairCostParameters,
};
use property_simulation_stop_condition::StopCondition;
use rust_decimal::Decimal;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::SqliteDataAccessError;

pub(crate) fn month(value: i64) -> Result<Month, SqliteDataAccessError> {
    u16::try_from(value)
        .map(Month::new)
        .map_err(|_| SqliteDataAccessError::InvalidData("month"))
}

pub(crate) fn u32_value(value: i64, field: &'static str) -> Result<u32, SqliteDataAccessError> {
    u32::try_from(value).map_err(|_| SqliteDataAccessError::InvalidData(field))
}

pub(crate) const fn event_kind(event: Event) -> &'static str {
    match event {
        Event::RentCollected => "rent_collected",
        Event::RepairOccurred => "repair_occurred",
        Event::RepairCostSettled => "repair_cost_settled",
        Event::LoanPayment => "loan_payment",
        Event::FixedExpensePaid => "fixed_expense_paid",
    }
}

pub(crate) fn event(value: &str) -> Result<Event, SqliteDataAccessError> {
    match value {
        "rent_collected" => Ok(Event::RentCollected),
        "repair_occurred" => Ok(Event::RepairOccurred),
        "repair_cost_settled" => Ok(Event::RepairCostSettled),
        "loan_payment" => Ok(Event::LoanPayment),
        "fixed_expense_paid" => Ok(Event::FixedExpensePaid),
        _ => Err(SqliteDataAccessError::InvalidData("event_kind")),
    }
}

fn enabled(value: Option<i64>) -> Result<Option<bool>, SqliteDataAccessError> {
    value
        .map(|value| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SqliteDataAccessError::InvalidData("enabled")),
        })
        .transpose()
}

fn decimal(value: String, field: &'static str) -> Result<Decimal, SqliteDataAccessError> {
    Decimal::from_str(&value).map_err(|_| SqliteDataAccessError::InvalidData(field))
}

fn money(
    coefficient: String,
    scale: i64,
    currency: Currency,
) -> Result<Money, SqliteDataAccessError> {
    let coefficient = coefficient
        .parse::<i128>()
        .map_err(|_| SqliteDataAccessError::InvalidData("money coefficient"))?;
    let scale =
        u8::try_from(scale).map_err(|_| SqliteDataAccessError::InvalidData("money scale"))?;
    Ok(Money::from_parts(coefficient, scale, currency)?)
}

pub(crate) fn simulation(row: &SqliteRow) -> Result<SimulationDto, SqliteDataAccessError> {
    let currency_code: String = row.try_get("currency")?;
    Ok(SimulationDto {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        currency: Currency::from_code(&currency_code)?,
    })
}

pub(crate) fn root_key(
    simulation_id: String,
    bytes: Vec<u8>,
) -> Result<SimulationRootKeyDto, SqliteDataAccessError> {
    Ok(SimulationRootKeyDto::from_bytes(simulation_id, &bytes)?)
}

pub(crate) fn month_dto(
    simulation_id: String,
    value: i64,
) -> Result<SimulationMonthDto, SqliteDataAccessError> {
    Ok(SimulationMonthDto {
        simulation_id,
        month: month(value)?,
    })
}

pub(crate) fn repair_cost_instance(
    row: &SqliteRow,
    currency: Currency,
) -> Result<RepairCostInstanceDto, SqliteDataAccessError> {
    let maximum_amount_coefficient: String = row.try_get("maximum_amount_coefficient")?;
    let maximum_amount_scale: i64 = row.try_get("maximum_amount_scale")?;
    let settled_month: Option<i64> = row.try_get("settled_month")?;
    let instance = RepairCostInstanceDto {
        simulation_id: row.try_get("simulation_id")?,
        occurrence_month: month(row.try_get("occurrence_month")?)?,
        due_month: month(row.try_get("due_month")?)?,
        maximum_amount: MoneyDto {
            coefficient: maximum_amount_coefficient
                .parse()
                .map_err(|_| SqliteDataAccessError::InvalidData("maximum amount coefficient"))?,
            scale: u8::try_from(maximum_amount_scale)
                .map_err(|_| SqliteDataAccessError::InvalidData("maximum amount scale"))?,
            currency,
        },
        settled_month: settled_month.map(month).transpose()?,
    };
    instance.validate()?;
    Ok(instance)
}

pub(crate) fn loan_settings(row: &SqliteRow) -> Result<LoanSettingsDto, SqliteDataAccessError> {
    let enabled_value: Option<i64> = row.try_get("enabled")?;
    let rate: Option<String> = row.try_get("annual_interest_rate")?;
    let term: Option<i64> = row.try_get("remaining_term_months_at_start")?;
    let parameters = match (enabled_value, rate, term) {
        (None, None, None) => None,
        (Some(flag), Some(rate), Some(term)) if flag == 0 || flag == 1 => Some(
            LoanParameters::new(
                decimal(rate, "annual_interest_rate")?,
                NonZeroU32::new(u32_value(term, "remaining_term_months_at_start")?).ok_or(
                    SqliteDataAccessError::InvalidData("remaining_term_months_at_start"),
                )?,
            )
            .map_err(|_| SqliteDataAccessError::InvalidData("loan settings"))?,
        ),
        _ => return Err(SqliteDataAccessError::InvalidData("loan settings")),
    };
    Ok(LoanSettingsDto {
        simulation_id: row.try_get("simulation_id")?,
        enabled: enabled(enabled_value)?,
        parameters,
    })
}

pub(crate) fn fixed_expense_settings(
    row: &SqliteRow,
    currency: Currency,
) -> Result<FixedExpenseSettingsDto, SqliteDataAccessError> {
    let enabled_value: Option<i64> = row.try_get("enabled")?;
    let method: Option<String> = row.try_get("method")?;
    let coefficient: Option<String> = row.try_get("monthly_amount_coefficient")?;
    let scale: Option<i64> = row.try_get("monthly_amount_scale")?;
    let annual_rate: Option<String> = row.try_get("annual_property_value_rate")?;
    let parameters = match (enabled_value, method, coefficient, scale, annual_rate) {
        (None, None, None, None, None) => None,
        (Some(flag), Some(method), coefficient, scale, annual_rate) if flag == 0 || flag == 1 => {
            let method = match (method.as_str(), coefficient, scale, annual_rate) {
                ("monthly_amount", Some(coefficient), Some(scale), None) => {
                    FixedExpenseMethod::MonthlyAmount(money(coefficient, scale, currency)?)
                }
                ("annual_property_value_rate", None, None, Some(rate)) => {
                    FixedExpenseMethod::AnnualPropertyValueRate(decimal(
                        rate,
                        "annual_property_value_rate",
                    )?)
                }
                _ => return Err(SqliteDataAccessError::InvalidData("fixed expense settings")),
            };
            Some(
                FixedExpenseParameters::new(method)
                    .map_err(|_| SqliteDataAccessError::InvalidData("fixed expense settings"))?,
            )
        }
        _ => return Err(SqliteDataAccessError::InvalidData("fixed expense settings")),
    };
    Ok(FixedExpenseSettingsDto {
        simulation_id: row.try_get("simulation_id")?,
        enabled: enabled(enabled_value)?,
        parameters,
    })
}

pub(crate) fn rental_settings(
    row: &SqliteRow,
    currency: Currency,
) -> Result<RentalSettingsDto, SqliteDataAccessError> {
    let enabled_value: Option<i64> = row.try_get("enabled")?;
    let coefficient: Option<String> = row.try_get("monthly_rent_coefficient")?;
    let scale: Option<i64> = row.try_get("monthly_rent_scale")?;
    let parameters = match (enabled_value, coefficient, scale) {
        (None, None, None) => None,
        (Some(flag), Some(coefficient), Some(scale)) if flag == 0 || flag == 1 => Some(
            RentalParameters::new(money(coefficient, scale, currency)?)
                .map_err(|_| SqliteDataAccessError::InvalidData("rental settings"))?,
        ),
        _ => return Err(SqliteDataAccessError::InvalidData("rental settings")),
    };
    Ok(RentalSettingsDto {
        simulation_id: row.try_get("simulation_id")?,
        enabled: enabled(enabled_value)?,
        parameters,
    })
}

pub(crate) fn repair_occurrence_settings(
    row: &SqliteRow,
) -> Result<RepairOccurrenceSettingsDto, SqliteDataAccessError> {
    let value: Option<i64> = row.try_get("enabled")?;
    Ok(RepairOccurrenceSettingsDto {
        simulation_id: row.try_get("simulation_id")?,
        enabled: enabled(value)?,
    })
}

pub(crate) fn repair_cost_settings(
    row: &SqliteRow,
    currency: Currency,
) -> Result<RepairCostSettingsDto, SqliteDataAccessError> {
    let coefficient: Option<String> = row.try_get("maximum_amount_coefficient")?;
    let scale: Option<i64> = row.try_get("maximum_amount_scale")?;
    let delay: Option<i64> = row.try_get("delay_months")?;
    let parameters = match (coefficient, scale, delay) {
        (None, None, None) => None,
        (Some(coefficient), Some(scale), Some(delay)) => Some(
            RepairCostParameters::new(
                money(coefficient, scale, currency)?,
                NonZeroU32::new(u32_value(delay, "delay_months")?)
                    .ok_or(SqliteDataAccessError::InvalidData("delay_months"))?,
            )
            .map_err(|_| SqliteDataAccessError::InvalidData("repair cost settings"))?,
        ),
        _ => return Err(SqliteDataAccessError::InvalidData("repair cost settings")),
    };
    Ok(RepairCostSettingsDto {
        simulation_id: row.try_get("simulation_id")?,
        parameters,
    })
}

pub(crate) fn stop_conditions(
    simulation_id: String,
    rows: &[SqliteRow],
) -> Result<StopConditionsDto, SqliteDataAccessError> {
    let conditions = rows
        .iter()
        .map(|row| {
            let kind: String = row.try_get("condition")?;
            let value: Option<i64> = row.try_get("value")?;
            match (kind.as_str(), value) {
                ("after_months", Some(value)) => {
                    let months = u16::try_from(value)
                        .map_err(|_| SqliteDataAccessError::InvalidData("stop condition value"))?;
                    Ok(StopCondition::AfterMonths(NonZeroU16::new(months).ok_or(
                        SqliteDataAccessError::InvalidData("stop condition value"),
                    )?))
                }
                ("all_loans_paid_off", None) => Ok(StopCondition::AllLoansPaidOff),
                _ => Err(SqliteDataAccessError::InvalidData("stop condition")),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(StopConditionsDto {
        simulation_id,
        conditions,
    })
}
