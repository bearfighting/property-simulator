use crate::sampling::{RandomSource, Sampler, SamplingError, uniform_inclusive};

/// Uniform integer utility usage in the inclusive range `0..=maximum`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtilityUsageDistribution {
    pub maximum: u64,
}

impl super::Distribution for UtilityUsageDistribution {
    type Sample = u64;

    fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Default)]
pub struct UtilityUsageSampler;

impl Sampler<UtilityUsageDistribution> for UtilityUsageSampler {
    fn sample<R: RandomSource>(
        &mut self,
        distribution: &UtilityUsageDistribution,
        random: &mut R,
    ) -> Result<u64, SamplingError<R::Error>> {
        uniform_inclusive(random, distribution.maximum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Words(Vec<u64>);
    impl RandomSource for Words {
        type Error = &'static str;
        fn next_u64(&mut self) -> Result<u64, Self::Error> {
            if self.0.is_empty() {
                Err("no random words")
            } else {
                Ok(self.0.remove(0))
            }
        }
    }

    #[test]
    fn samples_zero_and_inclusive_maximum_without_modulo_bias() {
        let distribution = UtilityUsageDistribution { maximum: 7 };
        let mut sampler = UtilityUsageSampler;
        assert_eq!(sampler.sample(&distribution, &mut Words(vec![0])), Ok(0));
        assert_eq!(sampler.sample(&distribution, &mut Words(vec![7])), Ok(7));
    }

    #[test]
    fn zero_maximum_always_samples_zero() {
        let mut sampler = UtilityUsageSampler;
        assert_eq!(
            sampler.sample(
                &UtilityUsageDistribution { maximum: 0 },
                &mut Words(vec![u64::MAX]),
            ),
            Ok(0)
        );
    }

    #[test]
    fn sampler_propagates_random_source_errors() {
        let mut sampler = UtilityUsageSampler;
        let mut random = Words(vec![]);

        assert_eq!(
            sampler.sample(&UtilityUsageDistribution { maximum: 9 }, &mut random),
            Err(SamplingError::RandomSource("no random words"))
        );
    }
}
