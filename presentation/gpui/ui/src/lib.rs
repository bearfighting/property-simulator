//! GPUI presentation for simulation workflows.
//!
//! The crate exposes plain UI contracts and a window entrypoint. Application
//! and persistence dependencies stay in the desktop composition root.

mod app;
mod components;
mod contract;
mod pages;

pub use app::run_simulation_ui;
pub(crate) use contract::PageAction;
pub use contract::{
    CashFlowChartSlice, CashFlowViewCategory, InitializationFormData, InitializationHandler,
    MonthlySimulationHandler, MonthlySimulationResponse, MonthlySimulationViewData,
    ReportAmountView, ReportCashFlowAccountView, ReportCashFlowCategoryView, ReportPeriodView,
    ReportingHandler, ReportingViewData, ResetSimulationHandler, StopSimulationHandler,
    SubmissionResult,
};
