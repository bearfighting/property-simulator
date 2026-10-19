use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};
use property_simulation_domain::{Money, RoundingPolicy};

use crate::activity::{SimulationSlice, ValidatedActivitySample};
use crate::error::SimulationError;
use crate::types::{
    ActivityInstance, ActivitySampleKind, AssetCondition, EffectProposal, EventProposal,
    FrozenInput, OpeningState, RepairSample, SliceProposal, TransactionProposal,
};

pub struct RepairSlice {
    pub instance: ActivityInstance,
}
impl SimulationSlice for RepairSlice {
    type Sample = RepairSample;
    const SAMPLE_KIND: ActivitySampleKind = ActivitySampleKind::Repair;
    fn instance(&self) -> &ActivityInstance {
        &self.instance
    }
    fn propose_validated(
        &self,
        _state: &OpeningState,
        input: &FrozenInput,
        sample: &ValidatedActivitySample<'_, RepairSample>,
    ) -> Result<SliceProposal, SimulationError> {
        let (cost, repair) = match sample.value() {
            RepairSample::NoRepair => (
                Money::new(0.into(), input.repair_cost_limit.currency()),
                false,
            ),
            RepairSample::Repair { cost } => (*cost, true),
        };
        if cost.is_negative()
            || cost.currency() != input.repair_cost_limit.currency()
            || cost.checked_cmp(input.repair_cost_limit)? == std::cmp::Ordering::Greater
        {
            return Err(SimulationError::InvalidInput(
                "repair cost outside configured support",
            ));
        }
        let posted_cost = cost.rounded_for_posting(RoundingPolicy::HalfUp)?;
        if !repair {
            return Ok(SliceProposal::default());
        }
        // A repair outcome is valid whether it resolves a known repair need or
        // represents preventive work on an already available property.
        let resulting_condition = AssetCondition::Available;
        let tx = TransactionProposal {
            key: "repair".into(),
            period: input.period,
            entries: vec![
                LedgerEntry::new(
                    input.repair_expense_account,
                    BalanceSide::Debit,
                    posted_cost,
                )
                .with_cash_flow_category(CashFlowCategory::Operating),
                LedgerEntry::new(input.cash_account, BalanceSide::Credit, posted_cost)
                    .with_cash_flow_category(CashFlowCategory::Operating),
            ],
            source_event_key: Some("repair_performed".into()),
        };
        Ok(SliceProposal {
            events: vec![EventProposal {
                period: input.period,
                key: "repair_performed".to_owned(),
            }],
            effects: vec![EffectProposal {
                period: input.period,
                target: "property.condition".to_owned(),
                key: "condition".to_owned(),
                source_event_key: Some("repair_performed".to_owned()),
                condition: resulting_condition,
            }],
            transactions: vec![tx],
        })
    }
}
