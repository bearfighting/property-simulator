//! Initialize one simulation and persist its initial facts.

mod execution;
mod opening_entry;
mod settings;
mod types;

pub use execution::initialize_simulation;
pub use types::{
    ConfiguredSlice, InitialSliceSettings, InitializeSimulationError, InitializeSimulationRequest,
    OpeningBalance,
};
