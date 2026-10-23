use property_simulation_accounting::{AccountCategory, AccountError};
#[cfg(test)]
use property_simulation_accounting::{Ledger, LedgerTransaction};

use crate::SimulationError;
#[cfg(test)]
use crate::types::IdentifiedProposal;
use crate::types::{ActivityConfiguration, ActivityInstance, FrozenInput, OpeningState};

/// A correctable issue found while freezing a period's input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationIssue {
    pub field: &'static str,
    pub message: &'static str,
}

/// All input issues found in a single validation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors(pub Vec<ValidationIssue>);

/// Account properties needed by simulation input checks. The application boundary
/// supplies these facts from its accounting implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountFacts {
    pub category: AccountCategory,
    pub cash_position: bool,
    pub contra: bool,
}

/// Narrow accounting read interface used by simulation validation.
/// Implementations belong to the caller that connects accounting data to a calculation.
/// For a monthly calculation, this view must expose the same account definitions,
/// currency, and signed balances as that calculation's `MonthlyInput::opening_ledger`.
/// The simulation boundary cannot verify this identity because the view does not
/// expose its source Ledger.
pub trait LedgerRead {
    fn currency(&self) -> property_simulation_domain::Currency;
    fn account(&self, id: property_simulation_domain::AccountId) -> Option<AccountFacts>;
    /// Return the signed balance, where debit balances are positive.
    fn signed_balance(
        &self,
        id: property_simulation_domain::AccountId,
    ) -> Result<property_simulation_domain::Money, AccountError>;
}

/// A transaction error found while preflighting an identified proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(test)]
pub struct ProposalValidationIssue {
    pub transaction_key: Option<String>,
    pub error: AccountError,
}

/// All transaction issues found while preflighting a proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(test)]
pub struct ProposalValidationErrors(pub Vec<ProposalValidationIssue>);

/// Inputs that have passed all checks required before sampling and transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedInput {
    opening_state: OpeningState,
    frozen_input: FrozenInput,
    preflighted_activities: Option<Vec<ActivityInstance>>,
}

impl ValidatedInput {
    pub fn opening_state(&self) -> &OpeningState {
        &self.opening_state
    }
    pub const fn frozen_input(&self) -> &FrozenInput {
        &self.frozen_input
    }

    /// Validate the complete active activity set and its monthly cash-flow
    /// support before any sampler is called. Random activity samplers require
    /// this preflight record to ensure the whole month's bounds were checked.
    pub fn validate_activities_before_sampling(
        &mut self,
        activities: &[ActivityInstance],
        ledger: &impl LedgerRead,
        additional_support: impl IntoIterator<
            Item = (
                property_simulation_accounting::CashFlowCategory,
                property_simulation_domain::Money,
            ),
        >,
    ) -> Result<(), SimulationError> {
        if self.preflighted_activities.is_some() {
            return Err(SimulationError::InvalidInput(
                "active activity set was already preflighted for this month",
            ));
        }
        self.validate_activity_configurations(activities, ledger)?;
        validate_cashflow_support_with_additional(
            &self.frozen_input,
            activities,
            additional_support,
        )?;
        self.preflighted_activities = Some(activities.to_vec());
        Ok(())
    }

    /// Check activity identity and account/configuration rules before any
    /// model calculation that depends on the active monthly composition.
    /// Cash-flow support is checked later, once model-derived amounts exist.
    pub(crate) fn validate_activity_configurations(
        &self,
        activities: &[ActivityInstance],
        ledger: &impl LedgerRead,
    ) -> Result<(), SimulationError> {
        let mut ids = std::collections::BTreeSet::new();
        for activity in activities {
            if !ids.insert(activity.id) {
                return Err(SimulationError::InvalidInput(
                    "duplicate activity instance in pre-sampling input",
                ));
            }
            validate_activity_configuration(activity, &self.frozen_input, ledger)?;
        }
        Ok(())
    }

    pub(crate) fn has_preflighted_activity(&self, activity: &ActivityInstance) -> bool {
        self.preflighted_activities
            .as_ref()
            .is_some_and(|activities| activities.iter().any(|candidate| candidate == activity))
    }
}

pub(crate) fn validate_activity_configuration(
    activity: &ActivityInstance,
    input: &FrozenInput,
    ledger: &impl LedgerRead,
) -> Result<(), SimulationError> {
    let (cash_account, expense_account, amount_valid, error) = match &activity.configuration {
        crate::ActivityConfiguration::PropertyTax(config) => (
            config.cash_account,
            config.expense_account,
            !config.amount.is_negative()
                && config.amount.currency() == ledger.currency()
                && config.amount.rounded_for_posting().is_ok(),
            "invalid property tax amount, currency, or account role",
        ),
        crate::ActivityConfiguration::Utility(config) => {
            let maximum = config.rate_per_unit.checked_mul_u64(config.maximum_usage);
            (
                config.cash_account,
                config.expense_account,
                !config.usage_unit.trim().is_empty()
                    && !config.rate_per_unit.is_negative()
                    && config.rate_per_unit.currency() == ledger.currency()
                    && maximum
                        .and_then(|amount| amount.rounded_for_posting())
                        .is_ok(),
                "invalid utility unit, maximum charge, currency, or account role",
            )
        }
        crate::ActivityConfiguration::RepairOccurrence(config) => (
            input.cash_account,
            config.cost_template.expense_account,
            config.cost_template.maximum.currency() == ledger.currency()
                && !config.cost_template.maximum.is_negative()
                && config.cost_template.maximum.rounded_for_posting().is_ok(),
            "invalid repair cost template amount, currency, or account role",
        ),
        crate::ActivityConfiguration::RepairCost(config) => (
            input.cash_account,
            config.expense_account,
            config.maximum.currency() == ledger.currency()
                && !config.maximum.is_negative()
                && config.maximum.rounded_for_posting().is_ok(),
            "invalid repair cost amount, currency, or account role",
        ),
        _ => return Ok(()),
    };
    let cash_valid = ledger.account(cash_account).is_some_and(|account| {
        account.category == AccountCategory::Asset && account.cash_position && !account.contra
    });
    let expense_valid = ledger
        .account(expense_account)
        .is_some_and(|account| account.category == AccountCategory::Expense && !account.contra);
    if amount_valid && cash_valid && expense_valid {
        Ok(())
    } else {
        Err(SimulationError::InvalidInput(error))
    }
}

/// Validate frozen values and collect every independent issue discoverable at this boundary.
pub(crate) fn validate_slice_input(
    input: &FrozenInput,
    ledger: &impl LedgerRead,
    state: &OpeningState,
) -> Result<ValidatedInput, ValidationErrors> {
    validate_input_inner(input, ledger, state)
}

fn validate_input_inner(
    input: &FrozenInput,
    ledger: &impl LedgerRead,
    state: &OpeningState,
) -> Result<ValidatedInput, ValidationErrors> {
    let mut issues = Vec::new();
    if input.rent.is_negative() {
        issues.push(ValidationIssue {
            field: "rent",
            message: "rent must be non-negative",
        });
    }
    let currency = ledger.currency();
    if input.rent.currency() != currency {
        issues.push(ValidationIssue {
            field: "rent",
            message: "rent and ledger must use the same currency",
        });
    }
    if state.cash.currency() != currency {
        issues.push(ValidationIssue {
            field: "opening_state.cash",
            message: "opening cash and frozen amounts must use the same currency",
        });
    }
    for (field, amount) in [("rent", input.rent)] {
        if !amount.is_negative() && amount.rounded_for_posting().is_err() {
            issues.push(ValidationIssue {
                field,
                message: "amount cannot be rounded for posting",
            });
        }
    }

    let cash_account_valid = match ledger.account(input.cash_account) {
        None => {
            issues.push(ValidationIssue {
                field: "cash_account",
                message: "cash account does not exist",
            });
            false
        }
        Some(account)
            if account.category != AccountCategory::Asset
                || !account.cash_position
                || account.contra =>
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
        match ledger.signed_balance(input.cash_account) {
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
    match ledger.account(input.rent_account) {
        None => issues.push(ValidationIssue {
            field: "rent_account",
            message: "rent account does not exist",
        }),
        Some(account) if account.category != AccountCategory::Income => {
            issues.push(ValidationIssue {
                field: "rent_account",
                message: "rent account must be an income account",
            });
        }
        Some(_) => {}
    }
    if issues.is_empty() {
        Ok(ValidatedInput {
            opening_state: state.clone(),
            frozen_input: *input,
            preflighted_activities: None,
        })
    } else {
        Err(ValidationErrors(issues))
    }
}

/// Ensure the sum of absolute cash-flow support for all active instances fits in `Money`.
/// This stronger bound makes every possible ordering and grouping of monthly contributions safe.
#[cfg(test)]
pub(crate) fn validate_cashflow_support(
    input: &FrozenInput,
    activities: &[ActivityInstance],
) -> Result<(), SimulationError> {
    validate_cashflow_support_with_additional(input, activities, std::iter::empty())
}

/// Add support for outputs whose bounds depend on typed month-start context, such as
/// loan continuations and explicit input postings. Each amount is an absolute
/// category contribution bound after within-transaction/category netting.
pub(crate) fn validate_cashflow_support_with_additional(
    input: &FrozenInput,
    activities: &[ActivityInstance],
    additional: impl IntoIterator<
        Item = (
            property_simulation_accounting::CashFlowCategory,
            property_simulation_domain::Money,
        ),
    >,
) -> Result<(), SimulationError> {
    let currency = input.rent.currency();
    let rent = input.rent.rounded_for_posting()?;
    let mut absolute_support = std::collections::BTreeMap::from([
        (
            property_simulation_accounting::CashFlowCategory::Operating,
            property_simulation_domain::Money::from_parts(0, 0, currency)?,
        ),
        (
            property_simulation_accounting::CashFlowCategory::Investing,
            property_simulation_domain::Money::from_parts(0, 0, currency)?,
        ),
        (
            property_simulation_accounting::CashFlowCategory::Financing,
            property_simulation_domain::Money::from_parts(0, 0, currency)?,
        ),
    ]);
    for activity in activities {
        let bound = match &activity.configuration {
            ActivityConfiguration::Rental => rent,
            ActivityConfiguration::RepairOccurrence(_) => {
                property_simulation_domain::Money::from_parts(0, 0, currency)?
            }
            ActivityConfiguration::RepairCost(config) => config.maximum.rounded_for_posting()?,
            ActivityConfiguration::Loan(_) => {
                property_simulation_domain::Money::from_parts(0, 0, currency)?
            }
            ActivityConfiguration::PropertyTax(config) => {
                if config.is_due(input.period) {
                    config.amount.rounded_for_posting()?
                } else {
                    property_simulation_domain::Money::from_parts(0, 0, currency)?
                }
            }
            ActivityConfiguration::Utility(config) => config
                .rate_per_unit
                .checked_mul_u64(config.maximum_usage)
                .map_err(|_| {
                    SimulationError::InvalidInput(
                        "utility maximum exact charge exceeds the representable Money range",
                    )
                })?
                .rounded_for_posting()
                .map_err(|_| {
                    SimulationError::InvalidInput(
                        "utility maximum charge cannot be rounded for posting",
                    )
                })?,
        };
        let total = absolute_support
            .get_mut(&property_simulation_accounting::CashFlowCategory::Operating)
            .expect("all cash-flow categories are initialized");
        *total = total.checked_add(bound).map_err(|_| {
            SimulationError::InvalidInput(
                "active activity cash-flow absolute support exceeds the representable Money range",
            )
        })?;
    }
    for (category, bound) in additional {
        if bound.currency() != currency || bound.is_negative() {
            return Err(SimulationError::InvalidInput(
                "additional cash-flow support must be non-negative and use the ledger currency",
            ));
        }
        let total = absolute_support
            .get_mut(&category)
            .expect("all cash-flow categories are initialized");
        *total = total.checked_add(bound).map_err(|_| {
            SimulationError::InvalidInput(
                "active activity cash-flow absolute support exceeds the representable Money range",
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn tax_due(
    period: property_simulation_domain::YearMonth,
    first_due: property_simulation_domain::YearMonth,
    cadence: crate::PropertyTaxCadence,
) -> bool {
    crate::PropertyTaxConfig::due_on(period, first_due, cadence)
}

/// Check all proposed transactions against a candidate ledger without mutating the caller's
/// ledger. `post_batch` applies the same account, currency, precision, cash classification,
/// balance, duplicate-ID, and month-end asset rules used for a real post.
#[cfg(test)]
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

    validate_transaction_batch(ledger, &transactions)
}

#[cfg(test)]
pub(crate) fn validate_transaction_batch(
    ledger: &Ledger,
    transactions: &[LedgerTransaction],
) -> Result<(), ProposalValidationErrors> {
    // Collect transaction-local errors without cloning the ledger per transaction. A valid
    // batch may contain individually invalid intermediate account balances, so the batch result
    // remains authoritative.
    let mut issues = Vec::new();
    for transaction in transactions {
        if let Err(error) = ledger.validate_transaction(transaction) {
            issues.push(ProposalValidationIssue {
                transaction_key: transaction.source().map(|source| source.output_key.clone()),
                error,
            });
        }
    }

    let mut candidate = ledger.clone();
    match candidate.post_batch(transactions.iter().cloned()) {
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
