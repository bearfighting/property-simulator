use std::collections::BTreeSet;

use property_simulation_accounting::{BalanceSide, CashFlowCategory, LedgerEntry};

use crate::activity::{SimulationSlice, ValidatedActivitySample};
use crate::composition::SliceResources;
use crate::error::SimulationError;
use crate::types::{
    ActivityConfiguration, ActivityInstance, ActivitySampleKind, EventProposal, FrozenInput,
    OpeningState, PropertyTaxConfig, SliceProposal, TransactionProposal,
};
use crate::validation::ValidatedInput;

pub struct PropertyTaxSlice {
    pub instance: ActivityInstance,
}

impl SimulationSlice for PropertyTaxSlice {
    type Sample = ();
    const SAMPLE_KIND: ActivitySampleKind = ActivitySampleKind::PropertyTax;
    fn instance(&self) -> &ActivityInstance {
        &self.instance
    }
    fn resources(&self, _input: &FrozenInput) -> SliceResources {
        let writes = match &self.instance.configuration {
            ActivityConfiguration::PropertyTax(config) => {
                [config.cash_account, config.expense_account]
                    .into_iter()
                    .map(property_simulation_domain::ResourceKey::for_account)
                    .collect()
            }
            _ => BTreeSet::new(),
        };
        SliceResources {
            instance_id: self.instance.id,
            slice_id: self.instance.slice_id,
            reads: BTreeSet::new(),
            writes,
        }
    }
    fn sample_value(&self, _sample: &()) -> crate::activity::ActivitySampleValue {
        crate::activity::ActivitySampleValue::PropertyTax
    }
    fn validate_sample(&self, input: &ValidatedInput, _sample: &()) -> Result<(), SimulationError> {
        config(&self.instance)?;
        if !input.has_preflighted_activity(&self.instance) {
            return Err(SimulationError::InvalidInput(
                "property-tax activity set must be preflighted before transition",
            ));
        }
        Ok(())
    }
    fn propose_validated(
        &self,
        _state: &OpeningState,
        input: &FrozenInput,
        _sample: &ValidatedActivitySample<'_, ()>,
    ) -> Result<SliceProposal, SimulationError> {
        let config = config(&self.instance)?;
        if !config.is_due(input.period) {
            return Ok(SliceProposal::default());
        }
        let amount = config.amount.rounded_for_posting()?;
        if amount.is_zero() {
            return Ok(SliceProposal::default());
        }
        let event_key = "property_tax.payment";
        Ok(SliceProposal {
            events: vec![EventProposal {
                period: input.period,
                key: event_key.into(),
            }],
            transactions: vec![TransactionProposal {
                key: "payment".into(),
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

fn config(instance: &ActivityInstance) -> Result<&PropertyTaxConfig, SimulationError> {
    match &instance.configuration {
        ActivityConfiguration::PropertyTax(config) => Ok(config),
        _ => Err(SimulationError::InvalidInput(
            "property tax Slice requires property tax configuration",
        )),
    }
}
