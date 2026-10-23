use blake3::Hasher;
use property_simulation_domain::TrialId;

use crate::{RandomStreams, RootKey};

pub struct RunKeyFactory {
    root: RootKey,
}

impl RunKeyFactory {
    pub fn new(root: RootKey) -> Self {
        Self { root }
    }

    pub fn run_streams(&self, trial: TrialId) -> RandomStreams {
        let mut hasher = Hasher::new_keyed(self.root.as_bytes());
        hasher.update(b"property-simulation/run-key\0");
        hasher.update(trial.as_bytes());
        RandomStreams::new(*hasher.finalize().as_bytes(), trial)
    }
}
