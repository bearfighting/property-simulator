use crate::{ReportingError, ReportingInput};
use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_journal_entry::JournalEntry;
use property_simulation_money::Money;
use property_simulation_month::Month;
use std::collections::BTreeMap;

pub(super) fn validate_period(start: Month, end: Month) -> Result<(), ReportingError> {
    if start.is_initial() || start > end {
        Err(ReportingError::InvalidPeriod { start, end })
    } else {
        Ok(())
    }
}

pub(super) fn validate_all(input: ReportingInput<'_>) -> Result<(), ReportingError> {
    super::audit::audit(input).map(|_| ())
}

pub(super) fn zero_balances(
    input: ReportingInput<'_>,
) -> Result<BTreeMap<AccountCode, Money>, ReportingError> {
    let zero = zero(input)?;
    Ok(input
        .accounts()
        .iter()
        .map(|account| (account.code(), zero))
        .collect())
}

pub(super) fn balances_through(
    input: ReportingInput<'_>,
    as_of: Month,
) -> Result<BTreeMap<AccountCode, Money>, ReportingError> {
    let mut balances = zero_balances(input)?;
    let mut previous_month = None;

    for entry in input.journal() {
        validate_entry(input, entry)?;
        validate_month_order(&mut previous_month, entry)?;
        if entry.month() <= as_of {
            apply_entry(&mut balances, entry)?;
        }
    }

    Ok(balances)
}

pub(super) fn validate_entry(
    input: ReportingInput<'_>,
    entry: &JournalEntry,
) -> Result<(), ReportingError> {
    let zero = zero(input)?;
    let mut debits = zero;
    let mut credits = zero;

    for line in entry.lines() {
        if input.accounts().get(line.account()).is_none() {
            return Err(ReportingError::UnknownAccount(line.account()));
        }
        validate_currency(input, line.amount())?;
        match line.side() {
            DebitCredit::Debit => debits = debits.checked_add(line.amount())?,
            DebitCredit::Credit => credits = credits.checked_add(line.amount())?,
        }
    }

    if debits.checked_cmp(credits)? != std::cmp::Ordering::Equal {
        return Err(ReportingError::UnbalancedJournalEntry {
            month: entry.month(),
            debits,
            credits,
        });
    }

    Ok(())
}

pub(super) fn validate_currency(
    input: ReportingInput<'_>,
    amount: Money,
) -> Result<(), ReportingError> {
    let expected = input.currency();
    let actual = amount.currency();
    if expected == actual {
        Ok(())
    } else {
        Err(ReportingError::CurrencyMismatch { expected, actual })
    }
}

pub(super) fn validate_month_order(
    previous_month: &mut Option<Month>,
    entry: &JournalEntry,
) -> Result<(), ReportingError> {
    if let Some(previous) = previous_month {
        if entry.month() < *previous {
            return Err(ReportingError::MonthRegression {
                previous: *previous,
                attempted: entry.month(),
            });
        }
    }
    *previous_month = Some(entry.month());
    Ok(())
}

pub(super) fn apply_entry(
    balances: &mut BTreeMap<AccountCode, Money>,
    entry: &JournalEntry,
) -> Result<(), ReportingError> {
    for line in entry.lines() {
        apply_amount(balances, line.account(), line.side(), line.amount())?;
    }
    Ok(())
}

pub(super) fn apply_selected_lines(
    input: ReportingInput<'_>,
    balances: &mut BTreeMap<AccountCode, Money>,
    entry: &JournalEntry,
    mut select: impl FnMut(property_simulation_account::AccountClass) -> bool,
) -> Result<(), ReportingError> {
    for line in entry.lines() {
        let account = input
            .accounts()
            .get(line.account())
            .ok_or(ReportingError::UnknownAccount(line.account()))?;
        if select(account.class()) {
            apply_amount(balances, line.account(), line.side(), line.amount())?;
        }
    }
    Ok(())
}

fn apply_amount(
    balances: &mut BTreeMap<AccountCode, Money>,
    account: AccountCode,
    side: DebitCredit,
    amount: Money,
) -> Result<(), ReportingError> {
    let balance = balances
        .get_mut(&account)
        .ok_or(ReportingError::UnknownAccount(account))?;
    let signed = match side {
        DebitCredit::Debit => amount,
        DebitCredit::Credit => amount.checked_neg()?,
    };
    *balance = balance.checked_add(signed)?;
    Ok(())
}

pub(super) fn zero(input: ReportingInput<'_>) -> Result<Money, ReportingError> {
    Ok(Money::from_parts(0, 0, input.currency())?)
}
