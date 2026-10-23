use crate::sampling::{RandomSource, Sampler, SamplingError, uniform_inclusive};
use property_simulation_domain::{DomainError, Money};

/// Conditional monthly repair occurrence probability, represented in basis
/// points (1/10,000). Age is the completed month difference from the latest
/// known repair baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairOccurrenceDistribution {
    pub age_months: u64,
}

impl RepairOccurrenceDistribution {
    pub const fn numerator(self) -> u64 {
        (25 + (125_u128 * self.age_months as u128) / (self.age_months as u128 + 60)) as u64
    }
    pub const fn denominator(self) -> u64 {
        10_000
    }
    pub const fn validate(self) -> bool {
        self.numerator() < self.denominator()
    }
}
impl super::Distribution for RepairOccurrenceDistribution {
    type Sample = RepairOccurrenceSample;
    fn validate(&self) -> bool {
        (*self).validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairOccurrenceSample(pub bool);

#[derive(Debug, Default)]
pub struct RepairOccurrenceSampler;

impl Sampler<RepairOccurrenceDistribution> for RepairOccurrenceSampler {
    fn sample<R: RandomSource>(
        &mut self,
        distribution: &RepairOccurrenceDistribution,
        random: &mut R,
    ) -> Result<RepairOccurrenceSample, SamplingError<R::Error>> {
        if !distribution.validate() {
            return Err(SamplingError::InvalidDistribution);
        }
        Ok(RepairOccurrenceSample(
            uniform_inclusive(random, distribution.denominator() - 1)? < distribution.numerator(),
        ))
    }
}

/// A repair amount is sampled independently, only after an occurrence has
/// generated a cost instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairCostSample(pub Money);

#[derive(Debug, Default)]
pub struct RepairCostSampler;

impl Sampler<RepairCostDistribution> for RepairCostSampler {
    fn sample<R: RandomSource>(
        &mut self,
        distribution: &RepairCostDistribution,
        random: &mut R,
    ) -> Result<RepairCostSample, SamplingError<R::Error>> {
        distribution
            .validate_support()
            .map_err(|_| SamplingError::InvalidDistribution)?;
        let units = uniform_inclusive(
            random,
            distribution
                .maximum_minor_units()
                .map_err(|_| SamplingError::InvalidDistribution)?,
        )?;
        let amount = distribution
            .money_for_minor_units(units)
            .and_then(Money::rounded_for_posting)
            .map_err(|_| SamplingError::InvalidDistribution)?;
        Ok(RepairCostSample(amount))
    }
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
impl super::Distribution for RepairCostDistribution {
    type Sample = RepairCostSample;
    fn validate(&self) -> bool {
        self.validate_support().is_ok() && self.maximum_minor_units().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occurrence_probability_matches_basis_point_curve_without_overflow() {
        assert_eq!(
            RepairOccurrenceDistribution { age_months: 0 }.numerator(),
            25
        );
        assert_eq!(
            RepairOccurrenceDistribution { age_months: 1 }.numerator(),
            27
        );
        assert_eq!(
            RepairOccurrenceDistribution { age_months: 60 }.numerator(),
            87
        );
        assert_eq!(
            RepairOccurrenceDistribution {
                age_months: u64::MAX
            }
            .numerator(),
            149
        );
        let values = [0, 1, 60, 120, 1_000, u64::MAX]
            .map(|age_months| RepairOccurrenceDistribution { age_months }.numerator());
        assert!(values.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(values.into_iter().all(|rate| rate < 10_000));
    }

    #[test]
    fn occurrence_sampler_draws_only_the_occurrence_decision() {
        let distribution = RepairOccurrenceDistribution { age_months: 0 };
        let mut sampler = RepairOccurrenceSampler;
        let mut no_occurrence = Words(vec![25]);
        assert_eq!(
            sampler.sample(&distribution, &mut no_occurrence).unwrap(),
            RepairOccurrenceSample(false)
        );
        assert!(no_occurrence.0.is_empty());
        let mut occurrence = Words(vec![24]);
        assert_eq!(
            sampler.sample(&distribution, &mut occurrence).unwrap(),
            RepairOccurrenceSample(true)
        );
        assert!(occurrence.0.is_empty());
    }

    #[test]
    fn cost_sampler_is_independent_and_includes_zero_and_limit() {
        let cad = property_simulation_domain::Currency::from_code("CAD").unwrap();
        let distribution = RepairCostDistribution {
            maximum: Money::parse("1.23", cad).unwrap(),
        };
        let mut sampler = RepairCostSampler;
        assert_eq!(
            sampler
                .sample(&distribution, &mut Words(vec![0]))
                .unwrap()
                .0,
            Money::parse("0", cad).unwrap()
        );
        assert_eq!(
            sampler
                .sample(&distribution, &mut Words(vec![123]))
                .unwrap()
                .0,
            distribution.maximum
        );
    }
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
}
