use property_simulation_accounting::{
    AccountCategory, BalanceSide, CashFlowCategory, Ledger, LedgerEntry, LedgerTransaction,
};
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, EffectId, EventId, ModelVersionId, Money, SliceId,
    TrialId, YearMonth,
};
use property_simulation_models::{FixedRateLoan, LoanModelError, LoanMonth};

use crate::{
    CashFlowContribution, EventProposal, OutputIdFactory, SimulationError, SliceProposal,
    TransactionProposal,
};

/// Fixed-rate loan activity with an independently tracked liability account.
#[derive(Debug, Clone)]
pub struct LoanSlice {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub liability_account: AccountId,
    pub interest_expense_account: AccountId,
    pub cash_account: AccountId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanMonthOutput {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub period: YearMonth,
    pub result: LoanMonth,
    pub state_effect: LoanStateEffectProposal,
    pub proposal: SliceProposal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoanStateEffectProposal {
    pub updated: Option<FixedRateLoan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanCoordinatedMonth {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub cash_account: AccountId,
    pub liability_account: AccountId,
    pub period: YearMonth,
    pub closing_loan: Option<FixedRateLoan>,
    pub month: Option<LoanMonth>,
    pub ledger: Ledger,
    pub cash_flow_contributions: Vec<CashFlowContribution>,
    pub events: Vec<LoanEvent>,
    pub transactions: Vec<LedgerTransaction>,
    pub state_effect: Option<LoanStateEffect>,
}

#[derive(Debug, Clone)]
pub struct LoanMonthInput {
    pub slice: LoanSlice,
    pub loan: Option<FixedRateLoan>,
    pub update: Option<LoanTermsUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanContinuation {
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub loan: Option<FixedRateLoan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanStateEffect {
    pub id: EffectId,
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub period: YearMonth,
    pub previous: Option<FixedRateLoan>,
    pub updated: Option<FixedRateLoan>,
    pub source_event_id: EventId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanEvent {
    pub id: EventId,
    pub instance_id: ActivityInstanceId,
    pub slice_id: SliceId,
    pub model_version: ModelVersionId,
    pub period: YearMonth,
    pub key: String,
    pub previous_balance: Option<Money>,
    pub updated_balance: Option<Money>,
    pub previous_remaining_months: Option<u32>,
    pub updated_remaining_months: Option<u32>,
    pub previous_annual_rate: Option<rust_decimal::Decimal>,
    pub updated_annual_rate: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanTermsUpdate {
    pub(super) expected_current: FixedRateLoan,
    pub(super) updated: Option<FixedRateLoan>,
    pub(super) effective_period: YearMonth,
    pub(super) principal_change: Money,
    pub(super) principal_transaction: Option<LedgerTransaction>,
}

pub struct LoanMonthCommand<'a> {
    pub period: YearMonth,
    pub loan: Option<FixedRateLoan>,
    pub update: Option<LoanTermsUpdate>,
    pub opening_ledger: &'a Ledger,
    pub trial_id: TrialId,
    pub branch_id: BranchId,
    pub ids: &'a dyn OutputIdFactory,
}

impl LoanTermsUpdate {
    pub const fn expected_loan(&self) -> FixedRateLoan {
        self.expected_current
    }
    pub const fn effective_period(&self) -> YearMonth {
        self.effective_period
    }
    pub const fn updated_loan(&self) -> Option<FixedRateLoan> {
        self.updated
    }
    pub const fn principal_change(&self) -> Money {
        self.principal_change
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanRunUpdate {
    pub instance_id: ActivityInstanceId,
    pub update: LoanTermsUpdate,
}

impl LoanSlice {
    /// Propose postings for a typed monthly calculation. The coordinator remains
    /// responsible for calculation, validation, ID assignment, and atomic commit.
    pub fn propose_month(
        &self,
        period: YearMonth,
        loan: FixedRateLoan,
        result: LoanMonth,
    ) -> Result<LoanMonthOutput, SimulationError> {
        let event_key = "loan_payment".to_owned();
        let proposal = SliceProposal {
            events: vec![EventProposal {
                period,
                key: event_key.clone(),
            }],
            effects: Vec::new(),
            transactions: vec![TransactionProposal {
                key: "loan_payment".into(),
                period,
                entries: vec![
                    LedgerEntry::new(
                        self.interest_expense_account,
                        BalanceSide::Debit,
                        result.interest,
                    ),
                    LedgerEntry::new(self.liability_account, BalanceSide::Debit, result.principal),
                    LedgerEntry::new(self.cash_account, BalanceSide::Credit, result.interest)
                        .with_cash_flow_category(CashFlowCategory::Operating),
                    LedgerEntry::new(self.cash_account, BalanceSide::Credit, result.principal)
                        .with_cash_flow_category(CashFlowCategory::Financing),
                ],
                source_event_key: Some(event_key),
            }],
        };
        Ok(LoanMonthOutput {
            instance_id: self.instance_id,
            slice_id: self.slice_id,
            model_version: self.model_version,
            period,
            state_effect: LoanStateEffectProposal {
                updated: if result.closing_balance.is_zero() {
                    None
                } else {
                    Some(FixedRateLoan {
                        opening_balance: result.closing_balance,
                        remaining_months: result.remaining_months,
                        annual_rate: loan.annual_rate,
                        scheduled_payment: Some(result.scheduled_payment),
                    })
                },
            },
            result,
            proposal,
        })
    }

    /// Delegate to the loan coordinator. This convenience method performs no validation
    /// or ledger mutation itself.
    pub fn coordinate_month(
        &self,
        command: LoanMonthCommand<'_>,
    ) -> Result<LoanCoordinatedMonth, SimulationError> {
        super::loan_coordinator::coordinate_loan_month(self, command)
    }
}

pub(super) fn validate_loan_accounts(
    slice: &LoanSlice,
    ledger: &Ledger,
) -> Result<(), SimulationError> {
    let chart = ledger.chart();
    let liability = chart
        .get(slice.liability_account)
        .ok_or(SimulationError::InvalidInput(
            "loan liability account does not exist",
        ))?;
    if liability.category() != AccountCategory::Liability || liability.is_contra() {
        return Err(SimulationError::InvalidInput(
            "each loan must use a non-contra liability account",
        ));
    }
    let interest =
        chart
            .get(slice.interest_expense_account)
            .ok_or(SimulationError::InvalidInput(
                "interest expense account does not exist",
            ))?;
    if interest.category() != AccountCategory::Expense {
        return Err(SimulationError::InvalidInput(
            "loan interest must use an expense account",
        ));
    }
    let cash = chart
        .get(slice.cash_account)
        .ok_or(SimulationError::InvalidInput(
            "loan cash account does not exist",
        ))?;
    if cash.category() != AccountCategory::Asset || !cash.is_cash_position() || cash.is_contra() {
        return Err(SimulationError::InvalidInput(
            "loan payments must use a cash-position asset account",
        ));
    }
    Ok(())
}

pub(super) fn cash_classified_entry(
    account: AccountId,
    side: BalanceSide,
    amount: Money,
    cash_account: AccountId,
) -> LedgerEntry {
    let entry = LedgerEntry::new(account, side, amount);
    if account == cash_account {
        entry.with_cash_flow_category(CashFlowCategory::Financing)
    } else {
        entry
    }
}

pub(super) fn map_model_error(error: LoanModelError) -> SimulationError {
    match error {
        LoanModelError::Arithmetic => {
            SimulationError::InvalidInput("loan calculation exceeds supported decimal precision")
        }
        LoanModelError::NegativeRate => {
            SimulationError::InvalidInput("loan annual rate cannot be negative")
        }
        LoanModelError::NonPositiveBalance => {
            SimulationError::InvalidInput("active loan principal must be positive")
        }
        LoanModelError::NonPositiveTerm => {
            SimulationError::InvalidInput("active loan remaining term must be positive")
        }
        LoanModelError::InvalidScheduledPayment => {
            SimulationError::InvalidInput("carried loan payment is invalid for its schedule")
        }
        LoanModelError::DerivedPaymentRoundsToZero => SimulationError::InvalidInput(
            "derived loan payment rounds to zero at currency minor-unit precision",
        ),
        LoanModelError::InvalidPrincipalPrecision => {
            SimulationError::InvalidInput("loan principal exceeds currency minor-unit precision")
        }
    }
}

/// Validates a month-start terms update. `coordinate_month` commits any principal delta
/// as a balanced financing transaction in the same atomic batch as the monthly payment.
pub fn apply_loan_terms_update(
    current: FixedRateLoan,
    effective_period: YearMonth,
    opening_balance: property_simulation_domain::Money,
    remaining_months: u32,
    annual_rate: rust_decimal::Decimal,
    principal_transaction: Option<LedgerTransaction>,
) -> Result<LoanTermsUpdate, SimulationError> {
    current.validate().map_err(map_model_error)?;
    current.derive_payment().map_err(map_model_error)?;
    if opening_balance.currency() != current.opening_balance.currency()
        || opening_balance.is_negative()
    {
        return Err(SimulationError::InvalidInput(
            "loan balance update must be non-negative and use the loan currency",
        ));
    }
    let updated = if opening_balance.is_zero() {
        if remaining_months == 0 && annual_rate >= rust_decimal::Decimal::ZERO {
            None
        } else {
            return Err(SimulationError::InvalidInput(
                "a settled loan update must have zero remaining term and non-negative rate",
            ));
        }
    } else {
        let active = FixedRateLoan {
            opening_balance,
            remaining_months,
            annual_rate,
            scheduled_payment: None,
        };
        active.validate().map_err(map_model_error)?;
        active.derive_payment().map_err(map_model_error)?;
        Some(active)
    };
    let principal_change = opening_balance.checked_sub(current.opening_balance)?;
    if principal_change.is_zero() != principal_transaction.is_none() {
        return Err(SimulationError::InvalidInput(
            "a principal balance change requires its balanced ledger transaction",
        ));
    }
    if let Some(transaction) = &principal_transaction {
        if transaction.period() != effective_period {
            return Err(SimulationError::InvalidInput(
                "principal update transaction must use the effective month",
            ));
        }
    }
    Ok(LoanTermsUpdate {
        expected_current: current,
        updated,
        effective_period,
        principal_change,
        principal_transaction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssetCondition, FrozenInput, OpeningState, coordinate_month_with_loans};
    use property_simulation_accounting::{
        Account, AccountCode, ChartOfAccounts, OpeningBalance, OpeningBalances,
    };
    use property_simulation_domain::{Currency, EffectId, EventId, LedgerTransactionId, Money};

    struct Ids;
    impl OutputIdFactory for Ids {
        fn event_id(
            &self,
            _: TrialId,
            _: BranchId,
            _: YearMonth,
            _: ActivityInstanceId,
            key: &str,
        ) -> EventId {
            EventId::from_bytes([if key == "loan_payment" { 1 } else { 2 }; 16])
        }
        fn effect_id(
            &self,
            _: TrialId,
            _: BranchId,
            _: YearMonth,
            _: ActivityInstanceId,
            _: &str,
        ) -> EffectId {
            EffectId::from_bytes([3; 16])
        }
        fn transaction_id(
            &self,
            _: TrialId,
            _: BranchId,
            _: YearMonth,
            _: ActivityInstanceId,
            key: &str,
        ) -> LedgerTransactionId {
            LedgerTransactionId::from_bytes([if key == "loan_payment" { 4 } else { 5 }; 16])
        }
    }

    fn fixture(balance: &str) -> (LoanSlice, Ledger) {
        let currency = Currency::from_code("CAD").unwrap();
        let account = |n, code, category, name: &str| {
            Account::new(
                AccountId::from_bytes([n; 16]),
                AccountCode::new(code).unwrap(),
                category,
                false,
                name,
            )
            .unwrap()
        };
        let cash = Account::new_cash_position(
            AccountId::from_bytes([1; 16]),
            AccountCode::new(1000).unwrap(),
            "Cash",
        )
        .unwrap();
        let property = account(2, 1001, AccountCategory::Asset, "Property");
        let liability = account(3, 2000, AccountCategory::Liability, "Loan");
        let interest = account(4, 5000, AccountCategory::Expense, "Interest");
        let rent = account(5, 4000, AccountCategory::Income, "Rent");
        let chart = ChartOfAccounts::new([cash, property, liability, interest, rent]).unwrap();
        let balances = OpeningBalances::new(
            &chart,
            [
                OpeningBalance::new(
                    AccountId::from_bytes([2; 16]),
                    BalanceSide::Debit,
                    Money::parse(balance, currency).unwrap(),
                )
                .unwrap(),
                OpeningBalance::new(
                    AccountId::from_bytes([3; 16]),
                    BalanceSide::Credit,
                    Money::parse(balance, currency).unwrap(),
                )
                .unwrap(),
            ],
            None,
        )
        .unwrap();
        let ledger = Ledger::new(chart, balances).unwrap();
        let slice = LoanSlice {
            instance_id: ActivityInstanceId::from_bytes([7; 16]),
            slice_id: SliceId::from_bytes([8; 16]),
            model_version: ModelVersionId::from_bytes([9; 16]),
            liability_account: AccountId::from_bytes([3; 16]),
            interest_expense_account: AccountId::from_bytes([4; 16]),
            cash_account: AccountId::from_bytes([1; 16]),
        };
        (slice, ledger)
    }

    fn loan(balance: &str, months: u32) -> FixedRateLoan {
        FixedRateLoan {
            opening_balance: Money::parse(balance, Currency::from_code("CAD").unwrap()).unwrap(),
            remaining_months: months,
            annual_rate: rust_decimal::Decimal::new(12, 2),
            scheduled_payment: None,
        }
    }

    fn balance_transaction(
        slice: &LoanSlice,
        period: YearMonth,
        old_balance: Money,
        new_balance: Money,
    ) -> LedgerTransaction {
        let delta = new_balance.checked_sub(old_balance).unwrap();
        let amount = delta.abs().unwrap();
        let (debit, credit) = if delta.is_negative() {
            (slice.liability_account, slice.cash_account)
        } else {
            (slice.cash_account, slice.liability_account)
        };
        LedgerTransaction::new(
            LedgerTransactionId::from_bytes([12; 16]),
            period,
            [
                cash_classified_entry(debit, BalanceSide::Debit, amount, slice.cash_account),
                cash_classified_entry(credit, BalanceSide::Credit, amount, slice.cash_account),
            ],
        )
    }

    fn command<'a>(
        period: YearMonth,
        loan: Option<FixedRateLoan>,
        update: Option<LoanTermsUpdate>,
        ledger: &'a Ledger,
    ) -> LoanMonthCommand<'a> {
        LoanMonthCommand {
            period,
            loan,
            update,
            opening_ledger: ledger,
            trial_id: TrialId::from_bytes([10; 16]),
            branch_id: BranchId::from_bytes([11; 16]),
            ids: &Ids,
        }
    }

    #[test]
    fn proposes_separate_operating_interest_and_financing_principal() {
        let currency = Currency::from_code("CAD").unwrap();
        let id = |n| AccountId::from_bytes([n; 16]);
        let slice = LoanSlice {
            instance_id: ActivityInstanceId::from_bytes([1; 16]),
            slice_id: SliceId::from_bytes([2; 16]),
            model_version: ModelVersionId::from_bytes([3; 16]),
            liability_account: id(4),
            interest_expense_account: id(5),
            cash_account: id(6),
        };
        let loan = FixedRateLoan {
            opening_balance: Money::parse("10000", currency).unwrap(),
            remaining_months: 12,
            annual_rate: rust_decimal::Decimal::new(12, 2),
            scheduled_payment: None,
        };
        let period = YearMonth::new(2026, 1).unwrap();
        let month = loan.calculate_month().unwrap();
        let output = slice.propose_month(period, loan, month).unwrap();
        let entries = &output.proposal.transactions[0].entries;
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].amount(), Money::parse("100", currency).unwrap());
        assert_eq!(entries[0].side(), BalanceSide::Debit);
        assert_eq!(
            entries[1].amount(),
            Money::parse("788.49", currency).unwrap()
        );
        assert_eq!(
            entries[2].cash_flow_category(),
            Some(CashFlowCategory::Operating)
        );
        assert_eq!(
            entries[3].cash_flow_category(),
            Some(CashFlowCategory::Financing)
        );
        assert_eq!(
            entries[2]
                .amount()
                .checked_add(entries[3].amount())
                .unwrap(),
            output.result.payment
        );
        assert_eq!(output.period, period);
    }

    #[test]
    fn coordinated_payment_reconciles_ledger_and_cash_flow_and_then_noops_after_payoff() {
        let (slice, ledger) = fixture("10000");
        let period = YearMonth::new(2026, 1).unwrap();
        let result = slice
            .coordinate_month(command(period, Some(loan("10000", 12)), None, &ledger))
            .unwrap();
        assert_eq!(
            result.ledger.balance(slice.liability_account).unwrap().1,
            result.month.unwrap().closing_balance
        );
        let flow = result
            .ledger
            .project(period, period, &[slice.cash_account])
            .unwrap()
            .cash_flow_statement
            .totals;
        assert_eq!(
            flow.operating,
            Money::parse("-100", Currency::from_code("CAD").unwrap()).unwrap()
        );
        assert_eq!(
            flow.financing,
            Money::parse("-788.49", Currency::from_code("CAD").unwrap()).unwrap()
        );
        let posted = result.ledger.transactions().last().unwrap();
        let source_event = posted.source().unwrap().source_event_id.unwrap();
        assert!(
            result
                .events
                .iter()
                .any(|event| event.id == source_event && event.key == "loan_payment")
        );
        let (slice, ledger) = fixture("100");
        let final_month = slice
            .coordinate_month(command(period, Some(loan("100", 1)), None, &ledger))
            .unwrap();
        assert_eq!(final_month.closing_loan, None);
        let continued = slice
            .coordinate_month(command(
                YearMonth::new(2026, 2).unwrap(),
                None,
                None,
                &final_month.ledger,
            ))
            .unwrap();
        assert_eq!(continued.ledger, final_month.ledger);
        assert!(continued.cash_flow_contributions.is_empty());

        let (inactive_slice, unsettled_ledger) = fixture("100");
        assert!(
            inactive_slice
                .coordinate_month(command(
                    YearMonth::new(2026, 2).unwrap(),
                    None,
                    None,
                    &unsettled_ledger,
                ))
                .is_err()
        );
    }

    #[test]
    fn principal_terms_update_is_committed_with_payment_and_cash_delta() {
        let (slice, ledger) = fixture("10000");
        let current = loan("10000", 12);
        let period = YearMonth::new(2026, 1).unwrap();
        let update = apply_loan_terms_update(
            current,
            period,
            Money::parse("11000", Currency::from_code("CAD").unwrap()).unwrap(),
            13,
            rust_decimal::Decimal::new(12, 2),
            Some(balance_transaction(
                &slice,
                period,
                current.opening_balance,
                Money::parse("11000", Currency::from_code("CAD").unwrap()).unwrap(),
            )),
        )
        .unwrap();
        let result = slice
            .coordinate_month(command(period, Some(current), Some(update), &ledger))
            .unwrap();
        assert_eq!(
            result.ledger.balance(slice.liability_account).unwrap().1,
            result.month.unwrap().closing_balance
        );
        assert_eq!(
            result.cash_flow_contributions[0].category,
            CashFlowCategory::Financing
        );
        assert_eq!(
            result.cash_flow_contributions[0].amount,
            Money::parse("1000", Currency::from_code("CAD").unwrap()).unwrap()
        );
        let update_tx = result
            .ledger
            .transactions()
            .find(|tx| tx.source().unwrap().output_key == "loan_balance_update")
            .unwrap();
        let update_event_id = update_tx.source().unwrap().source_event_id.unwrap();
        assert!(
            result
                .events
                .iter()
                .any(|event| event.id == update_event_id && event.key == "loan_terms_update")
        );
        assert_eq!(
            result
                .ledger
                .project(period, period, &[slice.cash_account])
                .unwrap()
                .cash_flow_statement
                .totals
                .financing,
            Money::parse("203.44", Currency::from_code("CAD").unwrap()).unwrap()
        );
        let totals = crate::CashFlowContributionTotals::sum(
            result
                .cash_flow_contributions
                .iter()
                .map(|item| (item.category, item.amount)),
            Currency::from_code("CAD").unwrap(),
        )
        .unwrap();
        let projection = result
            .ledger
            .project(period, period, &[slice.cash_account])
            .unwrap()
            .cash_flow_statement
            .totals;
        assert_eq!(totals.operating, projection.operating);
        assert_eq!(totals.financing, projection.financing);
    }

    #[test]
    fn rate_only_change_is_recorded_and_zero_balance_update_settles_without_payment() {
        let (slice, ledger) = fixture("10000");
        let current = loan("10000", 12);
        let period = YearMonth::new(2026, 1).unwrap();
        let rate_update = apply_loan_terms_update(
            current,
            period,
            current.opening_balance,
            current.remaining_months,
            rust_decimal::Decimal::new(6, 2),
            None,
        )
        .unwrap();
        let result = slice
            .coordinate_month(command(period, Some(current), Some(rate_update), &ledger))
            .unwrap();
        let terms_event = result
            .events
            .iter()
            .find(|event| event.key == "loan_terms_update")
            .unwrap();
        assert_eq!(terms_event.previous_annual_rate, Some(current.annual_rate));
        assert_eq!(
            terms_event.updated_annual_rate,
            Some(rust_decimal::Decimal::new(6, 2))
        );
        assert_eq!(
            result.closing_loan.unwrap().annual_rate,
            rust_decimal::Decimal::new(6, 2)
        );

        let (slice, ledger) = fixture("100");
        let current = loan("100", 12);
        let settled = Money::parse("0", Currency::from_code("CAD").unwrap()).unwrap();
        let payoff_period = YearMonth::new(2026, 1).unwrap();
        let update = apply_loan_terms_update(
            current,
            payoff_period,
            settled,
            0,
            current.annual_rate,
            Some(balance_transaction(
                &slice,
                payoff_period,
                current.opening_balance,
                settled,
            )),
        )
        .unwrap();
        let payoff = slice
            .coordinate_month(command(payoff_period, Some(current), Some(update), &ledger))
            .unwrap();
        assert_eq!(payoff.closing_loan, None);
        assert_eq!(payoff.month, None);
        assert_eq!(
            payoff.ledger.balance(slice.liability_account).unwrap().1,
            settled
        );
        assert_eq!(payoff.cash_flow_contributions.len(), 1);
        assert_eq!(
            payoff.cash_flow_contributions[0].amount,
            Money::parse("-100", settled.currency()).unwrap()
        );
    }

    #[test]
    fn mismatched_caller_principal_transaction_is_rejected_before_any_commit() {
        let (slice, ledger) = fixture("10000");
        let period = YearMonth::new(2026, 1).unwrap();
        let current = loan("10000", 12);
        let update = apply_loan_terms_update(
            current,
            period,
            Money::parse("11000", Currency::from_code("CAD").unwrap()).unwrap(),
            13,
            rust_decimal::Decimal::new(12, 2),
            Some(balance_transaction(
                &slice,
                period,
                current.opening_balance,
                Money::parse("10999", Currency::from_code("CAD").unwrap()).unwrap(),
            )),
        )
        .unwrap();
        assert!(
            slice
                .coordinate_month(command(period, Some(current), Some(update), &ledger))
                .is_err()
        );
        assert_eq!(
            ledger.balance(slice.liability_account).unwrap().1,
            current.opening_balance
        );
        assert_eq!(ledger.transactions().count(), 0);
    }

    #[test]
    fn loan_composition_is_retained_in_month_result_and_snapshot_payload() {
        let (slice, ledger) = fixture("10000");
        let currency = Currency::from_code("CAD").unwrap();
        let period = YearMonth::new(2026, 1).unwrap();
        let frozen = FrozenInput {
            period,
            rent: Money::parse("0", currency).unwrap(),
            rent_account: AccountId::from_bytes([5; 16]),
            cash_account: slice.cash_account,
            repair_expense_account: slice.interest_expense_account,
            repair_cost_limit: Money::parse("0", currency).unwrap(),
        };
        let state = OpeningState {
            cash: Money::parse("0", currency).unwrap(),
            condition: AssetCondition::Available,
        };
        let distribution = property_simulation_models::RepairCostDistribution {
            maximum: frozen.repair_cost_limit,
        };
        let input = crate::validate_input(&frozen, &distribution, &ledger, &state).unwrap();
        let (result, committed) = coordinate_month_with_loans(
            &input,
            &ledger,
            vec![],
            vec![],
            TrialId::from_bytes([10; 16]),
            BranchId::from_bytes([11; 16]),
            &Ids,
            vec![LoanMonthInput {
                slice: slice.clone(),
                loan: Some(loan("10000", 12)),
                update: None,
            }],
        )
        .unwrap();
        assert_eq!(result.activities().len(), 1);
        assert_eq!(result.loan_continuations().len(), 1);
        assert_eq!(
            result.loan_continuations()[0].loan.unwrap().opening_balance,
            Money::parse("9211.51", currency).unwrap()
        );
        assert_eq!(result.loan_events().len(), 1);
        assert_eq!(result.loan_effects().len(), 1);
        assert_eq!(
            result.loan_effects()[0].source_event_id,
            result.loan_events()[0].id
        );
        assert_eq!(result.cash_flow_contributions().len(), 2);
        assert_eq!(
            result.closing_state().cash,
            Money::parse("-888.49", currency).unwrap()
        );
        let projected = committed
            .project(period, period, &[slice.cash_account])
            .unwrap()
            .cash_flow_statement
            .totals;
        let contributed = result.cash_flow_totals().unwrap();
        assert_eq!(contributed.operating, projected.operating);
        assert_eq!(contributed.financing, projected.financing);
        assert_eq!(
            committed.balance(slice.liability_account).unwrap().1,
            Money::parse("9211.51", currency).unwrap()
        );
    }

    #[test]
    fn rejects_bad_account_roles_and_wrong_effective_month_before_commit() {
        let (slice, ledger) = fixture("10000");
        let period = YearMonth::new(2026, 1).unwrap();
        let mut bad_slice = slice.clone();
        bad_slice.liability_account = slice.interest_expense_account;
        assert!(
            bad_slice
                .coordinate_month(command(period, Some(loan("10000", 12)), None, &ledger))
                .is_err()
        );
        let update = apply_loan_terms_update(
            loan("10000", 12),
            YearMonth::new(2026, 2).unwrap(),
            Money::parse("11000", Currency::from_code("CAD").unwrap()).unwrap(),
            13,
            rust_decimal::Decimal::new(12, 2),
            Some(balance_transaction(
                &slice,
                YearMonth::new(2026, 2).unwrap(),
                loan("10000", 12).opening_balance,
                Money::parse("11000", Currency::from_code("CAD").unwrap()).unwrap(),
            )),
        )
        .unwrap();
        assert!(
            slice
                .coordinate_month(command(
                    period,
                    Some(loan("10000", 12)),
                    Some(update),
                    &ledger
                ))
                .is_err()
        );
        assert_eq!(
            ledger.balance(slice.liability_account).unwrap().1,
            Money::parse("10000", Currency::from_code("CAD").unwrap()).unwrap()
        );
    }
}
