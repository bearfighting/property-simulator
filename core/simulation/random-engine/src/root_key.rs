/// Root key created and persisted by the caller for one simulation.
#[derive(Clone, PartialEq, Eq)]
pub struct RandomRootKey([u8; 32]);

impl RandomRootKey {
    /// Construct a root key from bytes obtained by the caller.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return the bytes for caller-managed persistence of this root.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
