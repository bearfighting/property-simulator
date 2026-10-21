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
