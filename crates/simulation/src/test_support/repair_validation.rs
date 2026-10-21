use property_simulation_domain::{DomainError, Money};
use property_simulation_models::{RepairCostDistribution, RepairDistribution};

pub struct RepairCostRules;

impl crate::RepairCostValidation for RepairCostRules {
    fn validate_distribution(&self, distribution: &RepairDistribution) -> Result<(), DomainError> {
        distribution.validate_distribution()
    }

    fn validate_support(&self, distribution: &RepairCostDistribution) -> Result<(), DomainError> {
        distribution.validate_support()
    }

    fn maximum_minor_units(
        &self,
        distribution: &RepairCostDistribution,
    ) -> Result<u64, DomainError> {
        distribution.maximum_minor_units()
    }

    fn validate_sample(
        &self,
        distribution: &RepairCostDistribution,
        value: Money,
    ) -> Result<(), DomainError> {
        distribution.validate_sample(value)
    }
}

pub static REPAIR_COST_RULES: RepairCostRules = RepairCostRules;
