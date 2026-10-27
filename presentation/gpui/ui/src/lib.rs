//! GPUI presentation for simulation workflows.
//!
//! The crate exposes plain UI contracts and a window entrypoint. Application
//! and persistence dependencies stay in the desktop composition root.

mod app;
mod brand;
mod components;
mod contract;
mod pages;

pub use app::run_simulation_ui;
pub use contract::{
    CashFlowChartSlice, CashFlowViewCategory, InitializationFormData, InitializationHandler,
    MonthlySimulationHandler, MonthlySimulationResponse, MonthlySimulationViewData,
    MonthlyTimelineHandler, ReportAmountView, ReportCashFlowAccountView,
    ReportCashFlowCategoryView, ReportPeriodView, ReportingHandler, ReportingViewData,
    ResetSimulationHandler, StopSimulationHandler, SubmissionResult, TimelinePageDirection,
};
pub(crate) use contract::{MONTHS_PER_TIMELINE_PAGE, PageAction};
