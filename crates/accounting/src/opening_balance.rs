use std::collections::{BTreeMap, BTreeSet};

use property_simulation_domain::{AccountId, Currency, Money};

use crate::{AccountCategory, AccountError, BalanceSide, ChartOfAccounts};

/// A supplied opening fact. The amount is non-negative; side carries the sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpeningBalance {
    account_id: AccountId,
    side: BalanceSide,
    amount: Money,
}

impl OpeningBalance {
    pub fn new(
        account_id: AccountId,
        side: BalanceSide,
        amount: Money,
    ) -> Result<Self, AccountError> {
        if amount.amount().is_sign_negative() {
            return Err(AccountError::NegativeOpeningBalance(account_id));
        }
        Ok(Self {
            account_id,
            side,
            amount,
        })
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

/// Validated opening facts and their balance-sheet projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningBalances {
    chart: ChartOfAccounts,
    entries: Vec<OpeningBalance>,
    currency: Currency,
    totals: OpeningTotals,
    normal_sides: BTreeMap<AccountId, BalanceSide>,
}

/// Opening balance-sheet totals, derived from account categories and sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpeningTotals {
    assets: Money,
    liabilities: Money,
    equity: Money,
    net_worth: Money,
}

impl OpeningBalances {
    pub fn new(
        chart: &ChartOfAccounts,
        entries: impl IntoIterator<Item = OpeningBalance>,
        declared_net_worth: Option<Money>,
    ) -> Result<Self, AccountError> {
        let entries: Vec<_> = entries.into_iter().collect();
        let currency = entries
            .first()
            .map(|entry| entry.amount.currency())
            .or_else(|| declared_net_worth.map(Money::currency))
            .ok_or(AccountError::OpeningCurrencyMismatch)?;
        let mut ids = BTreeSet::new();
        let normal_sides = chart
            .accounts_by_code()
            .map(|account| (account.id(), account.normal_balance()))
            .collect();
        let mut debits = Money::new(0.into(), currency);
        let mut credits = Money::new(0.into(), currency);
        let (mut assets, mut liabilities, mut equity) = (debits, debits, debits);

        for entry in &entries {
            if !ids.insert(entry.account_id) {
                return Err(AccountError::DuplicateOpeningAccount(entry.account_id));
            }
            let account = chart
                .get(entry.account_id)
                .ok_or(AccountError::UnknownOpeningAccount(entry.account_id))?;
            if !matches!(
                account.category(),
                AccountCategory::Asset | AccountCategory::Liability | AccountCategory::Equity
            ) {
                return Err(AccountError::OpeningBalanceNotAllowed(account.category()));
            }
            if account.category() == AccountCategory::Asset
                && !account.is_contra()
                && !entry.amount.amount().is_zero()
                && entry.side != account.normal_balance()
            {
                return Err(AccountError::AbnormalAssetOpeningSide {
                    account_id: entry.account_id,
                    side: entry.side,
                });
            }
            if entry.amount.currency() != currency {
                return Err(AccountError::OpeningCurrencyMismatch);
            }
            match entry.side {
                BalanceSide::Debit => debits = debits.checked_add(entry.amount)?,
                BalanceSide::Credit => credits = credits.checked_add(entry.amount)?,
            }
            // Contra assets reduce the asset total when carrying their normal credit balance.
            let positive_side = match account.category() {
                AccountCategory::Asset => BalanceSide::Debit,
                AccountCategory::Liability | AccountCategory::Equity => BalanceSide::Credit,
                AccountCategory::Income | AccountCategory::Expense => unreachable!(),
            };
            let signed = if entry.side == positive_side {
                entry.amount
            } else {
                Money::new(-entry.amount.amount(), currency)
            };
            match account.category() {
                AccountCategory::Asset => assets = assets.checked_add(signed)?,
                AccountCategory::Liability => liabilities = liabilities.checked_add(signed)?,
                AccountCategory::Equity => equity = equity.checked_add(signed)?,
                AccountCategory::Income | AccountCategory::Expense => unreachable!(),
            }
        }
        if debits != credits {
            return Err(AccountError::OpeningBalancesDoNotBalance { debits, credits });
        }
        let net_worth = assets.checked_sub(liabilities)?;
        if let Some(declared) = declared_net_worth {
            if declared.currency() != currency {
                return Err(AccountError::OpeningCurrencyMismatch);
            }
            if declared != net_worth {
                return Err(AccountError::DeclaredNetWorthMismatch {
                    declared,
                    derived: net_worth,
                });
            }
        }
        // Equity is retained as a derived class total; balancing guarantees it equals net worth.
        let totals = OpeningTotals {
            assets,
            liabilities,
            equity,
            net_worth,
        };
        Ok(Self {
            chart: chart.clone(),
            entries,
            currency,
            totals,
            normal_sides,
        })
    }

    pub fn entries(&self) -> &[OpeningBalance] {
        &self.entries
    }
    pub(crate) fn belongs_to(&self, chart: &ChartOfAccounts) -> bool {
        self.chart.len() == chart.len()
            && self.chart.accounts_by_code().all(|opening_account| {
                chart.get(opening_account.id()).is_some_and(|account| {
                    opening_account.category() == account.category()
                        && opening_account.is_contra() == account.is_contra()
                })
            })
    }
    pub const fn currency(&self) -> Currency {
        self.currency
    }
    pub const fn totals(&self) -> OpeningTotals {
        self.totals
    }

    /// Returns an account's opening balance; omitted accounts have a zero normal-side balance.
    pub fn balance(&self, account_id: AccountId) -> Result<(BalanceSide, Money), AccountError> {
        let normal_side = self
            .normal_sides
            .get(&account_id)
            .ok_or(AccountError::UnknownOpeningAccount(account_id))?;
        Ok(self
            .entries
            .iter()
            .find(|entry| entry.account_id == account_id)
            .map(|entry| (entry.side, entry.amount))
            .unwrap_or((*normal_side, Money::new(0.into(), self.currency))))
    }

    pub fn balances_by_account(&self) -> BTreeMap<AccountId, (BalanceSide, Money)> {
        self.entries
            .iter()
            .map(|entry| (entry.account_id, (entry.side, entry.amount)))
            .collect()
    }
}

impl OpeningTotals {
    pub const fn assets(self) -> Money {
        self.assets
    }
    pub const fn liabilities(self) -> Money {
        self.liabilities
    }
    pub const fn equity(self) -> Money {
        self.equity
    }
    pub const fn net_worth(self) -> Money {
        self.net_worth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use property_simulation_domain::Currency;
    use rust_decimal::Decimal;

    fn money(value: i64, currency: Currency) -> Money {
        Money::new(Decimal::from(value), currency)
    }
    fn account(id: u8, code: u16, category: AccountCategory) -> crate::Account {
        let mut bytes = [0; 16];
        bytes[15] = id;
        crate::Account::new(
            AccountId::from_bytes(bytes),
            crate::AccountCode::new(code).unwrap(),
            category,
            false,
            format!("Account {id}"),
        )
        .unwrap()
    }
    fn id(n: u8) -> AccountId {
        let mut bytes = [0; 16];
        bytes[15] = n;
        AccountId::from_bytes(bytes)
    }
    fn entry(id: u8, side: BalanceSide, amount: Money) -> OpeningBalance {
        OpeningBalance::new(self::id(id), side, amount).unwrap()
    }
    fn chart() -> ChartOfAccounts {
        ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset),
            account(2, 1100, AccountCategory::Asset),
            account(3, 2000, AccountCategory::Liability),
            account(4, 3000, AccountCategory::Equity),
            account(5, 4000, AccountCategory::Income),
            account(6, 5000, AccountCategory::Expense),
        ])
        .unwrap()
    }

    fn chart_with_contra_asset() -> ChartOfAccounts {
        ChartOfAccounts::new([
            account(1, 1000, AccountCategory::Asset),
            crate::Account::new(
                id(8),
                crate::AccountCode::new(1100).unwrap(),
                AccountCategory::Asset,
                true,
                "Accumulated depreciation",
            )
            .unwrap(),
            account(4, 3000, AccountCategory::Equity),
        ])
        .unwrap()
    }

    #[test]
    fn derives_l01_opening_totals_without_period_transactions() {
        let currency = Currency::from_code("CAD").unwrap();
        let opening = OpeningBalances::new(
            &chart(),
            [
                entry(1, BalanceSide::Debit, money(300_000, currency)),
                entry(2, BalanceSide::Debit, money(20_000, currency)),
                entry(3, BalanceSide::Credit, money(200_000, currency)),
                entry(4, BalanceSide::Credit, money(120_000, currency)),
            ],
            Some(money(120_000, currency)),
        )
        .unwrap();
        assert_eq!(opening.totals().assets(), money(320_000, currency));
        assert_eq!(opening.totals().liabilities(), money(200_000, currency));
        assert_eq!(opening.totals().equity(), money(120_000, currency));
        assert_eq!(opening.totals().net_worth(), money(120_000, currency));
        assert_eq!(opening.entries().len(), 4);
        assert_eq!(
            opening.balance(id(5)),
            Ok((BalanceSide::Credit, money(0, currency)))
        );
        assert_eq!(
            opening.balance(id(2)).unwrap(),
            (BalanceSide::Debit, money(20_000, currency))
        );
        assert_eq!(
            opening.balance(id(7)),
            Err(AccountError::UnknownOpeningAccount(id(7)))
        );
    }

    #[test]
    fn debit_equity_represents_negative_net_worth_when_balanced() {
        let c = Currency::from_code("CAD").unwrap();
        let opening = OpeningBalances::new(
            &chart(),
            [
                entry(1, BalanceSide::Debit, money(80, c)),
                entry(3, BalanceSide::Credit, money(100, c)),
                entry(4, BalanceSide::Debit, money(20, c)),
            ],
            None,
        )
        .unwrap();
        assert_eq!(opening.totals().net_worth(), money(-20, c));
        assert_eq!(opening.totals().equity(), money(-20, c));
        assert_eq!(
            opening.balance(id(2)).unwrap(),
            (BalanceSide::Debit, money(0, c))
        );
    }

    #[test]
    fn rejects_credit_balance_on_normal_asset_and_nets_contra_asset() {
        let c = Currency::from_code("CAD").unwrap();
        let chart = chart();
        assert!(matches!(
            OpeningBalances::new(
                &chart,
                [
                    entry(1, BalanceSide::Credit, money(20, c)),
                    entry(4, BalanceSide::Credit, money(20, c))
                ],
                None
            ),
            Err(AccountError::AbnormalAssetOpeningSide {
                account_id,
                side: BalanceSide::Credit
            }) if account_id == id(1)
        ));

        let opening = OpeningBalances::new(
            &chart_with_contra_asset(),
            [
                entry(1, BalanceSide::Debit, money(100, c)),
                entry(8, BalanceSide::Credit, money(20, c)),
                entry(4, BalanceSide::Credit, money(80, c)),
            ],
            Some(money(80, c)),
        )
        .unwrap();
        assert_eq!(opening.totals().assets(), money(80, c));
        assert_eq!(opening.totals().equity(), money(80, c));

        let zero = OpeningBalances::new(&chart, [entry(1, BalanceSide::Credit, money(0, c))], None)
            .unwrap();
        assert_eq!(zero.entries().len(), 1);
        assert_eq!(
            zero.balance(id(1)).unwrap(),
            (BalanceSide::Credit, money(0, c))
        );
        assert_eq!(zero.totals().assets(), money(0, c));
    }

    #[test]
    fn rejects_structural_opening_errors() {
        let c = Currency::from_code("CAD").unwrap();
        let usd = Currency::from_code("USD").unwrap();
        let chart = chart();
        assert!(matches!(
            OpeningBalance::new(id(1), BalanceSide::Debit, money(-1, c)),
            Err(AccountError::NegativeOpeningBalance(_))
        ));
        assert!(matches!(
            OpeningBalances::new(&chart, [entry(1, BalanceSide::Debit, money(1, c))], None),
            Err(AccountError::OpeningBalancesDoNotBalance { .. })
        ));
        assert!(matches!(
            OpeningBalances::new(
                &chart,
                [
                    entry(1, BalanceSide::Debit, money(1, c)),
                    entry(1, BalanceSide::Debit, money(0, c))
                ],
                None
            ),
            Err(AccountError::DuplicateOpeningAccount(_))
        ));
        assert!(matches!(
            OpeningBalances::new(&chart, [entry(9, BalanceSide::Debit, money(0, c))], None),
            Err(AccountError::UnknownOpeningAccount(_))
        ));
        for n in [5, 6] {
            assert!(matches!(
                OpeningBalances::new(&chart, [entry(n, BalanceSide::Debit, money(0, c))], None),
                Err(AccountError::OpeningBalanceNotAllowed(_))
            ));
        }
        assert!(matches!(
            OpeningBalances::new(
                &chart,
                [
                    entry(1, BalanceSide::Debit, money(1, c)),
                    entry(3, BalanceSide::Credit, money(1, usd))
                ],
                None
            ),
            Err(AccountError::OpeningCurrencyMismatch)
        ));
        assert!(matches!(
            OpeningBalances::new(
                &chart,
                [
                    entry(1, BalanceSide::Debit, money(80, c)),
                    entry(3, BalanceSide::Credit, money(100, c)),
                    entry(4, BalanceSide::Debit, money(20, c))
                ],
                Some(money(-19, c))
            ),
            Err(AccountError::DeclaredNetWorthMismatch { .. })
        ));
    }
}
