use gpui_kit::component::{ActiveTheme as _, TitleBar};
use gpui_kit::{App, Context, Entity, IntoElement, ParentElement as _, Styled as _, div};
use property_simulation_lifecycle::SimulationState;
use std::rc::Rc;

use crate::components::PageTheme;
use crate::pages::{
    InitializationPage, MonthlySimulationActions, MonthlySimulationPage, monthly_simulation_page,
    placeholder_page, reporting_page,
};
use crate::{MonthlySimulationViewData, PageAction, ReportingViewData, TimelinePageDirection};

use super::shell::AppShell;
use super::{navigation, page::AppPage, page_content, system_actions};

pub(super) struct ViewState {
    pub(super) page: AppPage,
    pub(super) state: SimulationState,
    pub(super) monthly: MonthlyViewState,
    pub(super) reporting: ReportingViewState,
}

pub(super) struct MonthlyViewState {
    pub(super) is_simulating: bool,
    pub(super) is_loading_timeline: bool,
    pub(super) timeline: Vec<MonthlySimulationViewData>,
    pub(super) selected_month: Option<u16>,
    pub(super) timeline_page: usize,
    pub(super) timeline_error: Option<String>,
    pub(super) error: Option<String>,
}

pub(super) struct ReportingViewState {
    pub(super) data: Option<ReportingViewData>,
    pub(super) is_loading: bool,
    pub(super) error: Option<String>,
}

/// Composes the persistent window layout from the selected route and content.
/// It does not own navigation or lifecycle state.
pub(super) fn render(
    view: ViewState,
    initialization: Entity<InitializationPage>,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let theme = PageTheme {
        border: cx.theme().border,
        muted_foreground: cx.theme().muted_foreground,
        panel: cx.theme().popover,
        danger_foreground: cx.theme().danger_foreground,
        chart_colors: [cx.theme().chart_1, cx.theme().chart_2, cx.theme().chart_3],
    };
    let page_content = if view.page == AppPage::Initialization {
        div().child(initialization)
    } else if view.page == AppPage::MonthlySimulation && view.state != SimulationState::Initial {
        let shell = cx.entity().downgrade();
        let on_simulate: PageAction = Rc::new(move |cx: &mut App| {
            let _ = shell.update(cx, |shell, cx| shell.simulate_month(cx));
        });
        let shell = cx.entity().downgrade();
        let on_reporting: PageAction = Rc::new(move |cx: &mut App| {
            let _ = shell.update(cx, |shell, cx| shell.navigate_to_reporting(cx));
        });
        let shell = cx.entity().downgrade();
        let on_select_month = Rc::new(move |month: u16, cx: &mut App| {
            let _ = shell.update(cx, |shell, cx| shell.select_month(month, cx));
        });
        let shell = cx.entity().downgrade();
        let on_reload_timeline: PageAction = Rc::new(move |cx: &mut App| {
            let _ = shell.update(cx, |shell, cx| shell.load_timeline(cx));
        });
        let shell = cx.entity().downgrade();
        let on_change_timeline_page =
            Rc::new(move |direction: TimelinePageDirection, cx: &mut App| {
                let _ = shell.update(cx, |shell, cx| shell.change_timeline_page(direction, cx));
            });
        let selected_result = view.monthly.selected_month.and_then(|month| {
            view.monthly
                .timeline
                .iter()
                .find(|item| item.month_number == month)
        });
        div().child(monthly_simulation_page(
            MonthlySimulationPage {
                state: view.state,
                is_simulating: view.monthly.is_simulating,
                is_loading_timeline: view.monthly.is_loading_timeline,
                timeline: &view.monthly.timeline,
                selected_month: view.monthly.selected_month,
                timeline_page: view.monthly.timeline_page,
                timeline_error: view.monthly.timeline_error.as_deref(),
                data: selected_result,
                error: view.monthly.error.as_deref(),
            },
            MonthlySimulationActions {
                on_simulate,
                on_reporting,
                on_select_month,
                on_reload_timeline,
                on_change_timeline_page,
            },
            theme,
        ))
    } else if view.page == AppPage::Summary && view.state == SimulationState::Stopped {
        let shell = cx.entity().downgrade();
        let on_retry: PageAction = Rc::new(move |cx: &mut App| {
            let _ = shell.update(cx, |shell, cx| shell.load_report(cx));
        });
        div().child(reporting_page(
            view.reporting.data.as_ref(),
            view.reporting.is_loading,
            view.reporting.error.as_deref(),
            on_retry,
            theme,
        ))
    } else {
        let guidance = page_content::for_route(view.page, view.state);
        div().child(placeholder_page(guidance.title, guidance.description, cx))
    };

    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(cx.theme().background)
        .text_color(cx.theme().foreground)
        .child(
            TitleBar::new().child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .child(div().text_lg().child("Property Simulation")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .justify_center()
                            .child(system_actions::render(
                                view.state,
                                view.monthly.is_simulating,
                                cx.entity(),
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .justify_end()
                            .child(navigation::render(
                                view.page,
                                view.state,
                                view.monthly.is_simulating,
                                cx.entity(),
                            )),
                    ),
            ),
        )
        .child(page_content.flex_1().min_h_0())
}
