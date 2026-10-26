//! In-memory lifecycle rules for coordinating one simulation run.
//!
//! This crate has no dependency on Application, Core, Presentation, or storage.
//! A composition layer can hold the current [`SimulationState`], select its
//! associated [`WorkUnit`], and advance through the linear lifecycle.

mod lifecycle;

pub use lifecycle::{SimulationState, WorkUnit, new};
