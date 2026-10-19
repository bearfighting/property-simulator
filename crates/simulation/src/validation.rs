use property_simulation_accounting::{AccountCategory, ChartOfAccounts};
use property_simulation_domain::RoundingPolicy;
use property_simulation_models::RepairCostDistribution;

use crate::error::SimulationError;
use crate::types::{FrozenInput, OpeningState};

/// Validate configured support and referenced account roles before sampling.
pub fn validate_input(
    input: &FrozenInput,
    repair_distribution: &RepairCostDistribution,
    chart: &ChartOfAccounts,
    state: &OpeningState,
) -> Result<(), SimulationError> {
    if repair_distribution.validate_support().is_err() {
        return Err(SimulationError::InvalidInput(
            "invalid repair distribution support",
        ));
    }
    if input.rent.is_negative()
        || input.repair_cost_limit.is_negative()
        || input.rent.currency() != input.repair_cost_limit.currency()
        || state.cash.currency() != input.rent.currency()
    {
        return Err(SimulationError::InvalidInput(
            "invalid or mismatched amount",
        ));
    }
    // Exercise the same checked rounding used by the Slices before accepting
    // the frozen values. This also catches amounts that cannot be represented
    // as a posted minor-unit amount.
    input
        .rent
        .rounded_for_posting(RoundingPolicy::HalfUp)
        .map_err(|_| SimulationError::InvalidInput("rent cannot be rounded for posting"))?;
    input
        .repair_cost_limit
        .rounded_for_posting(RoundingPolicy::HalfUp)
        .map_err(|_| SimulationError::InvalidInput("repair limit cannot be rounded for posting"))?;
    repair_distribution
        .maximum
        .rounded_for_posting(RoundingPolicy::HalfUp)
        .map_err(|_| {
            SimulationError::InvalidInput("repair support cannot be rounded for posting")
        })?;
    if repair_distribution.maximum != input.repair_cost_limit {
        return Err(SimulationError::InvalidInput(
            "repair distribution support must match configured cost limit",
        ));
    }
    if chart
        .get(input.cash_account)
        .is_none_or(|a| a.category() != AccountCategory::Asset || !a.is_cash_position())
    {
        return Err(SimulationError::InvalidInput(
            "cash account must be a cash-position asset",
        ));
    }
    if chart
        .get(input.rent_account)
        .is_none_or(|a| a.category() != AccountCategory::Income)
        || chart
            .get(input.repair_expense_account)
            .is_none_or(|a| a.category() != AccountCategory::Expense)
    {
        return Err(SimulationError::InvalidInput(
            "invalid income or expense account",
        ));
    }
    Ok(())
}
