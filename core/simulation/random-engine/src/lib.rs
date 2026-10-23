//! Random input derivation for simulation calculations.

mod identity;
mod input;
mod root_key;
mod stream;

pub use identity::RandomStreamIdentity;
pub use input::RandomInput;
pub use root_key::RandomRootKey;
pub use stream::{RandomStream, derive_stream};
