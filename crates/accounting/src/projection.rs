use std::collections::BTreeMap;

use property_simulation_domain::{AccountId, Money, YearMonth};

use crate::{AccountCategory, AccountError, BalanceSide, CashFlowCategory, Ledger};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountBalance {
    pub account_id: AccountId,
    pub side: BalanceSide,
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceSheet {
    pub as_of: YearMonth,
    pub accounts: Vec<AccountBalance>,
    pub assets: Money,
    pub liabilities: Money,
    pub equity: Money,
    pub cumulative_net_income: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeStatement {
    pub start: YearMonth,
    pub end: YearMonth,
    pub income_accounts: Vec<AccountBalance>,
    pub expense_accounts: Vec<AccountBalance>,
    pub income: Money,
    pub expenses: Money,
    pub net_income: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashFlowTotals {
    pub operating: Money,
    pub investing: Money,
    pub financing: Money,
}
impl CashFlowTotals {
    pub fn net_change(self) -> Result<Money, AccountError> {
        Ok(self
            .operating
            .checked_add(self.investing)?
            .checked_add(self.financing)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashFlowStatement {
    pub start: YearMonth,
    pub end: YearMonth,
    pub accounts: BTreeMap<AccountId, CashFlowTotals>,
    pub totals: CashFlowTotals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinancialProjection {
    pub balance_sheet: BalanceSheet,
    pub income_statement: IncomeStatement,
    pub cash_flow_statement: CashFlowStatement,
}

impl Ledger {
    /// Builds balance sheet at `end`, and income/cash-flow statements over inclusive `[start, end]`.
    pub fn project(
        &self,
        start: YearMonth,
        end: YearMonth,
        cash_accounts: &[AccountId],
    ) -> Result<FinancialProjection, AccountError> {
        if start > end {
            return Err(AccountError::InvalidProjectionRange);
        }
        let zero = Money::new(0.into(), self.currency());
        let income_statement = self.income_statement(start, end)?;
        let mut accounts = Vec::new();
        let (mut assets, mut liabilities, mut equity) = (zero, zero, zero);
        let mut cumulative_net_income = zero;
        for account in self.chart().accounts_by_code() {
            let (side, amount) = self.balance_through(account.id(), end)?;
            let signed = signed_category_amount(account.category(), side, amount)?;
            match account.category() {
                AccountCategory::Asset => {
                    assets = assets.checked_add(signed)?;
                    accounts.push(AccountBalance {
                        account_id: account.id(),
                        side,
                        amount,
                    });
                }
                AccountCategory::Liability => {
                    liabilities = liabilities.checked_add(signed)?;
                    accounts.push(AccountBalance {
                        account_id: account.id(),
                        side,
                        amount,
                    });
                }
                AccountCategory::Equity => {
                    equity = equity.checked_add(signed)?;
                    accounts.push(AccountBalance {
                        account_id: account.id(),
                        side,
                        amount,
                    });
                }
                AccountCategory::Income => {
                    cumulative_net_income = cumulative_net_income.checked_add(signed)?
                }
                AccountCategory::Expense => {
                    cumulative_net_income = cumulative_net_income.checked_sub(signed)?
                }
            }
        }
        equity = equity.checked_add(cumulative_net_income)?;
        let cash_flow_statement = self.cash_flow_statement(start, end, cash_accounts)?;
        Ok(FinancialProjection {
            balance_sheet: BalanceSheet {
                as_of: end,
                accounts,
                assets,
                liabilities,
                equity,
                cumulative_net_income,
            },
            income_statement,
            cash_flow_statement,
        })
    }

    fn income_statement(
        &self,
        start: YearMonth,
        end: YearMonth,
    ) -> Result<IncomeStatement, AccountError> {
        let zero = Money::new(0.into(), self.currency());
        let (mut income, mut expenses) = (zero, zero);
        let mut income_accounts = Vec::new();
        let mut expense_accounts = Vec::new();
        for account in self.chart().accounts_by_code() {
            if !matches!(
                account.category(),
                AccountCategory::Income | AccountCategory::Expense
            ) {
                continue;
            }
            let mut signed = zero;
            for tx in self
                .transactions()
                .filter(|tx| tx.period() >= start && tx.period() <= end)
            {
                for entry in tx
                    .entries()
                    .iter()
                    .filter(|e| e.account_id() == account.id())
                {
                    signed = match entry.side() {
                        BalanceSide::Debit => signed.checked_add(entry.amount()),
                        BalanceSide::Credit => signed.checked_sub(entry.amount()),
                    }
                    .map_err(AccountError::Arithmetic)?;
                }
            }
            let amount = Money::new(signed.amount().abs(), self.currency());
            let side = if signed.amount().is_zero() {
                account.normal_balance()
            } else if signed.amount().is_sign_negative() {
                BalanceSide::Credit
            } else {
                BalanceSide::Debit
            };
            let balance = AccountBalance {
                account_id: account.id(),
                side,
                amount,
            };
            match account.category() {
                AccountCategory::Income => {
                    let value = Money::new(-signed.amount(), self.currency());
                    income = income.checked_add(value)?;
                    income_accounts.push(balance);
                }
                AccountCategory::Expense => {
                    expenses = expenses.checked_add(signed)?;
                    expense_accounts.push(balance);
                }
                _ => unreachable!(),
            }
        }
        let net_income = income.checked_sub(expenses)?;
        Ok(IncomeStatement {
            start,
            end,
            income_accounts,
            expense_accounts,
            income,
            expenses,
            net_income,
        })
    }

    fn cash_flow_statement(
        &self,
        start: YearMonth,
        end: YearMonth,
        cash_accounts: &[AccountId],
    ) -> Result<CashFlowStatement, AccountError> {
        let zero = Money::new(0.into(), self.currency());
        if cash_accounts.is_empty() {
            return Err(AccountError::CashFlowAccountsRequired);
        }
        for id in cash_accounts {
            let account = self
                .chart()
                .get(*id)
                .ok_or(AccountError::UnknownPostingAccount(*id))?;
            if account.category() != AccountCategory::Asset || account.is_contra() {
                return Err(AccountError::CashFlowAccountMustBeAsset(*id));
            }
        }
        let mut by_account = cash_accounts
            .iter()
            .map(|id| {
                (
                    *id,
                    CashFlowTotals {
                        operating: zero,
                        investing: zero,
                        financing: zero,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        for tx in self
            .transactions()
            .filter(|tx| tx.period() >= start && tx.period() <= end)
        {
            for entry in tx.entries() {
                let is_cash = by_account.contains_key(&entry.account_id());
                match (is_cash, entry.cash_flow_category()) {
                    (true, None) => {
                        return Err(AccountError::CashFlowCategoryMissing(entry.account_id()));
                    }
                    (false, Some(_)) => {
                        return Err(AccountError::CashFlowCategoryOnNonCashAccount(
                            entry.account_id(),
                        ));
                    }
                    (true, Some(category)) => {
                        let amount = match entry.side() {
                            BalanceSide::Debit => entry.amount(),
                            BalanceSide::Credit => {
                                Money::new(-entry.amount().amount(), self.currency())
                            }
                        };
                        let totals = by_account
                            .get_mut(&entry.account_id())
                            .expect("cash account was checked");
                        let target = match category {
                            CashFlowCategory::Operating => &mut totals.operating,
                            CashFlowCategory::Investing => &mut totals.investing,
                            CashFlowCategory::Financing => &mut totals.financing,
                        };
                        *target = target.checked_add(amount)?;
                    }
                    _ => {}
                }
            }
        }
        let mut totals = CashFlowTotals {
            operating: zero,
            investing: zero,
            financing: zero,
        };
        for value in by_account.values() {
            totals.operating = totals.operating.checked_add(value.operating)?;
            totals.investing = totals.investing.checked_add(value.investing)?;
            totals.financing = totals.financing.checked_add(value.financing)?;
        }
        Ok(CashFlowStatement {
            start,
            end,
            accounts: by_account,
            totals,
        })
    }
}

fn signed_category_amount(
    category: AccountCategory,
    side: BalanceSide,
    amount: Money,
) -> Result<Money, AccountError> {
    let positive = match category {
        AccountCategory::Asset | AccountCategory::Expense => side == BalanceSide::Debit,
        AccountCategory::Liability | AccountCategory::Equity | AccountCategory::Income => {
            side == BalanceSide::Credit
        }
    };
    Ok(if positive {
        amount
    } else {
        Money::new(-amount.amount(), amount.currency())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Account, AccountCode, ChartOfAccounts, LedgerEntry, LedgerTransaction, OpeningBalance,
        OpeningBalances,
    };
    use property_simulation_domain::{Currency, LedgerTransactionId};
    use rust_decimal::Decimal;

    fn id(n: u8) -> AccountId {
        let mut bytes = [0; 16];
        bytes[15] = n;
        AccountId::from_bytes(bytes)
    }
    fn txid(n: u8) -> LedgerTransactionId {
        let mut bytes = [0; 16];
        bytes[15] = n;
        LedgerTransactionId::from_bytes(bytes)
    }
    fn m(n: i64, c: Currency) -> Money {
        Money::new(Decimal::from(n), c)
    }
    fn account(n: u8, code: u16, category: AccountCategory, name: &str) -> Account {
        Account::new(
            id(n),
            AccountCode::new(code).unwrap(),
            category,
            false,
            name,
        )
        .unwrap()
    }
    fn tx(n: u8, entries: Vec<LedgerEntry>) -> LedgerTransaction {
        LedgerTransaction::new(txid(n), YearMonth::new(2025, 1).unwrap(), entries)
    }
    fn entry(
        a: u8,
        side: BalanceSide,
        amount: Money,
        category: Option<CashFlowCategory>,
    ) -> LedgerEntry {
        let entry = LedgerEntry::new(id(a), side, amount);
        category.map_or(entry, |category| entry.with_cash_flow_category(category))
    }

    #[test]
    fn l06_projects_balanced_statements_and_excludes_opening_cash_from_flow() {
        let c = Currency::from_code("CAD").unwrap();
        let chart = ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset, "Cash"),
            account(2, 1100, AccountCategory::Asset, "Property"),
            account(3, 2000, AccountCategory::Liability, "Mortgage"),
            account(4, 3000, AccountCategory::Equity, "Opening equity"),
            account(5, 4000, AccountCategory::Income, "Rent"),
            account(6, 5000, AccountCategory::Expense, "Interest"),
            account(7, 5001, AccountCategory::Expense, "Repairs"),
            account(8, 5002, AccountCategory::Expense, "Tax"),
        ])
        .unwrap();
        let opening = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(id(1), BalanceSide::Debit, m(20_000, c)).unwrap(),
                OpeningBalance::new(id(2), BalanceSide::Debit, m(300_000, c)).unwrap(),
                OpeningBalance::new(id(3), BalanceSide::Credit, m(200_000, c)).unwrap(),
                OpeningBalance::new(id(4), BalanceSide::Credit, m(120_000, c)).unwrap(),
            ],
            None,
        )
        .unwrap();
        let mut ledger = Ledger::new(chart, opening).unwrap();
        let month = YearMonth::new(2025, 1).unwrap();
        ledger
            .post(LedgerTransaction::new(
                txid(1),
                month,
                [
                    entry(3, BalanceSide::Debit, m(1_000, c), None),
                    entry(6, BalanceSide::Debit, m(500, c), None),
                    entry(
                        1,
                        BalanceSide::Credit,
                        m(500, c),
                        Some(CashFlowCategory::Operating),
                    ),
                    entry(
                        1,
                        BalanceSide::Credit,
                        m(1_000, c),
                        Some(CashFlowCategory::Financing),
                    ),
                ],
            ))
            .unwrap();
        ledger
            .post(tx(
                2,
                vec![
                    entry(7, BalanceSide::Debit, m(800, c), None),
                    entry(
                        1,
                        BalanceSide::Credit,
                        m(800, c),
                        Some(CashFlowCategory::Operating),
                    ),
                ],
            ))
            .unwrap();
        ledger
            .post(tx(
                3,
                vec![
                    entry(
                        1,
                        BalanceSide::Debit,
                        m(2_000, c),
                        Some(CashFlowCategory::Operating),
                    ),
                    entry(5, BalanceSide::Credit, m(2_000, c), None),
                ],
            ))
            .unwrap();
        ledger
            .post(tx(
                4,
                vec![
                    entry(8, BalanceSide::Debit, m(300, c), None),
                    entry(
                        1,
                        BalanceSide::Credit,
                        m(300, c),
                        Some(CashFlowCategory::Operating),
                    ),
                ],
            ))
            .unwrap();
        let projection = ledger.project(month, month, &[id(1)]).unwrap();
        let bs = projection.balance_sheet;
        assert_eq!(bs.assets, m(319_400, c));
        assert_eq!(bs.liabilities, m(199_000, c));
        assert_eq!(bs.equity, m(120_400, c));
        assert_eq!(bs.cumulative_net_income, m(400, c));
        assert_eq!(
            bs.accounts.len(),
            4,
            "only asset, liability, and equity accounts belong on the balance sheet"
        );
        let cf = projection.cash_flow_statement;
        assert_eq!(cf.totals.operating, m(400, c));
        assert_eq!(cf.totals.financing, m(-1_000, c));
        assert_eq!(cf.totals.net_change().unwrap(), m(-600, c));
        assert_eq!(projection.income_statement.net_income, m(400, c));
    }

    #[test]
    fn rejects_missing_cash_classification_and_non_cash_classification() {
        let c = Currency::from_code("CAD").unwrap();
        let chart = ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset, "Cash"),
            account(2, 4000, AccountCategory::Income, "Rent"),
            account(3, 3000, AccountCategory::Equity, "Equity"),
        ])
        .unwrap();
        let opening = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(id(1), BalanceSide::Debit, m(1, c)).unwrap(),
                OpeningBalance::new(id(3), BalanceSide::Credit, m(1, c)).unwrap(),
            ],
            None,
        )
        .unwrap();
        let mut ledger = Ledger::new(chart, opening).unwrap();
        let month = YearMonth::new(2025, 1).unwrap();
        ledger
            .post(LedgerTransaction::new(
                txid(1),
                month,
                [
                    LedgerEntry::new(id(2), BalanceSide::Debit, m(1, c)),
                    LedgerEntry::new(id(1), BalanceSide::Credit, m(1, c)),
                ],
            ))
            .unwrap();
        assert_eq!(
            ledger.project(month, month, &[id(1)]),
            Err(AccountError::CashFlowCategoryMissing(id(1)))
        );
        let mut other =
            Ledger::new(ledger.chart().clone(), ledger.opening_balances().clone()).unwrap();
        other
            .post(LedgerTransaction::new(
                txid(2),
                month,
                [
                    entry(
                        2,
                        BalanceSide::Debit,
                        m(1, c),
                        Some(CashFlowCategory::Operating),
                    ),
                    entry(
                        1,
                        BalanceSide::Credit,
                        m(1, c),
                        Some(CashFlowCategory::Operating),
                    ),
                ],
            ))
            .unwrap();
        assert_eq!(
            other.project(month, month, &[id(1)]),
            Err(AccountError::CashFlowCategoryOnNonCashAccount(id(2)))
        );
        assert_eq!(
            other.project(month, month, &[]),
            Err(AccountError::CashFlowAccountsRequired)
        );
        let next_month = month.checked_add_months(1).unwrap();
        assert_eq!(
            other.project(next_month, month, &[id(1)]),
            Err(AccountError::InvalidProjectionRange)
        );
    }

    #[test]
    fn inclusive_periods_exclude_postings_outside_each_projection_window() {
        let c = Currency::from_code("CAD").unwrap();
        let chart = ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset, "Cash"),
            account(2, 3000, AccountCategory::Equity, "Opening equity"),
            account(3, 4000, AccountCategory::Income, "Rent"),
        ])
        .unwrap();
        let opening = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(id(1), BalanceSide::Debit, m(10, c)).unwrap(),
                OpeningBalance::new(id(2), BalanceSide::Credit, m(10, c)).unwrap(),
            ],
            None,
        )
        .unwrap();
        let mut ledger = Ledger::new(chart, opening).unwrap();
        let jan = YearMonth::new(2025, 1).unwrap();
        let feb = YearMonth::new(2025, 2).unwrap();
        for (n, month, rent) in [(1, jan, 3), (2, feb, 4)] {
            ledger
                .post(LedgerTransaction::new(
                    txid(n),
                    month,
                    [
                        entry(
                            1,
                            BalanceSide::Debit,
                            m(rent, c),
                            Some(CashFlowCategory::Operating),
                        ),
                        entry(3, BalanceSide::Credit, m(rent, c), None),
                    ],
                ))
                .unwrap();
        }

        let january = ledger.project(jan, jan, &[id(1)]).unwrap();
        assert_eq!(january.balance_sheet.assets, m(13, c));
        assert_eq!(january.balance_sheet.equity, m(13, c));
        assert_eq!(january.income_statement.net_income, m(3, c));
        assert_eq!(january.cash_flow_statement.totals.operating, m(3, c));

        let february = ledger.project(feb, feb, &[id(1)]).unwrap();
        assert_eq!(february.balance_sheet.assets, m(17, c));
        assert_eq!(february.balance_sheet.equity, m(17, c));
        assert_eq!(february.balance_sheet.cumulative_net_income, m(7, c));
        assert_eq!(february.income_statement.net_income, m(4, c));
        assert_eq!(february.cash_flow_statement.totals.operating, m(4, c));

        let march = feb.checked_add_months(1).unwrap();
        let empty_month = ledger.project(march, march, &[id(1)]).unwrap();
        let rent_balance = empty_month
            .income_statement
            .income_accounts
            .iter()
            .find(|account| account.account_id == id(3))
            .unwrap();
        assert_eq!(rent_balance.side, BalanceSide::Credit);
        assert_eq!(rent_balance.amount, m(0, c));
    }

    #[test]
    fn l07_capitalized_interest_affects_expense_and_debt_but_not_cash_flow() {
        let c = Currency::from_code("CAD").unwrap();
        let chart = ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset, "Cash"),
            account(2, 2000, AccountCategory::Liability, "Mortgage"),
            account(3, 3000, AccountCategory::Equity, "Opening equity"),
            account(4, 5000, AccountCategory::Expense, "Interest"),
        ])
        .unwrap();
        let opening = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(id(1), BalanceSide::Debit, m(100, c)).unwrap(),
                OpeningBalance::new(id(2), BalanceSide::Credit, m(50, c)).unwrap(),
                OpeningBalance::new(id(3), BalanceSide::Credit, m(50, c)).unwrap(),
            ],
            None,
        )
        .unwrap();
        let mut ledger = Ledger::new(chart, opening).unwrap();
        let month = YearMonth::new(2025, 1).unwrap();
        ledger
            .post(LedgerTransaction::new(
                txid(1),
                month,
                [
                    entry(4, BalanceSide::Debit, m(100, c), None),
                    entry(
                        1,
                        BalanceSide::Credit,
                        m(50, c),
                        Some(CashFlowCategory::Operating),
                    ),
                    entry(2, BalanceSide::Credit, m(50, c), None),
                ],
            ))
            .unwrap();

        let projection = ledger.project(month, month, &[id(1)]).unwrap();
        assert_eq!(projection.balance_sheet.assets, m(50, c));
        assert_eq!(projection.balance_sheet.liabilities, m(100, c));
        assert_eq!(projection.balance_sheet.equity, m(-50, c));
        assert_eq!(projection.income_statement.net_income, m(-100, c));
        assert_eq!(projection.cash_flow_statement.totals.operating, m(-50, c));
        assert_eq!(
            projection.cash_flow_statement.totals.net_change().unwrap(),
            m(-50, c)
        );
    }

    #[test]
    fn aggregates_multiple_cash_accounts_by_activity_category() {
        let c = Currency::from_code("CAD").unwrap();
        let chart = ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset, "Operating cash"),
            account(2, 1001, AccountCategory::Asset, "Reserve cash"),
            account(3, 1100, AccountCategory::Asset, "Property"),
            account(4, 2000, AccountCategory::Liability, "Mortgage"),
            account(5, 3000, AccountCategory::Equity, "Opening equity"),
        ])
        .unwrap();
        let opening = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(id(1), BalanceSide::Debit, m(10, c)).unwrap(),
                OpeningBalance::new(id(2), BalanceSide::Debit, m(10, c)).unwrap(),
                OpeningBalance::new(id(3), BalanceSide::Debit, m(100, c)).unwrap(),
                OpeningBalance::new(id(4), BalanceSide::Credit, m(50, c)).unwrap(),
                OpeningBalance::new(id(5), BalanceSide::Credit, m(70, c)).unwrap(),
            ],
            None,
        )
        .unwrap();
        let mut ledger = Ledger::new(chart, opening).unwrap();
        let month = YearMonth::new(2025, 1).unwrap();
        ledger
            .post(LedgerTransaction::new(
                txid(1),
                month,
                [
                    entry(
                        1,
                        BalanceSide::Debit,
                        m(20, c),
                        Some(CashFlowCategory::Financing),
                    ),
                    entry(4, BalanceSide::Credit, m(20, c), None),
                ],
            ))
            .unwrap();
        ledger
            .post(LedgerTransaction::new(
                txid(2),
                month,
                [
                    entry(3, BalanceSide::Debit, m(7, c), None),
                    entry(
                        2,
                        BalanceSide::Credit,
                        m(7, c),
                        Some(CashFlowCategory::Investing),
                    ),
                ],
            ))
            .unwrap();

        let projection = ledger.project(month, month, &[id(1), id(2)]).unwrap();
        let cash_flow = projection.cash_flow_statement;
        assert_eq!(cash_flow.accounts[&id(1)].financing, m(20, c));
        assert_eq!(cash_flow.accounts[&id(2)].investing, m(-7, c));
        assert_eq!(cash_flow.totals.financing, m(20, c));
        assert_eq!(cash_flow.totals.investing, m(-7, c));
        assert_eq!(cash_flow.totals.net_change().unwrap(), m(13, c));
    }
}
