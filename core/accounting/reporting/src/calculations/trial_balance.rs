use super::validation::{balances_through, zero};
use crate::{AccountBalanceLine, ReportingError, ReportingInput, TrialBalance};
use property_simulation_month::Month;

/// Derive every account's net debit/credit balance through and including `as_of`.
pub fn trial_balance(
    input: ReportingInput<'_>,
    as_of: Month,
) -> Result<TrialBalance, ReportingError> {
    let balances = balances_through(input, as_of)?;

    let zero = zero(input)?;
    let mut account_lines = Vec::with_capacity(input.accounts().len());
    let mut debit_total = zero;
    let mut credit_total = zero;

    let mut accounts = input.accounts().iter().collect::<Vec<_>>();
    accounts.sort_by_key(|account| account.code());

    for account in accounts {
        let net_debit = balances.get(&account.code()).copied().unwrap_or(zero);
        let (debit, credit) = match net_debit.checked_cmp(zero)? {
            std::cmp::Ordering::Greater => (net_debit, zero),
            std::cmp::Ordering::Less => (zero, net_debit.abs()?),
            std::cmp::Ordering::Equal => (zero, zero),
        };
        debit_total = debit_total.checked_add(debit)?;
        credit_total = credit_total.checked_add(credit)?;
        account_lines.push(AccountBalanceLine::new(account.code(), debit, credit));
    }

    if debit_total.checked_cmp(credit_total)? != std::cmp::Ordering::Equal {
        return Err(ReportingError::UnbalancedTrialBalance {
            debits: debit_total,
            credits: credit_total,
        });
    }

    Ok(TrialBalance::new(
        as_of,
        account_lines,
        debit_total,
        credit_total,
    ))
}
