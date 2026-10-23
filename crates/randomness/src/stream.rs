use property_simulation_models::RandomSource;
use rand_chacha::ChaCha12Rng;
use rand_core::{RngCore, SeedableRng};

pub struct RandomStream(ChaCha12Rng);

impl RandomStream {
    pub(crate) fn from_seed(seed: [u8; 32]) -> Self {
        Self(ChaCha12Rng::from_seed(seed))
    }
}

impl RandomSource for RandomStream {
    type Error = std::convert::Infallible;

    fn next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.0.next_u64())
    }
}
