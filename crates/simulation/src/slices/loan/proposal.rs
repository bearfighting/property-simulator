use super::{LoanMonthOutput, LoanSlice, LoanStateEffectProposal};
use crate::{EventProposal, SimulationError, SliceProposal, TransactionProposal};
use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};
use property_simulation_domain::YearMonth;
use property_simulation_models::{FixedRateLoan, LoanMonth};

impl LoanSlice {
    /// Propose postings for a precomputed monthly loan result. The caller owns
    /// model validation and calculation; this method checks posting consistency
    /// and leaves composition, ID assignment, and commit to the caller.
    pub(crate) fn propose_month(
        &self,
        period: YearMonth,
        loan: FixedRateLoan,
        result: LoanMonth,
    ) -> Result<LoanMonthOutput, SimulationError> {
        let currency = loan.opening_balance.currency();
        let amounts = [
            result.interest,
            result.payment,
            result.principal,
            result.closing_balance,
            result.scheduled_payment,
        ];
        if amounts.iter().any(|amount| amount.currency() != currency)
            || amounts
                .iter()
                .any(|amount| amount.rounded_for_posting() != Ok(*amount))
            || result.interest.is_negative()
            || result.payment.is_negative()
            || result.payment.is_zero()
            || result.principal.is_negative()
            || result.closing_balance.is_negative()
            || result.scheduled_payment.is_negative()
            || result.scheduled_payment.is_zero()
            || result.payment != result.interest.checked_add(result.principal)?
            || result.closing_balance != loan.opening_balance.checked_sub(result.principal)?
            || result
                .payment
                .checked_cmp(loan.opening_balance.checked_add(result.interest)?)?
                == std::cmp::Ordering::Greater
            || result.remaining_months
                != if result.closing_balance.is_zero() {
                    0
                } else {
                    loan.remaining_months.saturating_sub(1)
                }
            || (result.closing_balance.is_zero() != (result.remaining_months == 0))
        {
            return Err(SimulationError::InvalidInput(
                "loan month output is inconsistent with its opening loan state",
            ));
        }
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
}
