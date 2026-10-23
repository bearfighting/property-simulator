use super::validation::{balances_through, zero};
use crate::{BalanceSheet, BalanceSheetLine, BalanceSheetTotals, ReportingError, ReportingInput};
use property_simulation_account::{AccountClass, DebitCredit};
use property_simulation_month::Month;

/// Derive assets, liabilities, equity, and cumulative earnings through `as_of`.
pub fn balance_sheet(
    input: ReportingInput<'_>,
    as_of: Month,
) -> Result<BalanceSheet, ReportingError> {
    let balances = balances_through(input, as_of)?;
    let zero = zero(input)?;
    let mut assets = Vec::new();
    let mut liabilities = Vec::new();
    let mut equity_accounts = Vec::new();
    let mut total_assets = zero;
    let mut total_liabilities = zero;
    let mut equity_account_total = zero;
    let mut net_income = zero;

    let mut accounts = input.accounts().iter().collect::<Vec<_>>();
    accounts.sort_by_key(|account| account.code());

    for account in accounts {
        let net_debit = balances
            .get(&account.code())
            .copied()
            .ok_or(ReportingError::UnknownAccount(account.code()))?;
        let natural_balance = match account.class().normal_balance() {
            DebitCredit::Debit => net_debit,
            DebitCredit::Credit => net_debit.checked_neg()?,
        };
        let line = BalanceSheetLine::new(account.code(), natural_balance);

        match account.class() {
            AccountClass::Asset => {
                total_assets = total_assets.checked_add(natural_balance)?;
                assets.push(line);
            }
            AccountClass::Liability => {
                total_liabilities = total_liabilities.checked_add(natural_balance)?;
                liabilities.push(line);
            }
            AccountClass::Equity => {
                equity_account_total = equity_account_total.checked_add(natural_balance)?;
                equity_accounts.push(line);
            }
            AccountClass::Income => net_income = net_income.checked_add(natural_balance)?,
            AccountClass::Expense => net_income = net_income.checked_sub(natural_balance)?,
        }
    }

    let total_equity = equity_account_total.checked_add(net_income)?;
    let liabilities_and_equity = total_liabilities.checked_add(total_equity)?;
    if total_assets.checked_cmp(liabilities_and_equity)? != std::cmp::Ordering::Equal {
        return Err(ReportingError::UnbalancedBalanceSheet {
            assets: total_assets,
            liabilities_and_equity,
        });
    }

    Ok(BalanceSheet::new(
        as_of,
        assets,
        liabilities,
        equity_accounts,
        BalanceSheetTotals {
            assets: total_assets,
            liabilities: total_liabilities,
            equity_accounts: equity_account_total,
            net_income,
            equity: total_equity,
        },
    ))
}
