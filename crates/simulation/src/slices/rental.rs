use std::collections::BTreeSet;

use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};

use crate::activity::{SimulationSlice, ValidatedActivitySample};
use crate::composition::{SliceResources, account_resource_key};
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
    fn resources(&self, input: &FrozenInput) -> SliceResources {
        SliceResources {
            instance_id: self.instance.id,
            slice_id: self.instance.slice_id,
            reads: BTreeSet::new(),
            writes: [input.cash_account, input.rent_account]
                .into_iter()
                .map(account_resource_key)
                .collect(),
        }
    }
    fn sample_value(&self, _sample: &Self::Sample) -> crate::activity::ActivitySampleValue {
        crate::activity::ActivitySampleValue::Rental
    }
    fn propose_validated(
        &self,
        _state: &OpeningState,
        input: &FrozenInput,
        _sample: &ValidatedActivitySample<'_, ()>,
    ) -> Result<SliceProposal, SimulationError> {
        let rent = input.rent.rounded_for_posting()?;
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
