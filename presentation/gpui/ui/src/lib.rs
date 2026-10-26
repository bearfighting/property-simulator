//! GPUI presentation for simulation workflows.
//!
//! The crate exposes plain UI contracts and a window entrypoint. Application
//! and persistence dependencies stay in the desktop composition root.

mod app;
mod components;
mod contract;
mod pages;

pub use app::run_initialization_ui;
pub use contract::{InitializationFormData, InitializationHandler, SubmissionResult};
