use super::{LoanMonthOutput, LoanSlice, LoanStateEffectProposal};
use crate::{EventProposal, SimulationError, SliceProposal, TransactionProposal};
use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};
use property_simulation_domain::YearMonth;
use property_simulation_models::{FixedRateLoan, LoanMonth};

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
