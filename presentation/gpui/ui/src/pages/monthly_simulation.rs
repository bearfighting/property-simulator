use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::chart::PieChart;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::{
    AnyElement, App, ClickEvent, Hsla, IntoElement, ParentElement as _, Styled as _, Window, div,
    px,
};
use property_simulation_lifecycle::SimulationState;
use std::rc::Rc;

use crate::{
    CashFlowChartSlice, CashFlowViewCategory, MONTHS_PER_TIMELINE_PAGE, MonthlySimulationViewData,
    PageAction, TimelinePageDirection,
    components::{PageTheme, page_heading},
};

pub(crate) fn monthly_simulation_page(
    state: SimulationState,
    is_simulating: bool,
    is_loading_timeline: bool,
    timeline: &[MonthlySimulationViewData],
    selected_month: Option<u16>,
    timeline_page: usize,
    timeline_error: Option<&str>,
    data: Option<&MonthlySimulationViewData>,
    error: Option<&str>,
    on_simulate: PageAction,
    on_reporting: PageAction,
    on_select_month: Rc<dyn Fn(u16, &mut App)>,
    on_reload_timeline: PageAction,
    on_change_timeline_page: Rc<dyn Fn(TimelinePageDirection, &mut App)>,
    theme: PageTheme,
) -> impl IntoElement {
    let next_month = timeline
        .last()
        .map_or(1, |month| month.month_number.saturating_add(1));
    let state_label = if is_simulating {
        format!("Simulating Month {}", next_month)
    } else if state == SimulationState::Stopped {
        "Simulation stopped".to_owned()
    } else {
        format!("Ready for Month {}", next_month)
    };
    let on_simulate = on_simulate.clone();
    let button = Button::new("simulate-month")
        .primary()
        .label(if is_simulating {
            "Simulating month…"
        } else {
            "Simulate month"
        })
        .disabled(state != SimulationState::Running || is_simulating || is_loading_timeline)
        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            on_simulate(cx);
        });
    let reporting_button = Button::new("monthly-view-reporting")
        .secondary()
        .label("View reporting")
        .disabled(state != SimulationState::Stopped || is_simulating)
        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            on_reporting(cx);
        });
    let muted = theme.muted_foreground;
    let border = theme.border;
    let popover = theme.panel;

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .overflow_y_scrollbar()
        .items_center()
        .px_8()
        .py_8()
        .child(
            div()
                .w_full()
                .max_w(px(1040.0))
                .flex()
                .flex_col()
                .gap_6()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_6()
                        .child(page_heading(
                            "Monthly simulation",
                            "Run a complete month and review its cash inflows and outflows.",
                            muted,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_end()
                                .gap_2()
                                .child(div().text_sm().text_color(muted).child(state_label))
                                .child(button)
                                .child(reporting_button),
                        ),
                )
                .children(error.map(|error| {
                    div()
                        .w_full()
                        .rounded_lg()
                        .border_1()
                        .border_color(theme.danger_foreground)
                        .p_4()
                        .text_color(theme.danger_foreground)
                        .child(format!("Monthly simulation stopped: {error}"))
                }))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap_5()
                        .child(timeline_panel(
                            timeline,
                            selected_month,
                            timeline_page,
                            is_loading_timeline,
                            timeline_error,
                            on_select_month,
                            on_reload_timeline,
                            on_change_timeline_page,
                            theme,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_4()
                                .child(if let Some(data) = data {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_4()
                                        .child(div().text_lg().child(format!(
                                            "Month {} cash flow",
                                            data.month_number
                                        )))
                                        .child(
                                            div()
                                                .grid()
                                                .grid_cols(2)
                                                .gap_5()
                                                .child(flow_panel(
                                                    "Cash inflows",
                                                    &data.inflow_total,
                                                    &data.inflows,
                                                    true,
                                                    theme,
                                                ))
                                                .child(flow_panel(
                                                    "Cash outflows",
                                                    &data.outflow_total,
                                                    &data.outflows,
                                                    false,
                                                    theme,
                                                )),
                                        )
                                        .into_any_element()
                                } else {
                                    empty_results(theme).into_any_element()
                                })
                                .children(data.and_then(|data| {
                                    data.stop_reason.as_ref().map(|reason| {
                                        div()
                                            .w_full()
                                            .rounded_lg()
                                            .border_1()
                                            .border_color(border)
                                            .bg(popover)
                                            .p_4()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .child("Automatic stop condition met"),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(muted)
                                                    .child(reason.clone()),
                                            )
                                    })
                                })),
                        ),
                ),
        )
}

fn timeline_panel(
    timeline: &[MonthlySimulationViewData],
    selected_month: Option<u16>,
    timeline_page: usize,
    is_loading: bool,
    error: Option<&str>,
    on_select_month: Rc<dyn Fn(u16, &mut App)>,
    on_reload: PageAction,
    on_change_page: Rc<dyn Fn(TimelinePageDirection, &mut App)>,
    theme: PageTheme,
) -> impl IntoElement {
    let muted = theme.muted_foreground;
    let mut months_panel = div()
        .flex()
        .flex_col()
        .gap_3()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .p_4();

    if is_loading && timeline.is_empty() {
        months_panel =
            months_panel.child(div().text_sm().text_color(muted).child("Loading months…"));
    } else if let Some(error) = error {
        let on_reload = on_reload.clone();
        months_panel = months_panel
            .child(
                div()
                    .text_sm()
                    .text_color(theme.danger_foreground)
                    .child(error.to_owned()),
            )
            .child(
                Button::new("reload-month-timeline")
                    .label("Retry")
                    .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| on_reload(cx)),
            );
    } else if timeline.is_empty() {
        months_panel = months_panel.child(
            div()
                .text_sm()
                .text_color(muted)
                .child("Complete Month 1 to start the timeline."),
        );
    } else {
        let page_count = timeline.len().div_ceil(MONTHS_PER_TIMELINE_PAGE);
        let page = timeline_page.min(page_count.saturating_sub(1));
        let end = timeline
            .len()
            .saturating_sub(page * MONTHS_PER_TIMELINE_PAGE);
        let start = end.saturating_sub(MONTHS_PER_TIMELINE_PAGE);
        months_panel = months_panel.children(timeline[start..end].iter().rev().map(|month| {
            let number = month.month_number;
            let on_select = on_select_month.clone();
            let button = Button::new(format!("timeline-month-{number}"))
                .label(format!("Month {number}"))
                .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                    on_select(number, cx);
                });
            if selected_month == Some(number) {
                button.primary()
            } else {
                button
            }
        }));
        if page_count > 1 {
            let newer_action = on_change_page.clone();
            let older_action = on_change_page;
            months_panel = months_panel.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        Button::new("timeline-newer-page")
                            .label("Newer")
                            .disabled(page == 0)
                            .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                newer_action(TimelinePageDirection::Newer, cx);
                            }),
                    )
                    .child(div().text_xs().text_color(muted).child(format!(
                        "{} / {}",
                        page + 1,
                        page_count
                    )))
                    .child(
                        Button::new("timeline-older-page")
                            .label("Older")
                            .disabled(page + 1 >= page_count)
                            .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                older_action(TimelinePageDirection::Older, cx);
                            }),
                    ),
            );
        }
    }

    div()
        .w(px(190.0))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .h(px(24.0))
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_lg().child("Timeline"))
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(format!("{} months", timeline.len())),
                ),
        )
        .child(months_panel)
}

fn empty_results(theme: PageTheme) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .p_6()
        .child(div().text_lg().child("No monthly results yet"))
        .child(div().text_sm().text_color(theme.muted_foreground).child(
            "The charts will show separate cash inflows and outflows after Month 1 completes.",
        ))
}

fn flow_panel(
    title: &'static str,
    total: &str,
    slices: &[CashFlowChartSlice],
    is_inflow: bool,
    theme: PageTheme,
) -> impl IntoElement {
    let colors = theme.chart_colors;
    let muted = theme.muted_foreground;
    let has_values = slices.iter().any(|slice| slice.share > 0.0);
    let chart: AnyElement = if has_values {
        let chart_slices = slices.to_vec();
        let chart_colors = colors;
        PieChart::new(chart_slices)
            .id(if is_inflow {
                "monthly-cash-inflows"
            } else {
                "monthly-cash-outflows"
            })
            .value(|slice| slice.share)
            .color(move |slice| chart_color(slice.category, chart_colors))
            .tooltip_name(|slice| slice.label.clone().into())
            .tooltip_value(|slice, _, _| slice.amount.clone().into())
            .inner_radius(48.0)
            .outer_radius(78.0)
            .into_any_element()
    } else {
        div()
            .h(px(180.0))
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(muted)
            .child(if is_inflow {
                "No cash inflows this month"
            } else {
                "No cash outflows this month"
            })
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap_3()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .p_5()
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_2()
                .child(div().text_base().child(title))
                .child(div().text_base().child(total.to_owned())),
        )
        .child(
            div()
                .h(px(190.0))
                .flex()
                .items_center()
                .justify_center()
                .child(chart),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .children(slices.iter().map(|slice| {
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .size_3()
                                        .rounded_full()
                                        .bg(chart_color(slice.category, colors)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(div().text_sm().child(slice.label.clone()))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(muted)
                                                .child(category_label(slice.category)),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(muted)
                                .child(slice.amount.clone()),
                        )
                })),
        )
}

const fn category_label(category: CashFlowViewCategory) -> &'static str {
    match category {
        CashFlowViewCategory::Operating => "Operating",
        CashFlowViewCategory::Investing => "Investing",
        CashFlowViewCategory::Financing => "Financing",
    }
}

fn chart_color(category: CashFlowViewCategory, colors: [Hsla; 3]) -> Hsla {
    colors[match category {
        CashFlowViewCategory::Operating => 0,
        CashFlowViewCategory::Investing => 1,
        CashFlowViewCategory::Financing => 2,
    }]
}
