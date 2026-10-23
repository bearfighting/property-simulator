use std::collections::BTreeSet;

use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};

use crate::activity::{SimulationSlice, ValidatedActivitySample};
use crate::composition::SliceResources;
use crate::error::SimulationError;
use crate::types::{
    ActivityConfiguration, ActivityInstance, EventProposal, FrozenInput, OpeningState,
    SliceProposal, TransactionProposal, UtilityConfig,
};
use crate::validation::ValidatedInput;

pub struct UtilitySlice {
    pub instance: ActivityInstance,
}

impl SimulationSlice for UtilitySlice {
    type Sample = u64;

    fn instance(&self) -> &ActivityInstance {
        &self.instance
    }

    fn resources(&self, _input: &FrozenInput) -> SliceResources {
        let writes = match &self.instance.configuration {
            ActivityConfiguration::Utility(config) => [config.cash_account, config.expense_account]
                .into_iter()
                .map(property_simulation_domain::ResourceKey::for_account)
                .collect(),
            _ => BTreeSet::new(),
        };
        SliceResources {
            instance_id: self.instance.id,
            slice_id: self.instance.slice_id,
            reads: BTreeSet::new(),
            writes,
        }
    }

    fn sample_value(&self, sample: &u64) -> crate::activity::ActivitySampleValue {
        crate::activity::ActivitySampleValue::Utility(*sample)
    }

    fn validate_sample(&self, input: &ValidatedInput, sample: &u64) -> Result<(), SimulationError> {
        let config = config(&self.instance)?;
        if !input.has_preflighted_activity(&self.instance) {
            return Err(SimulationError::InvalidInput(
                "utility activity set must be preflighted before sampling",
            ));
        }
        if *sample > config.maximum_usage {
            return Err(SimulationError::InvalidInput(
                "utility usage sample exceeds configured support",
            ));
        }
        Ok(())
    }

    fn propose_validated(
        &self,
        _state: &OpeningState,
        input: &FrozenInput,
        sample: &ValidatedActivitySample<'_, u64>,
    ) -> Result<SliceProposal, SimulationError> {
        let config = config(&self.instance)?;
        let amount = config
            .rate_per_unit
            .checked_mul_u64(*sample.value())?
            .rounded_for_posting()?;
        if amount.is_zero() {
            return Ok(SliceProposal::default());
        }
        let event_key = "utility.usage_charged";
        Ok(SliceProposal {
            events: vec![EventProposal {
                period: input.period,
                key: event_key.into(),
            }],
            transactions: vec![TransactionProposal {
                key: "usage-charge".into(),
                period: input.period,
                entries: vec![
                    LedgerEntry::new(config.expense_account, BalanceSide::Debit, amount),
                    LedgerEntry::new(config.cash_account, BalanceSide::Credit, amount)
                        .with_cash_flow_category(CashFlowCategory::Operating),
                ],
                source_event_key: Some(event_key.into()),
            }],
            ..Default::default()
        })
    }
}

fn config(instance: &ActivityInstance) -> Result<&UtilityConfig, SimulationError> {
    match &instance.configuration {
        ActivityConfiguration::Utility(config) => Ok(config),
        _ => Err(SimulationError::InvalidInput(
            "utility Slice requires utility configuration",
        )),
    }
}
