use std::{collections::BTreeMap, num::NonZeroU32};

use property_simulation_account::{AccountCatalogue, AccountCode};
use property_simulation_data_access::dto::{
    FixedExpenseSettingsDto, LoanSettingsDto, RentalSettingsDto,
};
use property_simulation_money::Money;
use property_simulation_month::Month;
use property_simulation_slice::{DeterministicSlice, FixedExpenseSlice, LoanSlice, RentalSlice};
use property_simulation_slice_catalogue::{
    FixedExpenseSliceInput, LoanSliceInput, RentalSliceInput, SliceResult,
};

use super::{
    MonthSimulationError,
    ledger::balance,
    settings::{enabled_fixed_expense, enabled_loan, enabled_rental, role_code},
};
use crate::{
    CASH_ROLE, INTEREST_EXPENSE_ROLE, MORTGAGE_ROLE, OPERATING_EXPENSE_ROLE, PROPERTY_ROLE,
    RENTAL_INCOME_ROLE,
};

pub(super) struct StandardSliceInputs<'a> {
    pub accounts: &'a AccountCatalogue,
    pub month: Month,
    pub starting_balances: &'a BTreeMap<AccountCode, Money>,
    pub initial_balances: &'a BTreeMap<AccountCode, Money>,
    pub loan_settings: Option<LoanSettingsDto>,
    pub fixed_expense_settings: Option<FixedExpenseSettingsDto>,
    pub rental_settings: Option<RentalSettingsDto>,
}

pub(super) fn calculate_standard_slices(
    input: StandardSliceInputs<'_>,
) -> Result<Vec<SliceResult>, MonthSimulationError> {
    let cash_account = role_code(input.accounts, CASH_ROLE)?;
    let mut results = Vec::new();
    calculate_loan(
        &mut results,
        input.loan_settings,
        input.month,
        input.starting_balances,
        input.initial_balances,
        cash_account,
        role_code(input.accounts, MORTGAGE_ROLE)?,
        role_code(input.accounts, INTEREST_EXPENSE_ROLE)?,
    )?;
    calculate_rental(
        &mut results,
        input.rental_settings,
        input.month,
        cash_account,
        role_code(input.accounts, RENTAL_INCOME_ROLE)?,
    )?;
    calculate_fixed_expense(
        &mut results,
        input.fixed_expense_settings,
        input.month,
        input.starting_balances,
        role_code(input.accounts, PROPERTY_ROLE)?,
        cash_account,
        role_code(input.accounts, OPERATING_EXPENSE_ROLE)?,
    )?;
    Ok(results)
}

// These arguments are the exact monthly facts mapped to `LoanSliceInput`; keeping
// them explicit avoids introducing another aggregate input type for one call.
#[allow(clippy::too_many_arguments)]
fn calculate_loan(
    results: &mut Vec<SliceResult>,
    settings: Option<LoanSettingsDto>,
    month: Month,
    starting_balances: &BTreeMap<AccountCode, Money>,
    initial_balances: &BTreeMap<AccountCode, Money>,
    cash_account: AccountCode,
    mortgage_account: AccountCode,
    interest_expense_account: AccountCode,
) -> Result<(), MonthSimulationError> {
    let Some(settings) = enabled_loan(settings)? else {
        return Ok(());
    };
    let opening_balance = balance(starting_balances, mortgage_account)?.checked_neg()?;
    if opening_balance.is_negative() || opening_balance.is_zero() {
        return Ok(());
    }
    let initial_principal = balance(initial_balances, mortgage_account)?.checked_neg()?;
    let elapsed = u32::from(month.number() - 1);
    let remaining = settings
        .parameters
        .remaining_term_months_at_start()
        .get()
        .checked_sub(elapsed)
        .and_then(NonZeroU32::new)
        .ok_or_else(|| {
            MonthSimulationError::InvalidData(
                "loan balance remains after its configured term".into(),
            )
        })?;
    let input = LoanSliceInput {
        month,
        initial_principal,
        opening_balance,
        remaining_term_months_at_month_start: remaining,
        cash_account,
        loan_liability_account: mortgage_account,
        interest_expense_account,
    };
    results.push(
        LoanSlice::new(settings.parameters, input)
            .and_then(|slice| slice.calculate())
            .map_err(|error| MonthSimulationError::calculation("loan", error))?,
    );
    Ok(())
}

fn calculate_rental(
    results: &mut Vec<SliceResult>,
    settings: Option<RentalSettingsDto>,
    month: Month,
    cash_account: AccountCode,
    income_account: AccountCode,
) -> Result<(), MonthSimulationError> {
    let Some(settings) = enabled_rental(settings)? else {
        return Ok(());
    };
    results.push(
        RentalSlice::new(
            settings.parameters,
            RentalSliceInput {
                month,
                cash_account,
                rental_income_account: income_account,
            },
        )
        .calculate()
        .map_err(|error| MonthSimulationError::calculation("rental income", error))?,
    );
    Ok(())
}

fn calculate_fixed_expense(
    results: &mut Vec<SliceResult>,
    settings: Option<FixedExpenseSettingsDto>,
    month: Month,
    starting_balances: &BTreeMap<AccountCode, Money>,
    property_account: AccountCode,
    cash_account: AccountCode,
    expense_account: AccountCode,
) -> Result<(), MonthSimulationError> {
    let Some(settings) = enabled_fixed_expense(settings)? else {
        return Ok(());
    };
    results.push(
        FixedExpenseSlice::new(
            settings.parameters,
            FixedExpenseSliceInput {
                month,
                property_value: balance(starting_balances, property_account)?,
                cash_account,
                expense_account,
            },
        )
        .calculate()
        .map_err(|error| MonthSimulationError::calculation("fixed expense", error))?,
    );
    Ok(())
}
