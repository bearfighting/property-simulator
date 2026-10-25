//! Storage-neutral DTOs shared between Application and Data Access adapters.

pub mod dto;
mod error;
mod persistence;

pub use error::{DataAccessError, DtoError};
pub use persistence::{DataAccess, MonthlyFactsStore, SettingsStore, SimulationStore};
