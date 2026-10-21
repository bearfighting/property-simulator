//! Ephemeral OS entropy, run-key derivation, and semantic ChaCha12 streams.

use blake3::Hasher;
use property_simulation_domain::{ActivityInstanceId, SliceId, TrialId, YearMonth};
use property_simulation_models::RandomSource;
use rand_chacha::ChaCha12Rng;
use rand_core::{RngCore, SeedableRng};
use std::sync::atomic::{AtomicU64, Ordering};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomnessError {
    EntropyUnavailable,
    RunSequenceExhausted,
    TrialIdentityMismatch,
}

/// Root key is deliberately non-serializable and is kept only in memory.
pub struct RootKey([u8; 32]);

impl RootKey {
    pub fn from_os() -> Result<Self, RandomnessError> {
        Self::from_entropy(getrandom::getrandom)
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self(seed)
    }

    pub fn from_entropy<E>(
        read: impl FnOnce(&mut [u8]) -> Result<(), E>,
    ) -> Result<Self, RandomnessError>
    where
        E: std::fmt::Debug,
    {
        let mut key = [0; 32];
        read(&mut key).map_err(|_| RandomnessError::EntropyUnavailable)?;
        Ok(Self(key))
    }
}

pub struct RunKeyFactory {
    root: RootKey,
    sequence: AtomicU64,
}

impl RunKeyFactory {
    pub fn new(root: RootKey) -> Self {
        Self {
            root,
            sequence: AtomicU64::new(0),
        }
    }

    pub fn run_streams(&self, trial: TrialId) -> Result<RandomStreams, RandomnessError> {
        let sequence = self
            .sequence
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| RandomnessError::RunSequenceExhausted)?;
        let mut hasher = Hasher::new_keyed(&self.root.0);
        hasher.update(b"property-simulation/run-key\0");
        hasher.update(trial.as_bytes());
        hasher.update(&sequence.to_le_bytes());
        Ok(RandomStreams {
            run_key: *hasher.finalize().as_bytes(),
            trial_id: trial,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RandomCategory {
    Environment = 1,
    Property = 2,
    PolicyMacro = 3,
    OccupancyOperations = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticIdentity {
    pub trial_id: TrialId,
    pub category: RandomCategory,
    pub slice_id: SliceId,
    pub activity_id: ActivityInstanceId,
    pub period: YearMonth,
    pub purpose: String,
}

pub struct RandomStreams {
    run_key: [u8; 32],
    trial_id: TrialId,
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
        Ok(RandomStream(ChaCha12Rng::from_seed(
            *hasher.finalize().as_bytes(),
        )))
    }
}

pub struct RandomStream(ChaCha12Rng);

impl RandomSource for RandomStream {
    type Error = std::convert::Infallible;

    fn next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.0.next_u64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> SemanticIdentity {
        SemanticIdentity {
            trial_id: TrialId::from_bytes([1; 16]),
            category: RandomCategory::Property,
            slice_id: SliceId::from_bytes([2; 16]),
            activity_id: ActivityInstanceId::from_bytes([3; 16]),
            period: YearMonth::new(2026, 1).unwrap(),
            purpose: "repair-cost".into(),
        }
    }

    fn draws(streams: &RandomStreams, id: &SemanticIdentity) -> [u64; 3] {
        let mut stream = streams.stream(id).unwrap();
        std::array::from_fn(|_| stream.next_u64().unwrap())
    }

    #[test]
    fn entropy_success_and_failure_are_explicit() {
        assert_eq!(
            RootKey::from_entropy(|out| {
                out.fill(9);
                Ok::<_, ()>(())
            })
            .unwrap()
            .0,
            [9; 32]
        );
        assert!(RootKey::from_entropy(|_| Err(())).is_err());
    }

    #[test]
    fn run_keys_change_with_trial_and_process_sequence() {
        let root = [8; 32];
        let factory = RunKeyFactory::new(RootKey::from_seed(root));
        let a = factory.run_streams(TrialId::from_bytes([1; 16])).unwrap();
        let b = factory.run_streams(TrialId::from_bytes([1; 16])).unwrap();
        let id = identity();
        assert_ne!(draws(&a, &id), draws(&b, &id));

        let different_trial = RunKeyFactory::new(RootKey::from_seed(root))
            .run_streams(TrialId::from_bytes([2; 16]))
            .unwrap();
        let mut different_trial_identity = id.clone();
        different_trial_identity.trial_id = TrialId::from_bytes([2; 16]);
        assert_ne!(
            draws(&a, &id),
            draws(&different_trial, &different_trial_identity)
        );
    }

    #[test]
    fn sequence_exhaustion_is_an_error_without_wrapping() {
        let factory = RunKeyFactory::new(RootKey::from_seed([3; 32]));
        factory.sequence.store(u64::MAX, Ordering::Relaxed);
        assert!(factory.run_streams(TrialId::from_bytes([1; 16])).is_err());
        assert_eq!(factory.sequence.load(Ordering::Relaxed), u64::MAX);
    }

    #[test]
    fn semantic_fields_and_independent_consumption_are_isolated() {
        let streams = RunKeyFactory::new(RootKey::from_seed([7; 32]))
            .run_streams(TrialId::from_bytes([1; 16]))
            .unwrap();
        let base = identity();
        let expected = draws(&streams, &base);
        let mut unrelated = streams
            .stream(&SemanticIdentity {
                purpose: "other".into(),
                ..base.clone()
            })
            .unwrap();
        for _ in 0..100 {
            let _ = unrelated.next_u64().unwrap();
        }
        assert_eq!(draws(&streams, &base), expected);
        let mut changed = base.clone();
        changed.period = changed.period.checked_add_months(1).unwrap();
        assert_ne!(draws(&streams, &changed), expected);
        changed = base.clone();
        changed.activity_id = ActivityInstanceId::from_bytes([5; 16]);
        assert_ne!(draws(&streams, &changed), expected);
        changed = base.clone();
        changed.slice_id = SliceId::from_bytes([6; 16]);
        assert_ne!(draws(&streams, &changed), expected);
        changed = base.clone();
        changed.category = RandomCategory::Environment;
        assert_ne!(draws(&streams, &changed), expected);
        changed = base.clone();
        changed.period = YearMonth::new(2027, 1).unwrap();
        assert_ne!(draws(&streams, &changed), expected);
        changed = base.clone();
        changed.purpose = "repair-occurrence".into();
        assert_ne!(draws(&streams, &changed), expected);
    }

    #[test]
    fn semantic_stream_rejects_a_different_trial_identity() {
        let streams = RunKeyFactory::new(RootKey::from_seed([4; 32]))
            .run_streams(TrialId::from_bytes([1; 16]))
            .unwrap();
        let mut foreign = identity();
        foreign.trial_id = TrialId::from_bytes([2; 16]);
        assert!(matches!(
            streams.stream(&foreign),
            Err(RandomnessError::TrialIdentityMismatch)
        ));
    }
}
