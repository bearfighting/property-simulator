/// A caller-owned source of random words. Stream allocation and seeding are supplied separately.
pub trait RandomSource {
    type Error;
    fn next_u64(&mut self) -> Result<u64, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingError<E> {
    InvalidDistribution,
    RandomSource(E),
}

/// A distribution declares the sample type it produces.
pub trait Distribution {
    type Sample;
    fn validate(&self) -> bool;
}

/// A replaceable sampling strategy that consumes caller-provided randomness.
pub trait Sampler<D: Distribution> {
    fn sample<R: RandomSource>(
        &mut self,
        distribution: &D,
        random: &mut R,
    ) -> Result<D::Sample, SamplingError<R::Error>>;
}

/// Draw a uniform integer in `0..=maximum` without modulo bias.
pub(crate) fn uniform_inclusive<R: RandomSource>(
    random: &mut R,
    maximum: u64,
) -> Result<u64, SamplingError<R::Error>> {
    let range = maximum as u128 + 1;
    let zone = (1_u128 << 64) / range * range;
    loop {
        let value = random.next_u64().map_err(SamplingError::RandomSource)? as u128;
        if value < zone {
            return Ok((value % range) as u64);
        }
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
    fn uniform_sampler_rejects_the_biased_tail_and_retries() {
        let mut words = Words(vec![u64::MAX, 11]);

        assert_eq!(uniform_inclusive(&mut words, 5), Ok(5));
        assert!(words.0.is_empty());
    }

    #[test]
    fn uniform_sampler_covers_the_full_u64_domain() {
        let mut words = Words(vec![u64::MAX]);

        assert_eq!(uniform_inclusive(&mut words, u64::MAX), Ok(u64::MAX));
        assert!(words.0.is_empty());
    }

    #[test]
    fn uniform_sampler_propagates_random_source_errors() {
        assert_eq!(
            uniform_inclusive(&mut Words(vec![]), 9),
            Err(SamplingError::RandomSource("no random words"))
        );
    }
}
