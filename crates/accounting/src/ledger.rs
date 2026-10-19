use std::collections::BTreeMap;

use property_simulation_domain::{
    AccountId, Currency, LedgerTransactionId, Money, RoundingPolicy, YearMonth,
};

use crate::{AccountError, BalanceSide, ChartOfAccounts, OpeningBalances};

/// One debit or credit in a proposed ledger transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerEntry {
    account_id: AccountId,
    side: BalanceSide,
    amount: Money,
}

impl LedgerEntry {
    pub const fn new(account_id: AccountId, side: BalanceSide, amount: Money) -> Self {
        Self {
            account_id,
            side,
            amount,
        }
    }
    pub const fn account_id(self) -> AccountId {
        self.account_id
    }
    pub const fn side(self) -> BalanceSide {
        self.side
    }
    pub const fn amount(self) -> Money {
        self.amount
    }
}

/// A caller-identified transaction proposed for a specific inclusive month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerTransaction {
    id: LedgerTransactionId,
    period: YearMonth,
    entries: Vec<LedgerEntry>,
}

impl LedgerTransaction {
    pub fn new(
        id: LedgerTransactionId,
        period: YearMonth,
        entries: impl IntoIterator<Item = LedgerEntry>,
    ) -> Self {
        Self {
            id,
            period,
            entries: entries.into_iter().collect(),
        }
    }
    pub const fn id(&self) -> LedgerTransactionId {
        self.id
    }
    pub const fn period(&self) -> YearMonth {
        self.period
    }
    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PostedTransaction(LedgerTransaction);

/// A single-currency ledger. Posted entries are authoritative; balances are recalculated on read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ledger {
    chart: ChartOfAccounts,
    opening: OpeningBalances,
    currency: Currency,
    posted: Vec<PostedTransaction>,
}

impl Ledger {
    pub fn new(chart: ChartOfAccounts, opening: OpeningBalances) -> Result<Self, AccountError> {
        if !opening.belongs_to(&chart) {
            return Err(AccountError::OpeningChartMismatch);
        }
        let currency = opening.currency();
        Ok(Self {
            chart,
            opening,
            currency,
            posted: Vec::new(),
        })
    }

    pub const fn currency(&self) -> Currency {
        self.currency
    }
    pub fn chart(&self) -> &ChartOfAccounts {
        &self.chart
    }
    pub fn opening_balances(&self) -> &OpeningBalances {
        &self.opening
    }
    pub fn transactions(&self) -> impl Iterator<Item = &LedgerTransaction> {
        self.posted.iter().map(|posted| &posted.0)
    }

    /// Validates the complete transaction before mutating the posted ledger.
    pub fn post(&mut self, transaction: LedgerTransaction) -> Result<(), AccountError> {
        if self
            .posted
            .iter()
            .any(|posted| posted.0.id == transaction.id)
        {
            return Err(AccountError::DuplicateTransactionId(transaction.id));
        }
        if transaction.entries.is_empty() {
            return Err(AccountError::EmptyTransaction);
        }
        let mut debits = Money::new(0.into(), self.currency);
        let mut credits = Money::new(0.into(), self.currency);
        let mut has_debit = false;
        let mut has_credit = false;
        for entry in &transaction.entries {
            if self.chart.get(entry.account_id).is_none() {
                return Err(AccountError::UnknownPostingAccount(entry.account_id));
            }
            if entry.amount.currency() != self.currency {
                return Err(AccountError::TransactionCurrencyMismatch);
            }
            if entry.amount.amount().is_sign_negative() {
                return Err(AccountError::NegativePostingAmount);
            }
            if entry.amount.rounded_for_posting(RoundingPolicy::HalfUp)? != entry.amount {
                return Err(AccountError::PostingAmountPrecision(entry.amount));
            }
            match entry.side {
                BalanceSide::Debit => {
                    has_debit = true;
                    debits = debits.checked_add(entry.amount)?;
                }
                BalanceSide::Credit => {
                    has_credit = true;
                    credits = credits.checked_add(entry.amount)?;
                }
            }
        }
        if !has_debit || !has_credit {
            return Err(AccountError::MissingDebitOrCredit);
        }
        if debits != credits {
            return Err(AccountError::TransactionDoesNotBalance { debits, credits });
        }
        // Ensure the committed ledger remains computable before mutating it.
        for account_id in transaction
            .entries
            .iter()
            .map(|entry| entry.account_id)
            .collect::<std::collections::BTreeSet<_>>()
        {
            let mut signed = self.signed_balance(account_id)?;
            for entry in transaction
                .entries
                .iter()
                .filter(|entry| entry.account_id == account_id)
            {
                signed = match entry.side {
                    BalanceSide::Debit => signed.checked_add(entry.amount),
                    BalanceSide::Credit => signed.checked_sub(entry.amount),
                }
                .map_err(AccountError::Arithmetic)?;
            }
        }
        self.posted.push(PostedTransaction(transaction));
        Ok(())
    }

    /// Returns the net balance side and non-negative magnitude for any known account.
    pub fn balance(&self, account_id: AccountId) -> Result<(BalanceSide, Money), AccountError> {
        let account = self
            .chart
            .get(account_id)
            .ok_or(AccountError::UnknownPostingAccount(account_id))?;
        let signed = self.signed_balance(account_id)?;
        let side = if signed.amount().is_sign_negative() {
            BalanceSide::Credit
        } else {
            BalanceSide::Debit
        };
        // A zero balance is represented using the account's normal side.
        let side = if signed.amount().is_zero() {
            account.normal_balance()
        } else {
            side
        };
        Ok((side, Money::new(signed.amount().abs(), self.currency)))
    }

    fn signed_balance(&self, account_id: AccountId) -> Result<Money, AccountError> {
        self.chart
            .get(account_id)
            .ok_or(AccountError::UnknownPostingAccount(account_id))?;
        let (opening_side, opening_amount) = self.opening.balance(account_id)?;
        let mut signed = if opening_side == BalanceSide::Debit {
            opening_amount
        } else {
            Money::new(-opening_amount.amount(), self.currency)
        };
        for transaction in &self.posted {
            for entry in transaction
                .0
                .entries
                .iter()
                .filter(|entry| entry.account_id == account_id)
            {
                signed = match entry.side {
                    BalanceSide::Debit => signed.checked_add(entry.amount),
                    BalanceSide::Credit => signed.checked_sub(entry.amount),
                }
                .map_err(AccountError::Arithmetic)?;
            }
        }
        Ok(signed)
    }

    pub fn balances(&self) -> Result<BTreeMap<AccountId, (BalanceSide, Money)>, AccountError> {
        self.chart
            .accounts_by_code()
            .map(|account| Ok((account.id(), self.balance(account.id())?)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Account, AccountCategory, AccountCode, OpeningBalance};
    use property_simulation_domain::Currency;
    use rust_decimal::Decimal;

    fn id(n: u8) -> AccountId {
        let mut b = [0; 16];
        b[15] = n;
        AccountId::from_bytes(b)
    }
    fn txid(n: u8) -> LedgerTransactionId {
        let mut b = [0; 16];
        b[15] = n;
        LedgerTransactionId::from_bytes(b)
    }
    fn money(value: i64, currency: Currency) -> Money {
        Money::new(Decimal::from(value), currency)
    }
    fn fixture() -> (Ledger, Currency) {
        let c = Currency::from_code("CAD").unwrap();
        let chart = ChartOfAccounts::new([
            Account::new(
                id(1),
                AccountCode::new(1000).unwrap(),
                AccountCategory::Asset,
                false,
                "Cash",
            )
            .unwrap(),
            Account::new(
                id(2),
                AccountCode::new(2000).unwrap(),
                AccountCategory::Liability,
                false,
                "Mortgage",
            )
            .unwrap(),
            Account::new(
                id(3),
                AccountCode::new(5000).unwrap(),
                AccountCategory::Expense,
                false,
                "Interest",
            )
            .unwrap(),
            Account::new(
                id(5),
                AccountCode::new(3000).unwrap(),
                AccountCategory::Equity,
                false,
                "Opening equity",
            )
            .unwrap(),
        ])
        .unwrap();
        let opening = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(id(1), BalanceSide::Debit, money(1000, c)).unwrap(),
                OpeningBalance::new(id(2), BalanceSide::Credit, money(800, c)).unwrap(),
                OpeningBalance::new(id(5), BalanceSide::Credit, money(200, c)).unwrap(),
            ],
            None,
        )
        .unwrap();
        (Ledger::new(chart, opening).unwrap(), c)
    }
    fn transaction(n: u8, entries: Vec<LedgerEntry>) -> LedgerTransaction {
        LedgerTransaction::new(txid(n), YearMonth::new(2025, 1).unwrap(), entries)
    }
    fn entry(account: u8, side: BalanceSide, amount: Money) -> LedgerEntry {
        LedgerEntry::new(id(account), side, amount)
    }

    #[test]
    fn posts_balanced_mortgage_payment_and_recomputes_opening_plus_entries() {
        let (mut ledger, c) = fixture();
        ledger
            .post(transaction(
                1,
                vec![
                    entry(2, BalanceSide::Debit, money(100, c)),
                    entry(3, BalanceSide::Debit, money(50, c)),
                    entry(1, BalanceSide::Credit, money(150, c)),
                ],
            ))
            .unwrap();
        assert_eq!(
            ledger.balance(id(1)).unwrap(),
            (BalanceSide::Debit, money(850, c))
        );
        assert_eq!(
            ledger.balance(id(2)).unwrap(),
            (BalanceSide::Credit, money(700, c))
        );
        assert_eq!(
            ledger.balance(id(3)).unwrap(),
            (BalanceSide::Debit, money(50, c))
        );
        assert_eq!(
            ledger.balance(id(5)).unwrap(),
            (BalanceSide::Credit, money(200, c))
        );
        assert_eq!(ledger.transactions().count(), 1);
        assert_eq!(
            ledger.post(transaction(
                1,
                vec![
                    entry(2, BalanceSide::Debit, money(1, c)),
                    entry(1, BalanceSide::Credit, money(1, c))
                ]
            )),
            Err(AccountError::DuplicateTransactionId(txid(1)))
        );
        ledger
            .post(transaction(
                2,
                vec![
                    entry(3, BalanceSide::Debit, money(10, c)),
                    entry(1, BalanceSide::Credit, money(10, c)),
                ],
            ))
            .unwrap();
        assert_eq!(ledger.transactions().count(), 2);
        assert_eq!(ledger.balance(id(1)).unwrap().1, money(840, c));
        assert_eq!(ledger.balance(id(3)).unwrap().1, money(60, c));
        assert_eq!(ledger.balance(id(2)).unwrap().1, money(700, c));
    }

    #[test]
    fn rejects_invalid_transactions_without_mutating_ledger() {
        let (mut l, c) = fixture();
        let usd = Currency::from_code("USD").unwrap();
        let period = YearMonth::new(2025, 1).unwrap();
        let bad = [
            (transaction(1, vec![]), AccountError::EmptyTransaction),
            (
                transaction(1, vec![entry(1, BalanceSide::Debit, money(1, c))]),
                AccountError::MissingDebitOrCredit,
            ),
            (
                transaction(
                    1,
                    vec![
                        entry(1, BalanceSide::Debit, money(1, c)),
                        entry(2, BalanceSide::Credit, money(2, c)),
                    ],
                ),
                AccountError::TransactionDoesNotBalance {
                    debits: money(1, c),
                    credits: money(2, c),
                },
            ),
            (
                transaction(
                    1,
                    vec![
                        entry(1, BalanceSide::Debit, money(-1, c)),
                        entry(2, BalanceSide::Credit, money(-1, c)),
                    ],
                ),
                AccountError::NegativePostingAmount,
            ),
            (
                transaction(
                    1,
                    vec![
                        LedgerEntry::new(id(1), BalanceSide::Debit, money(1, usd)),
                        LedgerEntry::new(id(2), BalanceSide::Credit, money(1, usd)),
                    ],
                ),
                AccountError::TransactionCurrencyMismatch,
            ),
            (
                transaction(
                    1,
                    vec![
                        entry(1, BalanceSide::Debit, Money::new(Decimal::new(1, 3), c)),
                        entry(2, BalanceSide::Credit, Money::new(Decimal::new(1, 3), c)),
                    ],
                ),
                AccountError::PostingAmountPrecision(Money::new(Decimal::new(1, 3), c)),
            ),
            (
                LedgerTransaction::new(
                    txid(1),
                    period,
                    vec![
                        LedgerEntry::new(id(99), BalanceSide::Debit, money(1, c)),
                        entry(2, BalanceSide::Credit, money(1, c)),
                    ],
                ),
                AccountError::UnknownPostingAccount(id(99)),
            ),
        ];
        for (tx, expected) in bad {
            assert_eq!(l.post(tx), Err(expected));
            assert!(l.transactions().next().is_none());
            assert_eq!(l.balance(id(1)).unwrap().1, money(1000, c));
        }
    }

    #[test]
    fn chart_compatibility_ignores_labels_and_codes_but_checks_account_semantics() {
        let (ledger, _) = fixture();
        let compatible = ChartOfAccounts::new([
            Account::new(
                id(1),
                AccountCode::new(1001).unwrap(),
                AccountCategory::Asset,
                false,
                "Renamed cash",
            )
            .unwrap(),
            ledger.chart().get(id(2)).unwrap().clone(),
            ledger.chart().get(id(3)).unwrap().clone(),
            ledger.chart().get(id(5)).unwrap().clone(),
        ])
        .unwrap();
        assert!(Ledger::new(compatible, ledger.opening_balances().clone()).is_ok());

        let incompatible = ChartOfAccounts::new([
            Account::new(
                id(1),
                AccountCode::new(2001).unwrap(),
                AccountCategory::Liability,
                false,
                "Cash classified as liability",
            )
            .unwrap(),
            ledger.chart().get(id(2)).unwrap().clone(),
            ledger.chart().get(id(3)).unwrap().clone(),
            ledger.chart().get(id(5)).unwrap().clone(),
        ])
        .unwrap();
        assert_eq!(
            Ledger::new(incompatible, ledger.opening_balances().clone()),
            Err(AccountError::OpeningChartMismatch)
        );
    }

    #[test]
    fn rejects_posting_that_would_overflow_a_recomputed_balance() {
        let c = Currency::from_code("CAD").unwrap();
        let chart = ChartOfAccounts::new([
            Account::new(
                id(1),
                AccountCode::new(1000).unwrap(),
                AccountCategory::Asset,
                false,
                "Cash",
            )
            .unwrap(),
            Account::new(
                id(2),
                AccountCode::new(2000).unwrap(),
                AccountCategory::Liability,
                false,
                "Mortgage",
            )
            .unwrap(),
        ])
        .unwrap();
        let maximum = Money::new(Decimal::MAX, c);
        let opening = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(id(1), BalanceSide::Debit, maximum).unwrap(),
                OpeningBalance::new(id(2), BalanceSide::Credit, maximum).unwrap(),
            ],
            None,
        )
        .unwrap();
        let mut ledger = Ledger::new(chart, opening).unwrap();
        assert_eq!(
            ledger.post(transaction(
                1,
                vec![
                    entry(1, BalanceSide::Debit, money(1, c)),
                    entry(2, BalanceSide::Credit, money(1, c)),
                ],
            )),
            Err(AccountError::Arithmetic(
                property_simulation_domain::DomainError::ArithmeticOverflow
            ))
        );
        assert_eq!(ledger.transactions().count(), 0);
        assert_eq!(ledger.balance(id(1)).unwrap().1, maximum);
    }
}
