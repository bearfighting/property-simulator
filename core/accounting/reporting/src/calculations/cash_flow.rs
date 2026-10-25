use super::validation::{validate_all, validate_period, zero};
use crate::{
    CashFlowAccountClassification, CashFlowCategoryTotal, CashFlowMovement, CashFlowReport,
    CashFlowSubcategoryTotal, ReportingError, ReportingInput,
};
use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_month::Month;
use std::collections::{HashMap, HashSet};

/// Derive categorized cash-flow movements for an inclusive period.
///
/// Callers identify cash accounts and classify every non-cash account that can
/// be posted opposite a cash line. Each counterpart line contributes the inverse
/// of its debit/credit amount to its configured cash-flow category.
pub fn cash_flow(
    input: ReportingInput<'_>,
    cash_accounts: &[AccountCode],
    classifications: &[CashFlowAccountClassification],
    start: Month,
    end: Month,
) -> Result<CashFlowReport, ReportingError> {
    validate_period(start, end)?;
    validate_all(input)?;

    let mut selected = HashSet::with_capacity(cash_accounts.len());
    for account in cash_accounts {
        if input.accounts().get(*account).is_none() {
            return Err(ReportingError::UnknownAccount(*account));
        }
        selected.insert(*account);
    }

    let mut classification_by_account = HashMap::with_capacity(classifications.len());
    for classification in classifications {
        if input.accounts().get(classification.account()).is_none() {
            return Err(ReportingError::UnknownAccount(classification.account()));
        }
        if classification_by_account
            .insert(classification.account(), classification.clone())
            .is_some()
        {
            return Err(ReportingError::DuplicateCashFlowClassification(
                classification.account(),
            ));
        }
    }

    let zero = zero(input)?;
    let mut movements = Vec::new();
    let mut total = zero;
    let mut category_totals = HashMap::new();
    let mut subcategory_totals = HashMap::new();

    for (journal_position, entry) in input.journal().iter().enumerate() {
        let month = entry.month();
        if month < start || month > end {
            continue;
        }

        let mut has_cash_line = false;
        let mut entry_cash_total = zero;
        for line in entry.lines() {
            if !selected.contains(&line.account()) {
                continue;
            }
            has_cash_line = true;
            let amount = match line.side() {
                DebitCredit::Debit => line.amount(),
                DebitCredit::Credit => line.amount().checked_neg()?,
            };
            total = total.checked_add(amount)?;
            entry_cash_total = entry_cash_total.checked_add(amount)?;
        }

        if !has_cash_line {
            continue;
        }

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

            let category_total = category_totals
                .entry(classification.category())
                .or_insert(zero);
            *category_total = category_total.checked_add(amount)?;

            let subcategory_key = (
                classification.category(),
                classification.subcategory().to_owned(),
            );
            let subcategory_total = subcategory_totals.entry(subcategory_key).or_insert(zero);
            *subcategory_total = subcategory_total.checked_add(amount)?;
        }

        if entry_cash_total.checked_cmp(entry_classified_total)? != std::cmp::Ordering::Equal {
            return Err(ReportingError::CashFlowClassificationMismatch {
                journal_position,
                cash_amount: entry_cash_total,
                classified_amount: entry_classified_total,
            });
        }
    }

    let mut category_totals: Vec<CashFlowCategoryTotal> = category_totals
        .into_iter()
        .map(|(category, amount)| CashFlowCategoryTotal::new(category, amount))
        .collect();
    category_totals.sort_by_key(CashFlowCategoryTotal::category);
    let mut subcategory_totals: Vec<CashFlowSubcategoryTotal> = subcategory_totals
        .into_iter()
        .map(|((category, subcategory), amount)| {
            CashFlowSubcategoryTotal::new(category, subcategory, amount)
        })
        .collect();
    subcategory_totals.sort_by_key(|total| (total.category(), total.subcategory().to_owned()));

    Ok(CashFlowReport::new(
        start,
        end,
        movements,
        category_totals,
        subcategory_totals,
        total,
    ))
}
