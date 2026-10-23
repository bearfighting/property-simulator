use property_simulation_account::{AccountCode, DebitCredit};
use property_simulation_money::Money;
use property_simulation_month::Month;

/// One account's net balance split into debit and credit sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountBalanceLine {
    account: AccountCode,
    debit: Money,
    credit: Money,
}

impl AccountBalanceLine {
    pub(crate) const fn new(account: AccountCode, debit: Money, credit: Money) -> Self {
        Self {
            account,
            debit,
            credit,
        }
    }

    pub const fn account(&self) -> AccountCode {
        self.account
    }

    pub const fn debit(&self) -> Money {
        self.debit
    }

    pub const fn credit(&self) -> Money {
        self.credit
    }
}

/// All account balances through and including one simulation month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialBalance {
    as_of: Month,
    accounts: Vec<AccountBalanceLine>,
    debit_total: Money,
    credit_total: Money,
}

impl TrialBalance {
    pub(crate) const fn new(
        as_of: Month,
        accounts: Vec<AccountBalanceLine>,
        debit_total: Money,
        credit_total: Money,
    ) -> Self {
        Self {
            as_of,
            accounts,
            debit_total,
            credit_total,
        }
    }

    pub const fn as_of(&self) -> Month {
        self.as_of
    }

    pub fn accounts(&self) -> &[AccountBalanceLine] {
        &self.accounts
    }

    pub const fn debit_total(&self) -> Money {
        self.debit_total
    }

    pub const fn credit_total(&self) -> Money {
        self.credit_total
    }
}

/// One account's natural balance shown on a balance sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalanceSheetLine {
    account: AccountCode,
    amount: Money,
}

impl BalanceSheetLine {
    pub(crate) const fn new(account: AccountCode, amount: Money) -> Self {
        Self { account, amount }
    }

    pub const fn account(&self) -> AccountCode {
        self.account
    }

    /// Positive means a normal balance; negative means a contra balance.
    pub const fn amount(&self) -> Money {
        self.amount
    }
}

/// Financial position through and including one simulation month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceSheet {
    as_of: Month,
    assets: Vec<BalanceSheetLine>,
    liabilities: Vec<BalanceSheetLine>,
    equity_accounts: Vec<BalanceSheetLine>,
    total_assets: Money,
    total_liabilities: Money,
    equity_account_total: Money,
    net_income: Money,
    total_equity: Money,
}

pub(crate) struct BalanceSheetTotals {
    pub(crate) assets: Money,
    pub(crate) liabilities: Money,
    pub(crate) equity_accounts: Money,
    pub(crate) net_income: Money,
    pub(crate) equity: Money,
}

impl BalanceSheet {
    pub(crate) const fn new(
        as_of: Month,
        assets: Vec<BalanceSheetLine>,
        liabilities: Vec<BalanceSheetLine>,
        equity_accounts: Vec<BalanceSheetLine>,
        totals: BalanceSheetTotals,
    ) -> Self {
        Self {
            as_of,
            assets,
            liabilities,
            equity_accounts,
            total_assets: totals.assets,
            total_liabilities: totals.liabilities,
            equity_account_total: totals.equity_accounts,
            net_income: totals.net_income,
            total_equity: totals.equity,
        }
    }

    pub const fn as_of(&self) -> Month {
        self.as_of
    }

    pub fn assets(&self) -> &[BalanceSheetLine] {
        &self.assets
    }

    pub fn liabilities(&self) -> &[BalanceSheetLine] {
        &self.liabilities
    }

    pub fn equity_accounts(&self) -> &[BalanceSheetLine] {
        &self.equity_accounts
    }

    pub const fn total_assets(&self) -> Money {
        self.total_assets
    }

    pub const fn total_liabilities(&self) -> Money {
        self.total_liabilities
    }

    pub const fn equity_account_total(&self) -> Money {
        self.equity_account_total
    }

    /// Cumulative income less expenses, presented as part of equity.
    pub const fn net_income(&self) -> Money {
        self.net_income
    }

    pub const fn total_equity(&self) -> Money {
        self.total_equity
    }
}

/// One account's net income or expense for a selected inclusive period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncomeStatementLine {
    account: AccountCode,
    normal_balance: DebitCredit,
    amount: Money,
}

impl IncomeStatementLine {
    pub(crate) const fn new(
        account: AccountCode,
        normal_balance: DebitCredit,
        amount: Money,
    ) -> Self {
        Self {
            account,
            normal_balance,
            amount,
        }
    }

    pub const fn account(&self) -> AccountCode {
        self.account
    }

    pub const fn normal_balance(&self) -> DebitCredit {
        self.normal_balance
    }

    /// Positive means the account has its normal balance; negative is a contra balance.
    pub const fn amount(&self) -> Money {
        self.amount
    }
}

/// Income and expense activity for an inclusive month interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeStatement {
    start: Month,
    end: Month,
    income: Vec<IncomeStatementLine>,
    expenses: Vec<IncomeStatementLine>,
    income_total: Money,
    expense_total: Money,
    net_income: Money,
}

impl IncomeStatement {
    pub(crate) const fn new(
        start: Month,
        end: Month,
        income: Vec<IncomeStatementLine>,
        expenses: Vec<IncomeStatementLine>,
        income_total: Money,
        expense_total: Money,
        net_income: Money,
    ) -> Self {
        Self {
            start,
            end,
            income,
            expenses,
            income_total,
            expense_total,
            net_income,
        }
    }

    pub const fn start(&self) -> Month {
        self.start
    }

    pub const fn end(&self) -> Month {
        self.end
    }

    pub fn income(&self) -> &[IncomeStatementLine] {
        &self.income
    }

    pub fn expenses(&self) -> &[IncomeStatementLine] {
        &self.expenses
    }

    pub const fn income_total(&self) -> Money {
        self.income_total
    }

    pub const fn expense_total(&self) -> Money {
        self.expense_total
    }

    pub const fn net_income(&self) -> Money {
        self.net_income
    }
}

/// Raw cash-account changes, without operating/investing/financing classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashFlowMovement {
    journal_position: usize,
    month: Month,
    account: AccountCode,
    amount: Money,
}

impl CashFlowMovement {
    pub(crate) const fn new(
        journal_position: usize,
        month: Month,
        account: AccountCode,
        amount: Money,
    ) -> Self {
        Self {
            journal_position,
            month,
            account,
            amount,
        }
    }

    /// Zero-based position in the supplied journal slice; it is not a persistent ID.
    pub const fn journal_position(&self) -> usize {
        self.journal_position
    }

    pub const fn month(&self) -> Month {
        self.month
    }

    pub const fn account(&self) -> AccountCode {
        self.account
    }

    /// Debit cash is positive; credit cash is negative.
    pub const fn amount(&self) -> Money {
        self.amount
    }
}

/// Cash-account line movements in an inclusive month interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashFlowReport {
    start: Month,
    end: Month,
    movements: Vec<CashFlowMovement>,
    total: Money,
}

impl CashFlowReport {
    pub(crate) const fn new(
        start: Month,
        end: Month,
        movements: Vec<CashFlowMovement>,
        total: Money,
    ) -> Self {
        Self {
            start,
            end,
            movements,
            total,
        }
    }

    pub const fn start(&self) -> Month {
        self.start
    }

    pub const fn end(&self) -> Month {
        self.end
    }

    pub fn movements(&self) -> &[CashFlowMovement] {
        &self.movements
    }

    pub const fn total(&self) -> Money {
        self.total
    }
}

/// Successful validation counts for a supplied set of ledger facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditReport {
    account_count: usize,
    opening_balance_count: usize,
    journal_entry_count: usize,
    journal_line_count: usize,
}

impl AuditReport {
    pub(crate) const fn new(
        account_count: usize,
        opening_balance_count: usize,
        journal_entry_count: usize,
        journal_line_count: usize,
    ) -> Self {
        Self {
            account_count,
            opening_balance_count,
            journal_entry_count,
            journal_line_count,
        }
    }

    pub const fn account_count(&self) -> usize {
        self.account_count
    }

    pub const fn opening_balance_count(&self) -> usize {
        self.opening_balance_count
    }

    pub const fn journal_entry_count(&self) -> usize {
        self.journal_entry_count
    }

    pub const fn journal_line_count(&self) -> usize {
        self.journal_line_count
    }
}
