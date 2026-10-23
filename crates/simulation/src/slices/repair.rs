use std::collections::BTreeSet;

use crate::activity::{SimulationSlice, ValidatedActivitySample};
use crate::composition::SliceResources;
use crate::error::SimulationError;
use crate::types::{
    ActivityConfiguration, ActivityInstance, AssetCondition, EffectProposal, EventProposal,
    FrozenInput, OpeningState, SliceProposal, TransactionProposal,
};
use crate::validation::ValidatedInput;
use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};

/// Monthly event process. It never posts to the Ledger; an occurrence event is
/// used by the monthly coordinator to create a delayed RepairCost instance.
pub struct RepairOccurrenceSlice {
    pub instance: ActivityInstance,
}

impl SimulationSlice for RepairOccurrenceSlice {
    type Sample = property_simulation_models::RepairOccurrenceSample;
    fn instance(&self) -> &ActivityInstance {
        &self.instance
    }
    fn resources(&self, _input: &FrozenInput) -> SliceResources {
        SliceResources {
            instance_id: self.instance.id,
            slice_id: self.instance.slice_id,
            reads: BTreeSet::new(),
            writes: BTreeSet::new(),
        }
    }
    fn sample_value(&self, sample: &Self::Sample) -> crate::activity::ActivitySampleValue {
        crate::activity::ActivitySampleValue::RepairOccurrence(*sample)
    }
    fn propose_validated(
        &self,
        _state: &OpeningState,
        input: &FrozenInput,
        sample: &ValidatedActivitySample<'_, Self::Sample>,
    ) -> Result<SliceProposal, SimulationError> {
        if !sample.value().0 {
            return Ok(SliceProposal::default());
        }
        Ok(SliceProposal {
            events: vec![EventProposal {
                period: input.period,
                key: "repair_occurrence".into(),
            }],
            ..SliceProposal::default()
        })
    }
}

/// One-time cost settlement. The occurrence decision and amount distribution
/// are separate, and the source event remains explicit on the instance.
pub struct RepairCostSlice {
    pub instance: ActivityInstance,
}

impl SimulationSlice for RepairCostSlice {
    type Sample = property_simulation_models::RepairCostSample;
    fn instance(&self) -> &ActivityInstance {
        &self.instance
    }
    fn resources(&self, input: &FrozenInput) -> SliceResources {
        let ActivityConfiguration::RepairCost(config) = &self.instance.configuration else {
            unreachable!()
        };
        SliceResources {
            instance_id: self.instance.id,
            slice_id: self.instance.slice_id,
            reads: BTreeSet::new(),
            writes: [
                property_simulation_domain::ResourceKey::for_account(config.expense_account),
                property_simulation_domain::ResourceKey::for_account(input.cash_account),
                property_simulation_domain::ResourceKey::new("property.condition")
                    .expect("static resource key"),
            ]
            .into_iter()
            .collect(),
        }
    }
    fn sample_value(&self, sample: &Self::Sample) -> crate::activity::ActivitySampleValue {
        crate::activity::ActivitySampleValue::RepairCost(*sample)
    }
    fn validate_sample(
        &self,
        _input: &ValidatedInput,
        sample: &Self::Sample,
    ) -> Result<(), SimulationError> {
        let ActivityConfiguration::RepairCost(config) = &self.instance.configuration else {
            unreachable!()
        };
        property_simulation_models::RepairCostDistribution {
            maximum: config.maximum,
        }
        .validate_sample(sample.0)
        .map_err(|_| SimulationError::InvalidInput("repair cost sample outside support"))
    }
    fn propose_validated(
        &self,
        _state: &OpeningState,
        input: &FrozenInput,
        sample: &ValidatedActivitySample<'_, Self::Sample>,
    ) -> Result<SliceProposal, SimulationError> {
        let ActivityConfiguration::RepairCost(config) = &self.instance.configuration else {
            unreachable!()
        };
        let amount = sample.value().0.rounded_for_posting()?;
        let condition = AssetCondition::Available;
        Ok(SliceProposal {
            events: vec![EventProposal {
                period: input.period,
                key: "repair_cost_settled".into(),
            }],
            effects: vec![EffectProposal {
                period: input.period,
                target: property_simulation_domain::ResourceKey::new("property.condition")
                    .expect("static resource key"),
                key: "condition".into(),
                source_event_key: Some("repair_cost_settled".into()),
                condition,
            }],
            transactions: vec![TransactionProposal {
                key: "repair_cost".into(),
                period: input.period,
                entries: vec![
                    LedgerEntry::new(config.expense_account, BalanceSide::Debit, amount),
                    LedgerEntry::new(input.cash_account, BalanceSide::Credit, amount)
                        .with_cash_flow_category(CashFlowCategory::Operating),
                ],
                source_event_key: Some("repair_cost_settled".into()),
            }],
        })
    }
}
