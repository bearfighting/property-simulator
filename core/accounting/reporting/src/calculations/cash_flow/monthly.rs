use super::super::validation::{validate_all, validate_period, zero};
use super::{build_report, prepare_classifications};
use crate::{
    CashFlowAccountClassification, CashFlowCategory, CashFlowMovement, CashFlowReport,
    ReportingError, ReportingInput,
};
use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_money::Money;
use property_simulation_month::Month;
use std::collections::{BTreeMap, HashMap};

/// Derive independent cash-flow reports for every month in an inclusive range.
///
/// The Ledger is audited and traversed once. Each result retains monthly
/// overflow behavior instead of accumulating amounts across the entire range.
pub fn cash_flow_by_month(
    input: ReportingInput<'_>,
    cash_accounts: &[AccountCode],
    classifications: &[CashFlowAccountClassification],
    start: Month,
    end: Month,
) -> Result<Vec<CashFlowReport>, ReportingError> {
    let entries = collect_entries(input, cash_accounts, classifications, start, end)?;
    let zero = zero(input)?;
    let month_range = start
        .range_inclusive(end)
        .map_err(|_| ReportingError::InvalidPeriod { start, end })?;
    let mut monthly: BTreeMap<_, _> = month_range
        .map(|month| (month, MonthCashFlow::new(zero)))
        .collect();

    for entry in entries {
        let month = monthly
            .get_mut(&entry.month)
            .expect("the validated period contains every included journal month");
        for amount in entry.cash_amounts {
            month.total = month.total.checked_add(amount)?;
        }
        for movement in entry.movements {
            let category_total = month
                .category_totals
                .entry(movement.category())
                .or_insert(zero);
            *category_total = category_total.checked_add(movement.amount())?;

            let subcategory_key = (movement.category(), movement.subcategory().to_owned());
            let subcategory_total = month
                .subcategory_totals
                .entry(subcategory_key)
                .or_insert(zero);
            *subcategory_total = subcategory_total.checked_add(movement.amount())?;
            month.movements.push(movement);
        }
    }

    Ok(monthly
        .into_iter()
        .map(|(month, result)| result.into_report(month))
        .collect())
}

struct ClassifiedEntry {
    month: Month,
    cash_amounts: Vec<Money>,
    movements: Vec<CashFlowMovement>,
}

fn collect_entries(
    input: ReportingInput<'_>,
    cash_accounts: &[AccountCode],
    classifications: &[CashFlowAccountClassification],
    start: Month,
    end: Month,
) -> Result<Vec<ClassifiedEntry>, ReportingError> {
    validate_period(start, end)?;
    validate_all(input)?;
    let (selected, classification_by_account) =
        prepare_classifications(input, cash_accounts, classifications)?;
    let zero = zero(input)?;
    let mut entries = Vec::new();

    for (journal_position, entry) in input.journal().iter().enumerate() {
        let month = entry.month();
        if month < start || month > end {
            continue;
        }

        let mut cash_amounts = Vec::new();
        let mut entry_cash_total = zero;
        for line in entry.lines() {
            if !selected.contains(&line.account()) {
                continue;
            }
            let amount = match line.side() {
                DebitCredit::Debit => line.amount(),
                DebitCredit::Credit => line.amount().checked_neg()?,
            };
            entry_cash_total = entry_cash_total.checked_add(amount)?;
            cash_amounts.push(amount);
        }

        if cash_amounts.is_empty() {
            continue;
        }

        let mut movements = Vec::new();
        let mut entry_classified_total = zero;
        for line in entry.lines() {
            if selected.contains(&line.account()) {
                continue;
            }
            let classification = classification_by_account.get(&line.account()).ok_or(
                ReportingError::MissingCashFlowClassification(line.account()),
            )?;
            let amount = match line.side() {
                DebitCredit::Debit => line.amount().checked_neg()?,
                DebitCredit::Credit => line.amount(),
            };
            entry_classified_total = entry_classified_total.checked_add(amount)?;
            movements.push(CashFlowMovement::new(
                journal_position,
                month,
                line.account(),
                classification.category(),
                classification.subcategory().to_owned(),
                amount,
            ));
        }

        if entry_cash_total.checked_cmp(entry_classified_total)? != std::cmp::Ordering::Equal {
            return Err(ReportingError::CashFlowClassificationMismatch {
                journal_position,
                cash_amount: entry_cash_total,
                classified_amount: entry_classified_total,
            });
        }

        entries.push(ClassifiedEntry {
            month,
            cash_amounts,
            movements,
        });
    }

    Ok(entries)
}

struct MonthCashFlow {
    movements: Vec<CashFlowMovement>,
    total: Money,
    category_totals: HashMap<CashFlowCategory, Money>,
    subcategory_totals: HashMap<(CashFlowCategory, String), Money>,
}

impl MonthCashFlow {
    fn new(zero: Money) -> Self {
        Self {
            movements: Vec::new(),
            total: zero,
            category_totals: HashMap::new(),
            subcategory_totals: HashMap::new(),
        }
    }

    fn into_report(self, month: Month) -> CashFlowReport {
        build_report(
            month,
            month,
            self.movements,
            self.category_totals,
            self.subcategory_totals,
            self.total,
        )
    }
}
