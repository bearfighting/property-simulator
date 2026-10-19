use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};
use property_simulation_domain::RoundingPolicy;

use crate::activity::{SimulationSlice, ValidatedActivitySample};
use crate::error::SimulationError;
use crate::types::{
    ActivityInstance, ActivitySampleKind, EventProposal, FrozenInput, OpeningState, SliceProposal,
    TransactionProposal,
};

pub struct RentalSlice {
    pub instance: ActivityInstance,
}
impl SimulationSlice for RentalSlice {
    type Sample = ();
    const SAMPLE_KIND: ActivitySampleKind = ActivitySampleKind::Rental;
    fn instance(&self) -> &ActivityInstance {
        &self.instance
    }
    fn propose_validated(
        &self,
        _state: &OpeningState,
        input: &FrozenInput,
        _sample: &ValidatedActivitySample<'_, ()>,
    ) -> Result<SliceProposal, SimulationError> {
        if input.rent.is_negative() || input.rent.currency() != input.repair_cost_limit.currency() {
            return Err(SimulationError::InvalidInput("invalid rent"));
        }
        let rent = input.rent.rounded_for_posting(RoundingPolicy::HalfUp)?;
        if rent.is_zero() {
            return Ok(SliceProposal::default());
        }
        let event = EventProposal {
            period: input.period,
            key: "rent_collected".into(),
        };
        let tx = TransactionProposal {
            key: "rent".into(),
            period: input.period,
            entries: vec![
                LedgerEntry::new(input.cash_account, BalanceSide::Debit, rent)
                    .with_cash_flow_category(CashFlowCategory::Operating),
                LedgerEntry::new(input.rent_account, BalanceSide::Credit, rent),
            ],
            source_event_key: Some(event.key.clone()),
        };
        Ok(SliceProposal {
            events: vec![event],
            transactions: vec![tx],
            ..Default::default()
        })
    }
}
