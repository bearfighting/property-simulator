use crate::slices::loan::*;
use crate::{
    ActivityConfiguration, ActivityInstance, ActivitySample, AssetCondition, FrozenInput,
    OpeningState, PropertyTaxCadence, PropertyTaxConfig, PropertyTaxSlice, propose_slice,
};
use crate::{OutputIdAllocator, coordinate_loan_month, coordinate_month_with_loans};
use property_simulation_accounting::{
    Account, AccountCategory, AccountCode, ChartOfAccounts, OpeningBalance, OpeningBalances,
};
use property_simulation_accounting::{
    BalanceSide, CashFlowCategory, Ledger, LedgerEntry, LedgerTransaction,
};
use property_simulation_domain::{
    AccountId, ActivityInstanceId, BranchId, SliceId, TrialId, YearMonth,
};
use property_simulation_domain::{Currency, EffectId, EventId, LedgerTransactionId, Money};
use property_simulation_models::FixedRateLoan;

fn prepare_loan_terms_update(
    current: FixedRateLoan,
    effective_period: YearMonth,
    opening_balance: Money,
    remaining_months: u32,
    annual_rate: rust_decimal::Decimal,
    principal_transaction: Option<LedgerTransaction>,
) -> Result<LoanTermsUpdate, crate::SimulationError> {
    apply_loan_terms_update(
        current,
        effective_period,
        opening_balance,
        remaining_months,
        annual_rate,
        principal_transaction,
        &crate::tests::TestLoanCalculation,
    )
}

struct Ids;
impl OutputIdAllocator for Ids {
    fn activity_instance_id(
        &self,
        _period: YearMonth,
        _instance: ActivityInstanceId,
        _key: &str,
    ) -> ActivityInstanceId {
        ActivityInstanceId::from_bytes([250; 16])
    }
    fn event_id(&self, _: YearMonth, _: ActivityInstanceId, key: &str) -> EventId {
        EventId::from_bytes([if key == "loan_payment" { 1 } else { 2 }; 16])
    }
    fn effect_id(&self, _: YearMonth, _: ActivityInstanceId, _: &str) -> EffectId {
        EffectId::from_bytes([3; 16])
    }
    fn transaction_id(
        &self,
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

#[test]
fn property_tax_composes_with_a_loan_in_the_same_month() {
    let (loan_slice, ledger) = fixture("10000");
    let currency = Currency::from_code("CAD").unwrap();
    let period = YearMonth::new(2026, 1).unwrap();
    let frozen = FrozenInput {
        period,
        rent: Money::parse("0", currency).unwrap(),
        rent_account: AccountId::from_bytes([5; 16]),
        cash_account: loan_slice.cash_account,
    };
    let state = OpeningState {
        cash: Money::parse("0", currency).unwrap(),
        condition: AssetCondition::Available,
    };
    let mut input = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let tax = ActivityInstance {
        id: ActivityInstanceId::from_bytes([40; 16]),
        slice_id: SliceId::from_bytes([41; 16]),
        configuration: ActivityConfiguration::PropertyTax(PropertyTaxConfig {
            amount: Money::parse("25", currency).unwrap(),
            cadence: PropertyTaxCadence::Monthly,
            first_due: period,
            cash_account: loan_slice.cash_account,
            expense_account: loan_slice.interest_expense_account,
        }),
    };
    let slice = PropertyTaxSlice {
        instance: tax.clone(),
    };
    input
        .validate_activities_before_sampling(std::slice::from_ref(&tax), &ledger, [])
        .unwrap();
    let output = propose_slice(&slice, &input, &ActivitySample::new(&tax, period, ())).unwrap();

    let (result, committed) = coordinate_month_with_loans(
        &input,
        &ledger,
        vec![output],
        TrialId::from_bytes([10; 16]),
        BranchId::from_bytes([11; 16]),
        &Ids,
        vec![LoanMonthInput {
            slice: loan_slice.clone(),
            loan: Some(loan("10000", 12)),
            update: None,
        }],
    )
    .unwrap();
    assert_eq!(result.activities().len(), 2);
    assert_eq!(result.proposal().transactions().len(), 2);
    assert_eq!(result.cash_flow_contributions().len(), 3);
    assert!(committed.transactions().count() >= 2);
}

#[test]
fn loan_aware_coordinator_accepts_a_month_with_no_active_activities() {
    let (slice, ledger) = fixture("10000");
    let currency = Currency::from_code("CAD").unwrap();
    let period = YearMonth::new(2026, 1).unwrap();
    let frozen = FrozenInput {
        period,
        rent: Money::parse("0", currency).unwrap(),
        rent_account: AccountId::from_bytes([5; 16]),
        cash_account: slice.cash_account,
    };
    let state = OpeningState {
        cash: Money::parse("0", currency).unwrap(),
        condition: AssetCondition::Available,
    };
    let input = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let (result, unchanged) = coordinate_month_with_loans(
        &input,
        &ledger,
        vec![],
        TrialId::from_bytes([10; 16]),
        BranchId::from_bytes([11; 16]),
        &Ids,
        vec![],
    )
    .unwrap();
    assert!(result.activities().is_empty());
    assert!(result.proposal().events().is_empty());
    assert!(result.proposal().transactions().is_empty());
    assert!(result.cash_flow_contributions().is_empty());
    assert_eq!(unchanged, ledger);
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
fn loan_input_validation_checks_account_roles_and_ledger_currency() {
    let (slice, ledger) = fixture("10000");
    validate_loan_input(
        &slice,
        Some(loan("10000", 12)),
        &ledger,
        &crate::tests::TestLoanCalculation,
    )
    .unwrap();

    let invalid_roles = LoanSlice {
        interest_expense_account: slice.liability_account,
        ..slice.clone()
    };
    assert!(
        validate_loan_input(
            &invalid_roles,
            Some(loan("10000", 12)),
            &ledger,
            &crate::tests::TestLoanCalculation,
        )
        .is_err()
    );

    let usd_loan = FixedRateLoan {
        opening_balance: Money::parse("10000", Currency::from_code("USD").unwrap()).unwrap(),
        ..loan("10000", 12)
    };
    assert!(
        validate_loan_input(
            &slice,
            Some(usd_loan),
            &ledger,
            &crate::tests::TestLoanCalculation,
        )
        .is_err()
    );

    assert!(
        validate_loan_input(
            &slice,
            Some(loan("9000", 12)),
            &ledger,
            &crate::tests::TestLoanCalculation,
        )
        .is_err()
    );
}

#[test]
fn loan_input_validation_rejects_a_schedule_that_cannot_calculate_its_first_month() {
    struct FailingMonthCalculation;
    impl LoanCalculation for FailingMonthCalculation {
        fn validate(
            &self,
            _loan: FixedRateLoan,
        ) -> Result<(), property_simulation_models::LoanModelError> {
            Ok(())
        }
        fn derive_payment(
            &self,
            loan: FixedRateLoan,
        ) -> Result<Money, property_simulation_models::LoanModelError> {
            Ok(Money::parse("900", loan.opening_balance.currency()).unwrap())
        }
        fn calculate_month(
            &self,
            _loan: FixedRateLoan,
        ) -> Result<property_simulation_models::LoanMonth, property_simulation_models::LoanModelError>
        {
            Err(property_simulation_models::LoanModelError::Arithmetic)
        }
    }

    let (slice, ledger) = fixture("10000");
    assert!(
        validate_loan_input(
            &slice,
            Some(loan("10000", 12)),
            &ledger,
            &FailingMonthCalculation,
        )
        .is_err()
    );
}

#[test]
fn rejects_a_loan_month_whose_postings_do_not_reconcile_to_the_opening_loan() {
    let currency = Currency::from_code("CAD").unwrap();
    let id = |n| AccountId::from_bytes([n; 16]);
    let slice = LoanSlice {
        instance_id: ActivityInstanceId::from_bytes([1; 16]),
        slice_id: SliceId::from_bytes([2; 16]),
        liability_account: id(4),
        interest_expense_account: id(5),
        cash_account: id(6),
    };
    let loan = loan("10000", 12);
    let mut invalid = loan.calculate_month().unwrap();
    invalid.principal = Money::parse("1", currency).unwrap();
    assert!(
        slice
            .propose_month(YearMonth::new(2026, 1).unwrap(), loan, invalid,)
            .is_err()
    );

    invalid.interest = Money::parse("100", currency).unwrap();
    invalid.payment = Money::parse("100", currency).unwrap();
    invalid.principal = Money::parse("1", currency).unwrap();
    invalid.scheduled_payment = Money::parse("100", currency).unwrap();
    invalid.closing_balance = loan.opening_balance;
    invalid.remaining_months = loan.remaining_months - 1;
    assert!(
        slice
            .propose_month(YearMonth::new(2026, 1).unwrap(), loan, invalid,)
            .is_err()
    );

    invalid.interest = Money::parse("0", currency).unwrap();
    invalid.payment = Money::parse("0", currency).unwrap();
    invalid.principal = Money::parse("0", currency).unwrap();
    invalid.scheduled_payment = Money::parse("0", currency).unwrap();
    invalid.closing_balance = loan.opening_balance;
    assert!(
        slice
            .propose_month(YearMonth::new(2026, 1).unwrap(), loan, invalid,)
            .is_err()
    );
}

#[test]
fn rejects_loan_calculation_amounts_that_cannot_be_posted_at_currency_precision() {
    struct FractionalCalculation;
    impl LoanCalculation for FractionalCalculation {
        fn validate(
            &self,
            _loan: FixedRateLoan,
        ) -> Result<(), property_simulation_models::LoanModelError> {
            Ok(())
        }
        fn derive_payment(
            &self,
            loan: FixedRateLoan,
        ) -> Result<Money, property_simulation_models::LoanModelError> {
            Ok(Money::parse("1.001", loan.opening_balance.currency()).unwrap())
        }
        fn calculate_month(
            &self,
            loan: FixedRateLoan,
        ) -> Result<property_simulation_models::LoanMonth, property_simulation_models::LoanModelError>
        {
            let currency = loan.opening_balance.currency();
            Ok(property_simulation_models::LoanMonth {
                interest: Money::parse("0.001", currency).unwrap(),
                payment: Money::parse("1.001", currency).unwrap(),
                principal: Money::parse("1", currency).unwrap(),
                closing_balance: loan
                    .opening_balance
                    .checked_sub(Money::parse("1", currency).unwrap())
                    .unwrap(),
                remaining_months: loan.remaining_months - 1,
                scheduled_payment: Money::parse("1.001", currency).unwrap(),
            })
        }
    }

    let (slice, _) = fixture("10000");
    let loan = loan("10000", 12);
    assert!(
        slice
            .propose_month(
                YearMonth::new(2026, 1).unwrap(),
                loan,
                FractionalCalculation.calculate_month(loan).unwrap(),
            )
            .is_err()
    );
}

#[test]
fn coordinated_payment_reconciles_ledger_and_cash_flow_and_then_noops_after_payoff() {
    let (slice, ledger) = fixture("10000");
    let period = YearMonth::new(2026, 1).unwrap();
    let result = coordinate_loan_month(
        &slice,
        command(period, Some(loan("10000", 12)), None, &ledger),
    )
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
    let final_month =
        coordinate_loan_month(&slice, command(period, Some(loan("100", 1)), None, &ledger))
            .unwrap();
    assert_eq!(final_month.closing_loan, None);
    let continued = coordinate_loan_month(
        &slice,
        command(
            YearMonth::new(2026, 2).unwrap(),
            None,
            None,
            &final_month.ledger,
        ),
    )
    .unwrap();
    assert_eq!(continued.ledger, final_month.ledger);
    assert!(continued.cash_flow_contributions.is_empty());

    let (inactive_slice, unsettled_ledger) = fixture("100");
    assert!(
        coordinate_loan_month(
            &inactive_slice,
            command(
                YearMonth::new(2026, 2).unwrap(),
                None,
                None,
                &unsettled_ledger,
            )
        )
        .is_err()
    );
}

#[test]
fn principal_terms_update_is_committed_with_payment_and_cash_delta() {
    let (slice, ledger) = fixture("10000");
    let current = loan("10000", 12);
    let period = YearMonth::new(2026, 1).unwrap();
    let update = prepare_loan_terms_update(
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
    let result = coordinate_loan_month(
        &slice,
        command(period, Some(current), Some(update), &ledger),
    )
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
    let rate_update = prepare_loan_terms_update(
        current,
        period,
        current.opening_balance,
        current.remaining_months,
        rust_decimal::Decimal::new(6, 2),
        None,
    )
    .unwrap();
    let result = coordinate_loan_month(
        &slice,
        command(period, Some(current), Some(rate_update), &ledger),
    )
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
    let update = prepare_loan_terms_update(
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
    let payoff = coordinate_loan_month(
        &slice,
        command(payoff_period, Some(current), Some(update), &ledger),
    )
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
    let result = apply_loan_terms_update(
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
        &crate::tests::TestLoanCalculation,
    );
    assert!(result.is_err());
    assert_eq!(
        ledger.balance(slice.liability_account).unwrap().1,
        current.opening_balance
    );
    assert_eq!(ledger.transactions().count(), 0);
}

#[test]
fn principal_increase_rejects_a_reversed_financing_posting_at_input_validation() {
    let (slice, _) = fixture("10000");
    let current = loan("10000", 12);
    let amount = Money::parse("1000", Currency::from_code("CAD").unwrap()).unwrap();
    let reversed = LedgerTransaction::new(
        LedgerTransactionId::from_bytes([13; 16]),
        YearMonth::new(2026, 1).unwrap(),
        [
            LedgerEntry::new(slice.cash_account, BalanceSide::Credit, amount)
                .with_cash_flow_category(CashFlowCategory::Financing),
            LedgerEntry::new(slice.liability_account, BalanceSide::Debit, amount),
        ],
    );
    assert!(
        apply_loan_terms_update(
            current,
            YearMonth::new(2026, 1).unwrap(),
            Money::parse("11000", Currency::from_code("CAD").unwrap()).unwrap(),
            13,
            current.annual_rate,
            Some(reversed),
            &crate::tests::TestLoanCalculation,
        )
        .is_err()
    );
}

#[test]
fn terms_update_boundary_checks_configured_account_references() {
    let (slice, ledger) = fixture("10000");
    let current = loan("10000", 12);
    let period = YearMonth::new(2026, 1).unwrap();
    let updated_balance = Money::parse("11000", Currency::from_code("CAD").unwrap()).unwrap();
    let valid = prepare_loan_terms_update(
        current,
        period,
        updated_balance,
        13,
        current.annual_rate,
        Some(balance_transaction(
            &slice,
            period,
            current.opening_balance,
            updated_balance,
        )),
    )
    .unwrap();
    validate_loan_terms_update(
        &slice,
        &valid,
        period,
        &ledger,
        &crate::tests::TestLoanCalculation,
    )
    .unwrap();

    let amount = Money::parse("1000", Currency::from_code("CAD").unwrap()).unwrap();
    let wrong_account_tx = LedgerTransaction::new(
        LedgerTransactionId::from_bytes([14; 16]),
        period,
        [
            LedgerEntry::new(slice.cash_account, BalanceSide::Debit, amount)
                .with_cash_flow_category(CashFlowCategory::Financing),
            LedgerEntry::new(slice.interest_expense_account, BalanceSide::Credit, amount),
        ],
    );
    let invalid = prepare_loan_terms_update(
        current,
        period,
        updated_balance,
        13,
        current.annual_rate,
        Some(wrong_account_tx),
    )
    .unwrap();
    assert!(
        validate_loan_terms_update(
            &slice,
            &invalid,
            period,
            &ledger,
            &crate::tests::TestLoanCalculation,
        )
        .is_err()
    );
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
    };
    let state = OpeningState {
        cash: Money::parse("0", currency).unwrap(),
        condition: AssetCondition::Available,
    };
    let input = crate::validation::validate_slice_input(&frozen, &ledger, &state).unwrap();
    let (result, committed) = coordinate_month_with_loans(
        &input,
        &ledger,
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
        coordinate_loan_month(
            &bad_slice,
            command(period, Some(loan("10000", 12)), None, &ledger)
        )
        .is_err()
    );
    let update = prepare_loan_terms_update(
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
        coordinate_loan_month(
            &slice,
            command(period, Some(loan("10000", 12)), Some(update), &ledger)
        )
        .is_err()
    );
    assert_eq!(
        ledger.balance(slice.liability_account).unwrap().1,
        Money::parse("10000", Currency::from_code("CAD").unwrap()).unwrap()
    );
}
