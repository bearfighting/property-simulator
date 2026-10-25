//! Translate presentation strings into the typed Application initialization request.

use std::{num::NonZeroU32, str::FromStr};

use property_simulation_account::AccountRole;
use property_simulation_application::{
    CASH_ROLE, ConfiguredSlice, EQUITY_ROLE, InitialSliceSettings, InitializeSimulationRequest,
    MORTGAGE_ROLE, OpeningBalance, PROPERTY_ROLE,
};
use property_simulation_gpui_ui::InitializationFormData;
use property_simulation_money::{Currency, Money};
use property_simulation_slice_catalogue::{
    FixedExpenseMethod, FixedExpenseParameters, LoanParameters, RentalParameters,
    RepairCostParameters,
};
use property_simulation_stop_condition::StopCondition;
use rust_decimal::Decimal;

pub(super) fn from_form(
    form: InitializationFormData,
) -> Result<InitializeSimulationRequest, String> {
    let currency = Currency::from_code("CAD").map_err(display_error)?;
    let property = parse_money(&form.property_value, currency, "property value")?;
    let cash = parse_money(&form.cash_balance, currency, "cash balance")?;
    let zero = Money::new(Decimal::ZERO, currency);
    let loan = if form.loan_enabled {
        parse_money(&form.loan_balance, currency, "mortgage balance")?
    } else {
        zero
    };
    let equity = property
        .checked_add(cash)
        .and_then(|total| total.checked_sub(loan))
        .map_err(display_error)?;
    let (annual_rate, term) = if form.loan_enabled {
        let rate_percent = Decimal::from_str(&form.annual_interest_rate_percent)
            .map_err(|error| format!("invalid annual interest rate: {error}"))?;
        (
            rate_percent / Decimal::from(100),
            parse_non_zero(&form.loan_term_months, "mortgage term")?,
        )
    } else {
        (Decimal::ZERO, NonZeroU32::new(1).expect("one is nonzero"))
    };
    let rent = if form.rental_enabled {
        parse_money(&form.monthly_rent, currency, "monthly rent")?
    } else {
        zero
    };
    let expenses = if form.fixed_expense_enabled {
        parse_money(&form.monthly_expenses, currency, "monthly expenses")?
    } else {
        zero
    };
    let repair_cost = if form.repairs_enabled {
        let maximum_amount =
            parse_money(&form.repair_maximum_amount, currency, "maximum repair cost")?;
        let delay_months = parse_non_zero(&form.repair_delay_months, "repair settlement delay")?;
        Some(RepairCostParameters::new(maximum_amount, delay_months).map_err(display_error)?)
    } else {
        None
    };

    let mut stop_conditions = Vec::new();
    if !form.stop_after_months.trim().is_empty() {
        stop_conditions.push(StopCondition::AfterMonths(parse_non_zero(
            &form.stop_after_months,
            "stop after months",
        )?));
    }
    if form.stop_when_all_loans_paid_off {
        stop_conditions.push(StopCondition::AllLoansPaidOff);
    }

    Ok(InitializeSimulationRequest {
        simulation_id: uuid::Uuid::new_v4().to_string(),
        name: form.name,
        currency,
        opening_balances: vec![
            opening_balance(PROPERTY_ROLE, property),
            opening_balance(CASH_ROLE, cash),
            opening_balance(MORTGAGE_ROLE, loan),
            opening_balance(EQUITY_ROLE, equity),
        ],
        slice_settings: InitialSliceSettings {
            loan: Some(ConfiguredSlice {
                enabled: form.loan_enabled,
                parameters: LoanParameters::new(annual_rate, term).map_err(display_error)?,
            }),
            fixed_expense: Some(ConfiguredSlice {
                enabled: form.fixed_expense_enabled,
                parameters: FixedExpenseParameters::new(FixedExpenseMethod::MonthlyAmount(
                    expenses,
                ))
                .map_err(display_error)?,
            }),
            rental: Some(ConfiguredSlice {
                enabled: form.rental_enabled,
                parameters: RentalParameters::new(rent).map_err(display_error)?,
            }),
            repair_occurrence_enabled: Some(form.repairs_enabled),
            repair_cost,
        },
        stop_conditions,
    })
}

fn opening_balance(role: &str, amount: Money) -> OpeningBalance {
    OpeningBalance {
        account_role: AccountRole::new(role).expect("application account roles are valid"),
        amount,
    }
}

fn parse_money(value: &str, currency: Currency, label: &str) -> Result<Money, String> {
    Money::parse(value.trim(), currency).map_err(|error| format!("invalid {label}: {error}"))
}

fn parse_non_zero(value: &str, label: &str) -> Result<NonZeroU32, String> {
    let parsed = value
        .trim()
        .parse::<u32>()
        .map_err(|error| format!("invalid {label}: {error}"))?;
    NonZeroU32::new(parsed).ok_or_else(|| format!("{label} must be greater than zero"))
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
