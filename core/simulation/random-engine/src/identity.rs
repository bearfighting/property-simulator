/// A caller-defined, canonical byte encoding of a semantic random stream
/// identity, such as Slice, activity instance, period, and purpose.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RandomStreamIdentity(Vec<u8>);

impl RandomStreamIdentity {
    /// Construct an identity from its canonical byte representation.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Return the canonical identity bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
