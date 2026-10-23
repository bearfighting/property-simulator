use blake3::Hasher;
use rand_chacha::ChaCha12Rng;
use rand_core::{RngCore, SeedableRng};
use std::convert::Infallible;

use crate::{RandomInput, RandomRootKey, RandomStreamIdentity};

/// Derive an independent pseudorandom stream from an explicit simulation root
/// and semantic identity.
///
/// The same root and identity produce the same stream. Callers must provide
/// distinct canonical identities for random inputs that should be isolated.
pub fn derive_stream(root_key: &RandomRootKey, identity: &RandomStreamIdentity) -> RandomStream {
    let mut hasher = Hasher::new_keyed(root_key.as_bytes());
    hasher.update(b"property-simulation/random-stream/v1\0");
    hasher.update(&(identity.as_bytes().len() as u64).to_le_bytes());
    hasher.update(identity.as_bytes());

    RandomStream(ChaCha12Rng::from_seed(*hasher.finalize().as_bytes()))
}

/// Stateful cursor over one derived random stream.
pub struct RandomStream(ChaCha12Rng);

impl RandomInput for RandomStream {
    type Error = Infallible;

    fn next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.0.next_u64())
    }
}
