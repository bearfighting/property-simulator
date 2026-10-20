use property_simulation_accounting::{AccountCategory, AccountError, Ledger, LedgerTransaction};
use property_simulation_domain::RoundingPolicy;
use property_simulation_models::RepairCostDistribution;

use crate::types::{FrozenInput, IdentifiedProposal, OpeningState};

/// A correctable issue found while freezing a period's input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationIssue {
    pub field: &'static str,
    pub message: &'static str,
}

/// All input issues found in a single validation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors(pub Vec<ValidationIssue>);

/// A transaction error found while preflighting an identified proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalValidationIssue {
    pub transaction_key: Option<String>,
    pub error: AccountError,
}

/// All transaction issues found while preflighting a proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalValidationErrors(pub Vec<ProposalValidationIssue>);

/// Inputs that have passed all checks required before sampling and transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedInput {
    opening_state: OpeningState,
    frozen_input: FrozenInput,
    repair_distribution: RepairCostDistribution,
    opening_ledger: Ledger,
}

impl ValidatedInput {
    pub fn opening_state(&self) -> &OpeningState {
        &self.opening_state
    }
    pub const fn frozen_input(&self) -> &FrozenInput {
        &self.frozen_input
    }
    pub const fn repair_distribution(&self) -> &RepairCostDistribution {
        &self.repair_distribution
    }
    pub(crate) fn matches_opening_ledger(&self, ledger: &Ledger) -> bool {
        self.opening_ledger == *ledger
    }
}

/// Validate frozen values and collect every independent issue discoverable at this boundary.
pub fn validate_input(
    input: &FrozenInput,
    repair_distribution: &RepairCostDistribution,
    ledger: &Ledger,
    state: &OpeningState,
) -> Result<ValidatedInput, ValidationErrors> {
    let mut issues = Vec::new();
    if repair_distribution.validate_support().is_err() {
        issues.push(ValidationIssue {
            field: "repair_distribution.maximum",
            message: "repair distribution support must be non-negative",
        });
    }
    if input.rent.is_negative() {
        issues.push(ValidationIssue {
            field: "rent",
            message: "rent must be non-negative",
        });
    }
    if input.repair_cost_limit.is_negative() {
        issues.push(ValidationIssue {
            field: "repair_cost_limit",
            message: "repair cost limit must be non-negative",
        });
    }
    let currency = ledger.currency();
    if input.rent.currency() != currency {
        issues.push(ValidationIssue {
            field: "rent",
            message: "rent and ledger must use the same currency",
        });
    }
    if input.repair_cost_limit.currency() != currency {
        issues.push(ValidationIssue {
            field: "repair_cost_limit",
            message: "repair cost limit and ledger must use the same currency",
        });
    }
    if state.cash.currency() != currency {
        issues.push(ValidationIssue {
            field: "opening_state.cash",
            message: "opening cash and frozen amounts must use the same currency",
        });
    }
    if repair_distribution.maximum.currency() != currency {
        issues.push(ValidationIssue {
            field: "repair_distribution.maximum",
            message: "repair support and frozen amounts must use the same currency",
        });
    }
    if repair_distribution.maximum != input.repair_cost_limit {
        issues.push(ValidationIssue {
            field: "repair_distribution.maximum",
            message: "repair support must match the configured repair cost limit",
        });
    }

    for (field, amount) in [
        ("rent", input.rent),
        ("repair_cost_limit", input.repair_cost_limit),
        ("repair_distribution.maximum", repair_distribution.maximum),
    ] {
        if !amount.is_negative() && amount.rounded_for_posting(RoundingPolicy::HalfUp).is_err() {
            issues.push(ValidationIssue {
                field,
                message: "amount cannot be rounded for posting",
            });
        }
    }

    let cash_account_valid = match ledger.chart().get(input.cash_account) {
        None => {
            issues.push(ValidationIssue {
                field: "cash_account",
                message: "cash account does not exist",
            });
            false
        }
        Some(account)
            if account.category() != AccountCategory::Asset
                || !account.is_cash_position()
                || account.is_contra() =>
        {
            issues.push(ValidationIssue {
                field: "cash_account",
                message: "cash account must be a non-contra cash-position asset",
            });
            false
        }
        Some(_) => true,
    };
    if cash_account_valid && state.cash.currency() == currency {
        match ledger.balance(input.cash_account) {
            Ok((side, balance)) => {
                let signed_balance = if side == property_simulation_accounting::BalanceSide::Debit {
                    Ok(balance)
                } else {
                    balance.checked_neg().map_err(AccountError::from)
                };
                match signed_balance {
                    Ok(balance) if balance == state.cash => {}
                    Ok(_) => issues.push(ValidationIssue {
                        field: "opening_state.cash",
                        message: "opening cash must match the ledger cash-account balance",
                    }),
                    Err(_) => issues.push(ValidationIssue {
                        field: "opening_state.cash",
                        message: "ledger cash-account balance cannot be represented",
                    }),
                }
            }
            Err(_) => issues.push(ValidationIssue {
                field: "opening_state.cash",
                message: "ledger cash-account balance cannot be represented",
            }),
        }
    }
    match ledger.chart().get(input.rent_account) {
        None => issues.push(ValidationIssue {
            field: "rent_account",
            message: "rent account does not exist",
        }),
        Some(account) if account.category() != AccountCategory::Income => {
            issues.push(ValidationIssue {
                field: "rent_account",
                message: "rent account must be an income account",
            });
        }
        Some(_) => {}
    }
    match ledger.chart().get(input.repair_expense_account) {
        None => issues.push(ValidationIssue {
            field: "repair_expense_account",
            message: "repair account does not exist",
        }),
        Some(account) if account.category() != AccountCategory::Expense => {
            issues.push(ValidationIssue {
                field: "repair_expense_account",
                message: "repair account must be an expense account",
            });
        }
        Some(_) => {}
    }

    if issues.is_empty() {
        Ok(ValidatedInput {
            opening_state: state.clone(),
            frozen_input: *input,
            repair_distribution: *repair_distribution,
            opening_ledger: ledger.clone(),
        })
    } else {
        Err(ValidationErrors(issues))
    }
}

/// Check all proposed transactions against a candidate ledger without mutating the caller's
/// ledger. `post_batch` applies the same account, currency, precision, cash classification,
/// balance, duplicate-ID, and month-end asset rules used for a real post.
pub fn validate_proposal_transactions(
    ledger: &Ledger,
    proposal: &IdentifiedProposal,
) -> Result<(), ProposalValidationErrors> {
    let transactions: Vec<LedgerTransaction> = proposal
        .transactions()
        .iter()
        .cloned()
        .map(|transaction| transaction.into_ledger_transaction())
        .collect();

    // Collect transaction-local errors without cloning the ledger per transaction. A valid
    // batch may contain individually invalid intermediate account balances, so the batch result
    // remains authoritative.
    let mut issues = Vec::new();
    for transaction in &transactions {
        if let Err(error) = ledger.validate_transaction(transaction) {
            issues.push(ProposalValidationIssue {
                transaction_key: transaction.source().map(|source| source.output_key.clone()),
                error,
            });
        }
    }

    let mut candidate = ledger.clone();
    match candidate.post_batch(transactions) {
        Ok(()) => Ok(()),
        Err(error) => {
            if !issues.iter().any(|issue| issue.error == error) {
                issues.push(ProposalValidationIssue {
                    transaction_key: None,
                    error,
                });
            }
            Err(ProposalValidationErrors(issues))
        }
    }
}
