use std::collections::BTreeMap;

use property_simulation_domain::{AccountId, Currency, Money, YearMonth};

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
        sum_signed(
            [self.operating, self.investing, self.financing],
            self.operating.currency(),
        )
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

/// Validated projection inputs tied to an immutable ledger borrow.
/// Construct this before running the projection to surface correctable input errors up front.
pub struct ValidatedProjection<'a> {
    ledger: &'a Ledger,
    start: YearMonth,
    end: YearMonth,
    cash_accounts: Vec<AccountId>,
}

impl ValidatedProjection<'_> {
    pub fn project(&self) -> Result<FinancialProjection, AccountError> {
        self.ledger
            .calculate_projection(self.start, self.end, &self.cash_accounts)
    }
}

impl Ledger {
    /// Convenience API: validates inputs, then computes the inclusive projection.
    pub fn project(
        &self,
        start: YearMonth,
        end: YearMonth,
        cash_accounts: &[AccountId],
    ) -> Result<FinancialProjection, AccountError> {
        self.validate_projection(start, end, cash_accounts)?
            .project()
    }

    /// Validates range, configured cash accounts, and classifications across the full ledger.
    pub fn validate_projection<'a>(
        &'a self,
        start: YearMonth,
        end: YearMonth,
        cash_accounts: &[AccountId],
    ) -> Result<ValidatedProjection<'a>, AccountError> {
        if start > end {
            return Err(AccountError::InvalidProjectionRange);
        }
        if cash_accounts.is_empty() {
            return Err(AccountError::CashFlowAccountsRequired);
        }
        let selected = cash_accounts
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if selected.len() != cash_accounts.len() {
            let mut seen = std::collections::BTreeSet::new();
            let duplicate = cash_accounts
                .iter()
                .find(|id| !seen.insert(**id))
                .copied()
                .expect("a size mismatch means there is a duplicate");
            return Err(AccountError::DuplicateCashFlowAccount(duplicate));
        }
        for id in cash_accounts {
            let account = self
                .chart()
                .get(*id)
                .ok_or(AccountError::UnknownPostingAccount(*id))?;
            if account.category() != AccountCategory::Asset
                || account.is_contra()
                || !account.is_cash_position()
            {
                return Err(AccountError::CashFlowAccountMustBeAsset(*id));
            }
        }
        if selected != self.cash_accounts().collect() {
            return Err(AccountError::CashFlowAccountsMismatch);
        }
        Ok(ValidatedProjection {
            ledger: self,
            start,
            end,
            cash_accounts: cash_accounts.to_vec(),
        })
    }

    /// Computes a projection from inputs returned by `validate_projection`.
    fn calculate_projection(
        &self,
        start: YearMonth,
        end: YearMonth,
        cash_accounts: &[AccountId],
    ) -> Result<FinancialProjection, AccountError> {
        let income_statement = self.income_statement(start, end)?;
        let mut accounts = Vec::new();
        let mut asset_amounts = Vec::new();
        let mut liability_amounts = Vec::new();
        let mut equity_amounts = Vec::new();
        let mut cumulative_income_amounts = Vec::new();
        for account in self.chart().accounts_by_code() {
            let (side, amount) = self.balance_through(account.id(), end)?;
            let signed = signed_category_amount(account.category(), side, amount)?;
            match account.category() {
                AccountCategory::Asset => {
                    asset_amounts.push(signed);
                    accounts.push(AccountBalance {
                        account_id: account.id(),
                        side,
                        amount,
                    });
                }
                AccountCategory::Liability => {
                    liability_amounts.push(signed);
                    accounts.push(AccountBalance {
                        account_id: account.id(),
                        side,
                        amount,
                    });
                }
                AccountCategory::Equity => {
                    equity_amounts.push(signed);
                    accounts.push(AccountBalance {
                        account_id: account.id(),
                        side,
                        amount,
                    });
                }
                AccountCategory::Income => cumulative_income_amounts.push(signed),
                AccountCategory::Expense => cumulative_income_amounts.push(signed.checked_neg()?),
            }
        }
        let assets = sum_signed(asset_amounts, self.currency())?;
        let liabilities = sum_signed(liability_amounts, self.currency())?;
        let cumulative_net_income = sum_signed(cumulative_income_amounts, self.currency())?;
        equity_amounts.push(cumulative_net_income);
        let equity = sum_signed(equity_amounts, self.currency())?;
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
        let mut income_amounts = Vec::new();
        let mut expense_amounts = Vec::new();
        let mut income_accounts = Vec::new();
        let mut expense_accounts = Vec::new();
        for account in self.chart().accounts_by_code() {
            if !matches!(
                account.category(),
                AccountCategory::Income | AccountCategory::Expense
            ) {
                continue;
            }
            let mut signed_entries = Vec::new();
            for tx in self
                .transactions()
                .filter(|tx| tx.period() >= start && tx.period() <= end)
            {
                for entry in tx
                    .entries()
                    .iter()
                    .filter(|e| e.account_id() == account.id())
                {
                    signed_entries.push(match entry.side() {
                        BalanceSide::Debit => entry.amount(),
                        BalanceSide::Credit => entry.amount().checked_neg()?,
                    });
                }
            }
            let signed = sum_signed(signed_entries, self.currency())?;
            let amount = signed.abs()?;
            let side = if signed.is_zero() {
                account.normal_balance()
            } else if signed.is_negative() {
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
                    let value = signed.checked_neg()?;
                    income_amounts.push(value);
                    income_accounts.push(balance);
                }
                AccountCategory::Expense => {
                    expense_amounts.push(signed);
                    expense_accounts.push(balance);
                }
                _ => unreachable!(),
            }
        }
        let income = sum_signed(income_amounts, self.currency())?;
        let expenses = sum_signed(expense_amounts, self.currency())?;
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
        let mut postings_by_account = cash_accounts
            .iter()
            .map(|id| (*id, [Vec::new(), Vec::new(), Vec::new()]))
            .collect::<BTreeMap<_, _>>();
        for tx in self
            .transactions()
            .filter(|tx| tx.period() >= start && tx.period() <= end)
        {
            for entry in tx.entries() {
                let is_cash = postings_by_account.contains_key(&entry.account_id());
                if is_cash {
                    let category = entry
                        .cash_flow_category()
                        .expect("cash posting classification is validated before posting");
                    let amount = match entry.side() {
                        BalanceSide::Debit => entry.amount(),
                        BalanceSide::Credit => entry.amount().checked_neg()?,
                    };
                    let postings = postings_by_account
                        .get_mut(&entry.account_id())
                        .expect("cash account was checked");
                    let category_index = match category {
                        CashFlowCategory::Operating => 0,
                        CashFlowCategory::Investing => 1,
                        CashFlowCategory::Financing => 2,
                    };
                    postings[category_index].push(amount);
                }
            }
        }
        let by_account = postings_by_account
            .into_iter()
            .map(|(id, postings)| {
                Ok((
                    id,
                    CashFlowTotals {
                        operating: sum_signed(postings[0].iter().copied(), self.currency())?,
                        investing: sum_signed(postings[1].iter().copied(), self.currency())?,
                        financing: sum_signed(postings[2].iter().copied(), self.currency())?,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>, AccountError>>()?;
        let totals = CashFlowTotals {
            operating: sum_signed(
                by_account.values().map(|value| value.operating),
                self.currency(),
            )?,
            investing: sum_signed(
                by_account.values().map(|value| value.investing),
                self.currency(),
            )?,
            financing: sum_signed(
                by_account.values().map(|value| value.financing),
                self.currency(),
            )?,
        };
        Ok(CashFlowStatement {
            start,
            end,
            accounts: by_account,
            totals,
        })
    }
}

/// Adds signed amounts without overflowing on intermediate values that later cancel.
/// An error is returned only when the resulting net value cannot fit in `Money`.
fn sum_signed(
    values: impl IntoIterator<Item = Money>,
    currency: Currency,
) -> Result<Money, AccountError> {
    let values = values.into_iter().collect::<Vec<_>>();
    let scale = values.iter().map(|value| value.scale()).max().unwrap_or(0);
    let mut total = SignedDecimal::zero();
    total.scale = scale;
    for value in values {
        if value.currency() != currency {
            return Err(property_simulation_domain::DomainError::CurrencyMismatch.into());
        }
        let magnitude = value
            .coefficient()
            .unsigned_abs()
            .to_string()
            .bytes()
            .rev()
            .map(|digit| digit - b'0')
            .collect::<Vec<_>>();
        let extra_zeros = usize::from(scale - value.scale());
        let mut aligned = vec![0; extra_zeros];
        aligned.extend(magnitude);
        total.add(value.is_negative(), aligned);
    }

    while total.scale > 0 && total.digits.len() > 1 && total.digits.first() == Some(&0) {
        total.digits.remove(0);
        total.scale -= 1;
    }
    let magnitude = total
        .digits
        .iter()
        .rev()
        .map(|digit| char::from(b'0' + digit))
        .collect::<String>();
    let magnitude = magnitude
        .parse::<u128>()
        .map_err(|_| property_simulation_domain::DomainError::ArithmeticOverflow)?;
    let coefficient = if total.negative {
        if magnitude == (i128::MAX as u128) + 1 {
            i128::MIN
        } else if magnitude <= i128::MAX as u128 {
            -(magnitude as i128)
        } else {
            return Err(property_simulation_domain::DomainError::ArithmeticOverflow.into());
        }
    } else {
        i128::try_from(magnitude)
            .map_err(|_| property_simulation_domain::DomainError::ArithmeticOverflow)?
    };
    Ok(Money::from_parts(coefficient, total.scale, currency)?)
}

/// Small signed base-10 accumulator used only for intermediate statement totals.
/// The public `Money` result remains checked and bounded to its `i128` representation.
struct SignedDecimal {
    negative: bool,
    digits: Vec<u8>,
    scale: u8,
}

impl SignedDecimal {
    fn zero() -> Self {
        Self {
            negative: false,
            digits: vec![0],
            scale: 0,
        }
    }

    fn add(&mut self, negative: bool, mut magnitude: Vec<u8>) {
        trim_high_zeros(&mut magnitude);
        if magnitude == [0] {
            return;
        }
        if self.digits == [0] {
            self.negative = negative;
            self.digits = magnitude;
            return;
        }
        if self.negative == negative {
            add_magnitudes(&mut self.digits, &magnitude);
            return;
        }
        match compare_magnitudes(&self.digits, &magnitude) {
            std::cmp::Ordering::Equal => {
                self.negative = false;
                self.digits = vec![0];
            }
            std::cmp::Ordering::Greater => subtract_magnitude(&mut self.digits, &magnitude),
            std::cmp::Ordering::Less => {
                subtract_magnitude(&mut magnitude, &self.digits);
                self.digits = magnitude;
                self.negative = negative;
            }
        }
    }
}

fn trim_high_zeros(digits: &mut Vec<u8>) {
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
}

fn compare_magnitudes(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
    left.len()
        .cmp(&right.len())
        .then_with(|| left.iter().rev().cmp(right.iter().rev()))
}

fn add_magnitudes(left: &mut Vec<u8>, right: &[u8]) {
    let mut carry = 0;
    for index in 0..left.len().max(right.len()) {
        if index == left.len() {
            left.push(0);
        }
        let sum = left[index] + right.get(index).copied().unwrap_or(0) + carry;
        left[index] = sum % 10;
        carry = sum / 10;
    }
    if carry > 0 {
        left.push(carry);
    }
}

fn subtract_magnitude(left: &mut Vec<u8>, right: &[u8]) {
    let mut borrow = 0_i8;
    for (index, left_digit) in left.iter_mut().enumerate() {
        let difference = i8::try_from(*left_digit).expect("decimal digit fits i8")
            - i8::try_from(right.get(index).copied().unwrap_or(0)).expect("decimal digit fits i8")
            - borrow;
        if difference < 0 {
            *left_digit = u8::try_from(difference + 10).expect("adjusted digit is non-negative");
            borrow = 1;
        } else {
            *left_digit = u8::try_from(difference).expect("difference is a decimal digit");
            borrow = 0;
        }
    }
    debug_assert_eq!(borrow, 0, "left magnitude must be at least right magnitude");
    trim_high_zeros(left);
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
        amount.checked_neg()?
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

    #[test]
    fn signed_aggregation_cancels_before_summing_and_ignores_order() {
        let c = Currency::from_code("CAD").unwrap();
        let maximum = Money::from_parts(i128::MAX, 0, c).unwrap();
        let one = m(1, c);
        let negative_maximum = maximum.checked_neg().unwrap();

        assert_eq!(
            sum_signed([maximum, one, negative_maximum], c).unwrap(),
            one
        );
        assert_eq!(
            sum_signed([negative_maximum, one, maximum], c).unwrap(),
            one
        );
        let maximum_at_scale_one = Money::from_parts(i128::MAX, 1, c).unwrap();
        assert_eq!(
            sum_signed(
                [
                    maximum,
                    maximum_at_scale_one,
                    maximum_at_scale_one.checked_neg().unwrap(),
                    negative_maximum,
                ],
                c
            )
            .unwrap(),
            Money::from_parts(0, 0, c).unwrap()
        );
        assert_eq!(
            sum_signed([Money::from_parts(i128::MIN, 0, c).unwrap(), maximum], c).unwrap(),
            Money::from_parts(-1, 0, c).unwrap()
        );
        assert_eq!(
            CashFlowTotals {
                operating: maximum,
                investing: one,
                financing: negative_maximum,
            }
            .net_change()
            .unwrap(),
            one
        );
    }

    fn account(n: u8, code: u16, category: AccountCategory, name: &str) -> Account {
        let code = AccountCode::new(code).unwrap();
        if category == AccountCategory::Asset && name.to_lowercase().contains("cash") {
            Account::new_cash_position(id(n), code, name).unwrap()
        } else {
            Account::new(id(n), code, category, false, name).unwrap()
        }
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
        let validated = ledger.validate_projection(month, month, &[id(1)]).unwrap();
        assert!(matches!(
            ledger.validate_projection(month, month, &[]),
            Err(AccountError::CashFlowAccountsRequired)
        ));
        let projection = validated.project().unwrap();
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
        assert_eq!(
            ledger.post(LedgerTransaction::new(
                txid(1),
                month,
                [
                    LedgerEntry::new(id(2), BalanceSide::Debit, m(1, c)),
                    LedgerEntry::new(id(1), BalanceSide::Credit, m(1, c)),
                ],
            )),
            Err(AccountError::CashFlowCategoryMissing(id(1)))
        );
        let mut other =
            Ledger::new(ledger.chart().clone(), ledger.opening_balances().clone()).unwrap();
        assert_eq!(
            other.post(LedgerTransaction::new(
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
            )),
            Err(AccountError::CashFlowCategoryOnNonCashAccount(id(2)))
        );
        assert_eq!(
            other.validate_projection(month, month, &[]).map(|_| ()),
            Err(AccountError::CashFlowAccountsRequired)
        );
        let next_month = month.checked_add_months(1).unwrap();
        assert_eq!(
            other
                .validate_projection(next_month, month, &[id(1)])
                .map(|_| ()),
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
    fn l07_accrued_interest_liability_affects_expense_and_balance_sheet_but_not_cash_flow() {
        let c = Currency::from_code("CAD").unwrap();
        let chart = ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset, "Cash"),
            account(2, 2000, AccountCategory::Liability, "Interest payable"),
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

    #[test]
    fn large_amounts_aggregate_across_accounts_and_months_exactly() {
        let c = Currency::from_code("CAD").unwrap();
        let large = Money::parse("100000000000000000000000000000000000", c).unwrap();
        let total = Money::parse("200000000000000000000000000000000000", c).unwrap();
        let chart = ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset, "Cash"),
            account(2, 3000, AccountCategory::Equity, "Opening equity"),
            account(3, 4000, AccountCategory::Income, "Rent A"),
            account(4, 4001, AccountCategory::Income, "Rent B"),
        ])
        .unwrap();
        let opening = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(id(1), BalanceSide::Debit, m(0, c)).unwrap(),
                OpeningBalance::new(id(2), BalanceSide::Credit, m(0, c)).unwrap(),
            ],
            None,
        )
        .unwrap();
        let mut ledger = Ledger::new(chart, opening).unwrap();
        let jan = YearMonth::new(2025, 1).unwrap();
        let feb = jan.checked_add_months(1).unwrap();
        ledger
            .post(LedgerTransaction::new(
                txid(1),
                jan,
                [
                    LedgerEntry::new(id(1), BalanceSide::Debit, large)
                        .with_cash_flow_category(CashFlowCategory::Operating),
                    LedgerEntry::new(id(3), BalanceSide::Credit, large),
                ],
            ))
            .unwrap();
        ledger
            .post(LedgerTransaction::new(
                txid(2),
                feb,
                [
                    LedgerEntry::new(id(1), BalanceSide::Debit, large)
                        .with_cash_flow_category(CashFlowCategory::Operating),
                    LedgerEntry::new(id(4), BalanceSide::Credit, large),
                ],
            ))
            .unwrap();

        assert_eq!(ledger.balance(id(1)).unwrap(), (BalanceSide::Debit, total));
        let projection = ledger
            .validate_projection(jan, feb, &[id(1)])
            .unwrap()
            .project()
            .unwrap();
        assert_eq!(projection.balance_sheet.assets, total);
        assert_eq!(projection.balance_sheet.equity, total);
        assert_eq!(projection.balance_sheet.cumulative_net_income, total);
        assert_eq!(projection.income_statement.income, total);
        assert_eq!(projection.income_statement.net_income, total);
        assert_eq!(projection.cash_flow_statement.totals.operating, total);
    }

    #[test]
    fn projection_nets_large_signed_asset_balances_before_aggregation() {
        let c = Currency::from_code("CAD").unwrap();
        let chart = ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset, "Property one"),
            account(2, 1100, AccountCategory::Asset, "Property two"),
            account(3, 1900, AccountCategory::Asset, "Cash"),
            account(4, 2000, AccountCategory::Liability, "Mortgage"),
            account(5, 3000, AccountCategory::Equity, "Equity"),
            account(6, 5000, AccountCategory::Expense, "Repairs"),
        ])
        .unwrap();
        let opening = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(id(3), BalanceSide::Debit, m(0, c)).unwrap(),
                OpeningBalance::new(id(5), BalanceSide::Credit, m(0, c)).unwrap(),
            ],
            None,
        )
        .unwrap();
        let mut ledger = Ledger::new(chart, opening).unwrap();
        let month = YearMonth::new(2025, 1).unwrap();
        let maximum = Money::from_parts(i128::MAX, 0, c).unwrap();

        ledger
            .post_batch([
                LedgerTransaction::new(
                    txid(1),
                    month,
                    [
                        LedgerEntry::new(id(1), BalanceSide::Debit, maximum),
                        LedgerEntry::new(id(4), BalanceSide::Credit, maximum),
                    ],
                ),
                LedgerTransaction::new(
                    txid(2),
                    month,
                    [
                        LedgerEntry::new(id(2), BalanceSide::Debit, m(1, c)),
                        LedgerEntry::new(id(5), BalanceSide::Credit, m(1, c)),
                    ],
                ),
                LedgerTransaction::new(
                    txid(3),
                    month,
                    [
                        LedgerEntry::new(id(6), BalanceSide::Debit, maximum),
                        LedgerEntry::new(id(3), BalanceSide::Credit, maximum)
                            .with_cash_flow_category(CashFlowCategory::Operating),
                    ],
                ),
            ])
            .unwrap();

        let projection = ledger.project(month, month, &[id(3)]).unwrap();
        assert_eq!(projection.balance_sheet.assets, m(1, c));
        assert_eq!(projection.balance_sheet.liabilities, maximum);
        assert_eq!(
            projection.balance_sheet.equity,
            m(1, c).checked_sub(maximum).unwrap()
        );
        assert_eq!(projection.income_statement.expenses, maximum);
        assert_eq!(
            projection.cash_flow_statement.totals.operating,
            maximum.checked_neg().unwrap()
        );
    }
}
