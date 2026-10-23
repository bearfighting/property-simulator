use crate::RandomnessError;

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

impl RootKey {
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
