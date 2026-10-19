use std::fmt;

use property_simulation_domain::{AccountId, DomainError, Money};

use crate::{AccountCategory, AccountCode};

/// Account validation failures and explicitly bounded numeric calculation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountError {
    InvalidCode(u16),
    CodeCategoryMismatch {
        code: AccountCode,
        category: AccountCategory,
    },
    ContraNotAllowed(AccountCategory),
    EmptyName,
    DuplicateId(AccountId),
    DuplicateCode(AccountCode),
    UnknownOpeningAccount(AccountId),
    DuplicateOpeningAccount(AccountId),
    OpeningBalanceNotAllowed(AccountCategory),
    NegativeOpeningBalance(AccountId),
    OpeningBalancePrecision(Money),
    AbnormalAssetOpeningSide {
        account_id: AccountId,
        side: crate::BalanceSide,
    },
    OpeningCurrencyMismatch,
    OpeningBalancesDoNotBalance {
        debits: Money,
        credits: Money,
    },
    DeclaredNetWorthMismatch {
        declared: Money,
        derived: Money,
    },
    DuplicateTransactionId(property_simulation_domain::LedgerTransactionId),
    UnknownPostingAccount(AccountId),
    TransactionCurrencyMismatch,
    EmptyTransaction,
    MissingDebitOrCredit,
    NegativePostingAmount,
    TransactionDoesNotBalance {
        debits: Money,
        credits: Money,
    },
    Numeric(DomainError),
    OpeningChartMismatch,
    PostingAmountPrecision(Money),
    InvalidProjectionRange,
    CashFlowAccountMustBeAsset(AccountId),
    CashFlowCategoryMissing(AccountId),
    CashFlowCategoryOnNonCashAccount(AccountId),
    CashFlowAccountsRequired,
    CashFlowAccountsMismatch,
    DuplicateCashFlowAccount(AccountId),
    NegativeAssetBalance(AccountId),
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCode(code) => write!(f, "account code {code} is outside 1000–5999"),
            Self::CodeCategoryMismatch { code, category } => {
                write!(f, "account code {code} does not belong to {category:?}")
            }
            Self::ContraNotAllowed(category) => {
                write!(f, "contra {category:?} accounts are not allowed")
            }
            Self::EmptyName => f.write_str("account name must not be empty"),
            Self::DuplicateId(id) => write!(f, "duplicate account ID {id:?}"),
            Self::DuplicateCode(code) => write!(f, "duplicate account code {code}"),
            Self::UnknownOpeningAccount(id) => write!(f, "unknown opening-balance account {id:?}"),
            Self::DuplicateOpeningAccount(id) => {
                write!(f, "duplicate opening balance for account {id:?}")
            }
            Self::OpeningBalanceNotAllowed(category) => write!(
                f,
                "opening balances are not allowed for {category:?} accounts"
            ),
            Self::NegativeOpeningBalance(id) => {
                write!(f, "opening balance for {id:?} cannot be negative")
            }
            Self::OpeningBalancePrecision(amount) => write!(
                f,
                "opening balance {amount} exceeds its currency minor-unit precision"
            ),
            Self::AbnormalAssetOpeningSide { account_id, side } => write!(
                f,
                "asset account {account_id:?} cannot have an opening {side:?} balance"
            ),
            Self::OpeningCurrencyMismatch => f.write_str("opening balances must use one currency"),
            Self::OpeningBalancesDoNotBalance { debits, credits } => {
                write!(f, "opening debits {debits} do not equal credits {credits}")
            }
            Self::DeclaredNetWorthMismatch { declared, derived } => write!(
                f,
                "declared net worth {declared} does not equal derived net worth {derived}"
            ),
            Self::DuplicateTransactionId(id) => write!(f, "duplicate ledger transaction ID {id:?}"),
            Self::UnknownPostingAccount(id) => write!(f, "unknown posting account {id:?}"),
            Self::TransactionCurrencyMismatch => {
                f.write_str("transaction currency differs from ledger currency")
            }
            Self::EmptyTransaction => f.write_str("ledger transaction must contain entries"),
            Self::MissingDebitOrCredit => {
                f.write_str("ledger transaction must contain at least one debit and one credit")
            }
            Self::NegativePostingAmount => f.write_str("posting amount cannot be negative"),
            Self::TransactionDoesNotBalance { debits, credits } => write!(
                f,
                "transaction debits {debits} do not equal credits {credits}"
            ),
            Self::Numeric(error) => write!(f, "accounting numeric calculation failed: {error}"),
            Self::OpeningChartMismatch => {
                f.write_str("opening balances were validated against a different chart of accounts")
            }
            Self::PostingAmountPrecision(amount) => write!(
                f,
                "posting amount {amount} exceeds its currency minor-unit precision"
            ),
            Self::InvalidProjectionRange => {
                f.write_str("projection start month must not be after end month")
            }
            Self::CashFlowAccountMustBeAsset(id) => {
                write!(
                    f,
                    "cash-flow account {id:?} must be marked as a non-contra cash-position asset"
                )
            }
            Self::CashFlowCategoryMissing(id) => {
                write!(f, "cash posting to {id:?} requires a cash-flow category")
            }
            Self::CashFlowCategoryOnNonCashAccount(id) => write!(
                f,
                "non-cash posting to {id:?} cannot have a cash-flow category"
            ),
            Self::CashFlowAccountsRequired => {
                f.write_str("at least one cash account must be supplied for cash-flow projection")
            }
            Self::CashFlowAccountsMismatch => f.write_str(
                "projection cash accounts must match the ledger's configured cash accounts",
            ),
            Self::DuplicateCashFlowAccount(id) => {
                write!(f, "cash account {id:?} was supplied more than once")
            }
            Self::NegativeAssetBalance(id) => {
                write!(
                    f,
                    "non-contra asset account {id:?} cannot have a credit balance"
                )
            }
        }
    }
}

impl std::error::Error for AccountError {}

impl From<DomainError> for AccountError {
    fn from(error: DomainError) -> Self {
        Self::Numeric(error)
    }
}
