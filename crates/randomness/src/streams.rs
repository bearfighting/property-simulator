use blake3::Hasher;
use property_simulation_domain::TrialId;

use crate::{RandomStream, RandomnessError, SemanticIdentity};

pub struct RandomStreams {
    run_key: [u8; 32],
    trial_id: TrialId,
}

impl RandomStreams {
    pub(crate) const fn new(run_key: [u8; 32], trial_id: TrialId) -> Self {
        Self { run_key, trial_id }
    }
}

impl RandomStreams {
    pub fn stream(&self, identity: &SemanticIdentity) -> Result<RandomStream, RandomnessError> {
        if identity.trial_id != self.trial_id {
            return Err(RandomnessError::TrialIdentityMismatch);
        }
        let mut hasher = Hasher::new_keyed(&self.run_key);
        hasher.update(b"property-simulation/semantic-stream\0");
        hasher.update(identity.trial_id.as_bytes());
        hasher.update(&[identity.category as u8]);
        hasher.update(identity.slice_id.as_bytes());
        hasher.update(identity.activity_id.as_bytes());
        hasher.update(&identity.period.year().to_le_bytes());
        hasher.update(&[identity.period.month()]);
        hasher.update(&(identity.purpose.len() as u64).to_le_bytes());
        hasher.update(identity.purpose.as_bytes());
        Ok(RandomStream::from_seed(*hasher.finalize().as_bytes()))
    }
}
