use property_simulation_account::AccountCatalogue;
use property_simulation_ledger::Ledger;
use property_simulation_money::Money;
use property_simulation_month::Month;
use property_simulation_reporting::{
    CashFlowCategory as ReportingCategory, ReportingInput, cash_flow,
};

use super::result::{
    CashFlowAccountChanges, CashFlowCategory, CashFlowCategoryChanges, CashFlowChanges,
    MonthSimulationError,
};
use crate::{CASH_ROLE, reporting::cash_flow::classifications};

pub(super) fn monthly_cash_flow_changes(
    accounts: &AccountCatalogue,
    ledger: &Ledger,
    month: Month,
    currency: property_simulation_money::Currency,
) -> Result<CashFlowChanges, MonthSimulationError> {
    let cash_role = property_simulation_account::AccountRole::new(CASH_ROLE)
        .expect("static account roles are valid");
    let cash = accounts.code_for_role(&cash_role).ok_or_else(|| {
        MonthSimulationError::InvalidData(format!("account role {CASH_ROLE} is not configured"))
    })?;
    let classifications = classifications(accounts);
    let report = cash_flow(
        ReportingInput::new(accounts, currency, ledger.journal()),
        &[cash],
        &classifications,
        month,
        month,
    )
    .map_err(|error| MonthSimulationError::calculation("cash-flow reporting", error))?;
    let zero = Money::from_parts(0, 0, currency).map_err(MonthSimulationError::from_money)?;
    let mut positive = zero;
    let mut negative = zero;
    let mut categories = [
        CashFlowCategoryChanges {
            category: CashFlowCategory::Operating,
            positive: zero,
            negative: zero,
        },
        CashFlowCategoryChanges {
            category: CashFlowCategory::Investing,
            positive: zero,
            negative: zero,
        },
        CashFlowCategoryChanges {
            category: CashFlowCategory::Financing,
            positive: zero,
            negative: zero,
        },
    ];
    let mut account_changes = std::collections::BTreeMap::new();
    for movement in report.movements() {
        let category = match movement.category() {
            ReportingCategory::Operating => 0,
            ReportingCategory::Investing => 1,
            ReportingCategory::Financing => 2,
        };
        if movement.amount().is_negative() {
            negative = negative
                .checked_add(movement.amount())
                .map_err(MonthSimulationError::from_money)?;
            categories[category].negative = categories[category]
                .negative
                .checked_add(movement.amount())
                .map_err(MonthSimulationError::from_money)?;
            let account = accounts
                .get(movement.counterpart_account())
                .ok_or_else(|| {
                    MonthSimulationError::InvalidData(format!(
                        "cash-flow movement refers to missing account {}",
                        movement.counterpart_account()
                    ))
                })?;
            let entry = account_changes
                .entry(movement.counterpart_account())
                .or_insert_with(|| (account.name().to_owned(), movement.category(), zero, zero));
            entry.3 = entry
                .3
                .checked_add(movement.amount())
                .map_err(MonthSimulationError::from_money)?;
        } else if !movement.amount().is_zero() {
            positive = positive
                .checked_add(movement.amount())
                .map_err(MonthSimulationError::from_money)?;
            categories[category].positive = categories[category]
                .positive
                .checked_add(movement.amount())
                .map_err(MonthSimulationError::from_money)?;
            let account = accounts
                .get(movement.counterpart_account())
                .ok_or_else(|| {
                    MonthSimulationError::InvalidData(format!(
                        "cash-flow movement refers to missing account {}",
                        movement.counterpart_account()
                    ))
                })?;
            let entry = account_changes
                .entry(movement.counterpart_account())
                .or_insert_with(|| (account.name().to_owned(), movement.category(), zero, zero));
            entry.2 = entry
                .2
                .checked_add(movement.amount())
                .map_err(MonthSimulationError::from_money)?;
        }
    }
    Ok(CashFlowChanges {
        positive,
        negative,
        categories,
        accounts: account_changes
            .into_values()
            .map(
                |(account_name, category, positive, negative)| CashFlowAccountChanges {
                    account_name,
                    category: match category {
                        ReportingCategory::Operating => CashFlowCategory::Operating,
                        ReportingCategory::Investing => CashFlowCategory::Investing,
                        ReportingCategory::Financing => CashFlowCategory::Financing,
                    },
                    positive,
                    negative,
                },
            )
            .collect(),
    })
}
