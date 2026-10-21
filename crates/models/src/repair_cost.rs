use crate::sampling::{RandomSource, Sampler, SamplingError, uniform_inclusive};
use property_simulation_domain::{DomainError, Money};

/// Typed outcome shared with the simulation crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairSample {
    NoRepair,
    Repair { cost: Money },
}

/// A bounded discrete repair-cost distribution. The inclusive upper bound makes
/// every supported sample safe to validate before a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairCostDistribution {
    pub maximum: Money,
}

impl RepairCostDistribution {
    pub fn validate_support(&self) -> Result<(), DomainError> {
        if self.maximum.is_negative() {
            return Err(DomainError::InvalidMoneyAmount);
        }
        Ok(())
    }

    pub fn validate_sample(&self, value: Money) -> Result<(), DomainError> {
        self.validate_support()?;
        let rounded = value.rounded_for_posting()?;
        let maximum_supported = self.money_for_minor_units(self.maximum_minor_units()?)?;
        if value.is_negative()
            || value != rounded
            || value.currency() != self.maximum.currency()
            || value.checked_cmp(maximum_supported)? == std::cmp::Ordering::Greater
        {
            return Err(DomainError::InvalidMoneyAmount);
        }
        Ok(())
    }

    /// Largest supported integer number of currency minor units. A fractional
    /// minor-unit tail is intentionally discarded, so sampling never exceeds maximum.
    pub fn maximum_minor_units(&self) -> Result<u64, DomainError> {
        self.validate_support()?;
        let digits = self.maximum.currency().minor_unit_digits() as u8;
        let scaled = if self.maximum.scale() <= digits {
            self.maximum
                .coefficient()
                .checked_mul(
                    10_i128
                        .checked_pow((digits - self.maximum.scale()) as u32)
                        .ok_or(DomainError::ArithmeticOverflow)?,
                )
                .ok_or(DomainError::ArithmeticOverflow)?
        } else {
            self.maximum.coefficient()
                / 10_i128
                    .checked_pow((self.maximum.scale() - digits) as u32)
                    .ok_or(DomainError::ArithmeticOverflow)?
        };
        u64::try_from(scaled).map_err(|_| DomainError::MoneyOutOfRange)
    }

    fn money_for_minor_units(&self, units: u64) -> Result<Money, DomainError> {
        Money::from_parts(
            units as i128,
            self.maximum.currency().minor_unit_digits() as u8,
            self.maximum.currency(),
        )
    }
}

/// Repair occurs with an exact rational probability, then cost is uniform over
/// every minor-unit amount from zero through the floored configured maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairDistribution {
    pub occurrence_numerator: u64,
    pub occurrence_denominator: u64,
    pub cost: RepairCostDistribution,
}

impl RepairDistribution {
    pub fn validate_distribution(&self) -> Result<(), DomainError> {
        if self.occurrence_denominator == 0
            || self.occurrence_numerator > self.occurrence_denominator
        {
            return Err(DomainError::InvalidMoneyAmount);
        }
        self.cost.validate_support()?;
        self.cost.maximum_minor_units()?;
        Ok(())
    }
}

impl super::Distribution for RepairDistribution {
    type Sample = RepairSample;
    fn validate(&self) -> bool {
        self.validate_distribution().is_ok()
    }
}

#[derive(Debug, Default)]
pub struct RepairSampler;

impl Sampler<RepairDistribution> for RepairSampler {
    fn sample<R: RandomSource>(
        &mut self,
        distribution: &RepairDistribution,
        random: &mut R,
    ) -> Result<RepairSample, SamplingError<R::Error>> {
        distribution
            .validate_distribution()
            .map_err(|_| SamplingError::InvalidDistribution)?;
        let denominator = distribution.occurrence_denominator;
        let fire = if distribution.occurrence_numerator == 0 {
            false
        } else if distribution.occurrence_numerator == denominator {
            true
        } else {
            uniform_inclusive(random, denominator - 1)? < distribution.occurrence_numerator
        };
        if !fire {
            return Ok(RepairSample::NoRepair);
        }
        let maximum = distribution
            .cost
            .maximum_minor_units()
            .map_err(|_| SamplingError::InvalidDistribution)?;
        let units = uniform_inclusive(random, maximum)?;
        let cost = distribution
            .cost
            .money_for_minor_units(units)
            .map_err(|_| SamplingError::InvalidDistribution)?
            .rounded_for_posting()
            .map_err(|_| SamplingError::InvalidDistribution)?;
        Ok(RepairSample::Repair { cost })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_repair_support_includes_edges_and_rejects_outside() {
        let cad = property_simulation_domain::Currency::from_code("CAD").unwrap();
        let d = RepairCostDistribution {
            maximum: Money::parse("50", cad).unwrap(),
        };
        assert!(d.validate_sample(Money::parse("0", cad).unwrap()).is_ok());
        assert!(d.validate_sample(d.maximum).is_ok());
        assert!(
            d.validate_sample(Money::parse("50.01", cad).unwrap())
                .is_err()
        );
        let fractional_limit = RepairCostDistribution {
            maximum: Money::parse("1.239", cad).unwrap(),
        };
        assert!(
            fractional_limit
                .validate_sample(Money::parse("1.23", cad).unwrap())
                .is_ok()
        );
        assert!(
            fractional_limit
                .validate_sample(Money::parse("1.239", cad).unwrap())
                .is_err()
        );
        assert!(
            fractional_limit
                .validate_sample(Money::parse("1.005", cad).unwrap())
                .is_err()
        );
    }

    struct Words(Vec<u64>);
    impl RandomSource for Words {
        type Error = &'static str;
        fn next_u64(&mut self) -> Result<u64, Self::Error> {
            if self.0.is_empty() {
                Err("empty")
            } else {
                Ok(self.0.remove(0))
            }
        }
    }

    fn distribution(n: u64, d: u64, maximum: &str) -> RepairDistribution {
        let cad = property_simulation_domain::Currency::from_code("CAD").unwrap();
        RepairDistribution {
            occurrence_numerator: n,
            occurrence_denominator: d,
            cost: RepairCostDistribution {
                maximum: Money::parse(maximum, cad).unwrap(),
            },
        }
    }

    #[test]
    fn repair_distribution_validates_exact_probability_and_minor_unit_support() {
        assert!(distribution(0, 1, "1").validate_distribution().is_ok());
        assert!(distribution(1, 1, "1").validate_distribution().is_ok());
        assert!(distribution(2, 3, "1.239").validate_distribution().is_ok());
        assert!(
            distribution(2, 3, "0.009")
                .cost
                .maximum_minor_units()
                .unwrap()
                == 0
        );
        assert_eq!(
            distribution(1, 0, "1").validate_distribution(),
            Err(DomainError::InvalidMoneyAmount)
        );
        assert_eq!(
            distribution(2, 1, "1").validate_distribution(),
            Err(DomainError::InvalidMoneyAmount)
        );
    }

    #[test]
    fn repair_sampler_obeys_probability_cost_bounds_and_propagates_source_errors() {
        let mut sampler = RepairSampler;
        let mut no_repair = Words(vec![]);
        assert_eq!(
            sampler
                .sample(&distribution(0, 1, "1"), &mut no_repair)
                .unwrap(),
            RepairSample::NoRepair
        );
        let mut lower = Words(vec![0, 0]);
        assert_eq!(
            sampler
                .sample(&distribution(1, 2, "1.239"), &mut lower)
                .unwrap(),
            RepairSample::Repair {
                cost: Money::parse(
                    "0",
                    property_simulation_domain::Currency::from_code("CAD").unwrap()
                )
                .unwrap()
            }
        );
        let mut upper = Words(vec![0, 123]);
        assert_eq!(
            sampler
                .sample(&distribution(1, 2, "1.239"), &mut upper)
                .unwrap(),
            RepairSample::Repair {
                cost: Money::parse(
                    "1.23",
                    property_simulation_domain::Currency::from_code("CAD").unwrap()
                )
                .unwrap()
            }
        );
        let mut broken = Words(vec![]);
        assert_eq!(
            sampler.sample(&distribution(1, 2, "1"), &mut broken),
            Err(SamplingError::RandomSource("empty"))
        );
    }
}
