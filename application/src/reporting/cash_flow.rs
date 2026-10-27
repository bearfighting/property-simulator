use std::collections::BTreeMap;

use property_simulation_account::{AccountCatalogue, AccountClass, AccountRole};
use property_simulation_money::Money;
use property_simulation_month::Month;
use property_simulation_reporting::{
    CashFlowAccountClassification, CashFlowCategory as CoreCashFlowCategory, CashFlowMovement,
    CashFlowReport, ReportingInput, cash_flow, cash_flow_by_month,
};

use crate::{CASH_ROLE, CashFlowCategory};

use super::result::{
    CashFlowAccountSummary, CashFlowCategorySummary, CashFlowSummary, GenerateReportError,
};

pub(crate) fn classifications(accounts: &AccountCatalogue) -> Vec<CashFlowAccountClassification> {
    accounts
        .iter()
        .map(|account| {
            let category = match account.class() {
                AccountClass::Asset => CoreCashFlowCategory::Investing,
                AccountClass::Liability | AccountClass::Equity => CoreCashFlowCategory::Financing,
                AccountClass::Income | AccountClass::Expense => CoreCashFlowCategory::Operating,
            };
            CashFlowAccountClassification::new(account.code(), category, account.role().as_str())
        })
        .collect()
}

pub(crate) fn report(
    accounts: &AccountCatalogue,
    input: ReportingInput<'_>,
    start: Month,
    end: Month,
) -> Result<CashFlowReport, GenerateReportError> {
    let cash_account = accounts
        .code_for_role(&AccountRole::new(CASH_ROLE).expect("static account roles are valid"))
        .ok_or_else(|| {
            GenerateReportError::InvalidData(format!("account role {CASH_ROLE} is not configured"))
        })?;
    let classifications = classifications(accounts);
    cash_flow(input, &[cash_account], &classifications, start, end).map_err(Into::into)
}

pub(crate) fn reports_by_month(
    accounts: &AccountCatalogue,
    input: ReportingInput<'_>,
    start: Month,
    end: Month,
) -> Result<Vec<CashFlowReport>, GenerateReportError> {
    let cash_account = accounts
        .code_for_role(&AccountRole::new(CASH_ROLE).expect("static account roles are valid"))
        .ok_or_else(|| {
            GenerateReportError::InvalidData(format!("account role {CASH_ROLE} is not configured"))
        })?;
    let classifications = classifications(accounts);
    cash_flow_by_month(input, &[cash_account], &classifications, start, end).map_err(Into::into)
}

pub(crate) fn summarize(
    movements: &[CashFlowMovement],
    currency: property_simulation_money::Currency,
) -> Result<CashFlowSummary, GenerateReportError> {
    let zero = Money::from_parts(0, 0, currency)
        .map_err(|error| calculation_error("cash-flow summary", error))?;
    let mut total_inflows = zero;
    let mut total_outflows = zero;
    let mut categories = [
        category_summary(CashFlowCategory::Operating, zero),
        category_summary(CashFlowCategory::Investing, zero),
        category_summary(CashFlowCategory::Financing, zero),
    ];
    let mut accounts = BTreeMap::new();

    for movement in movements {
        let category_index = match movement.category() {
            CoreCashFlowCategory::Operating => 0,
            CoreCashFlowCategory::Investing => 1,
            CoreCashFlowCategory::Financing => 2,
        };
        let category = categories[category_index].category;
        let account = accounts
            .entry(movement.counterpart_account())
            .or_insert((category, zero, zero));

        if movement.amount().is_negative() {
            total_outflows = checked_add(total_outflows, movement.amount(), "cash-flow outflows")?;
            categories[category_index].outflows = checked_add(
                categories[category_index].outflows,
                movement.amount(),
                "cash-flow category outflows",
            )?;
            account.2 = checked_add(account.2, movement.amount(), "cash-flow account outflows")?;
        } else if !movement.amount().is_zero() {
            total_inflows = checked_add(total_inflows, movement.amount(), "cash-flow inflows")?;
            categories[category_index].inflows = checked_add(
                categories[category_index].inflows,
                movement.amount(),
                "cash-flow category inflows",
            )?;
            account.1 = checked_add(account.1, movement.amount(), "cash-flow account inflows")?;
        }
    }

    Ok(CashFlowSummary {
        total_inflows,
        total_outflows,
        categories,
        accounts: accounts
            .into_iter()
            .filter_map(|(account_code, (category, inflows, outflows))| {
                (!inflows.is_zero() || !outflows.is_zero()).then_some(CashFlowAccountSummary {
                    account_code,
                    category,
                    inflows,
                    outflows,
                })
            })
            .collect(),
    })
}

fn category_summary(category: CashFlowCategory, zero: Money) -> CashFlowCategorySummary {
    CashFlowCategorySummary {
        category,
        inflows: zero,
        outflows: zero,
    }
}

fn checked_add(
    left: Money,
    right: Money,
    operation: &'static str,
) -> Result<Money, GenerateReportError> {
    left.checked_add(right)
        .map_err(|error| calculation_error(operation, error))
}

fn calculation_error(
    operation: &'static str,
    error: impl std::fmt::Display,
) -> GenerateReportError {
    GenerateReportError::LedgerCalculation {
        operation,
        message: error.to_string(),
    }
}
