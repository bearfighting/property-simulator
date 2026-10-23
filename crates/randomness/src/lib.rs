//! Ephemeral OS entropy, run-key derivation, and semantic ChaCha12 streams.

mod error;
mod identity;
mod root_key;
mod run_key_factory;
mod stream;
mod streams;

#[cfg(test)]
mod tests;

pub use error::RandomnessError;
pub use identity::{RandomCategory, SemanticIdentity};
pub use root_key::RootKey;
pub use run_key_factory::RunKeyFactory;
pub use stream::RandomStream;
pub use streams::RandomStreams;
