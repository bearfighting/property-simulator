//! Final simulation summary page.

mod financial_position;
mod period;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{App, ClickEvent, IntoElement, ParentElement as _, Styled as _, Window, div, px};

use crate::{
    PageAction, ReportingViewData,
    components::{PageTheme, page_heading},
};

use self::{
    financial_position::financial_position,
    period::{cumulative_cash_flow, period_performance},
};

pub(crate) fn reporting_page(
    data: Option<&ReportingViewData>,
    is_loading: bool,
    error: Option<&str>,
    on_retry: PageAction,
    theme: PageTheme,
) -> impl IntoElement {
    let muted = theme.muted_foreground;
    let content = if is_loading {
        centered_message("Preparing the final report…", muted).into_any_element()
    } else if let Some(error) = error {
        let on_retry = on_retry.clone();
        div()
            .w_full()
            .rounded_lg()
            .border_1()
            .border_color(theme.danger_foreground)
            .p_5()
            .flex()
            .flex_col()
            .items_start()
            .gap_3()
            .child(
                div()
                    .text_base()
                    .text_color(theme.danger_foreground)
                    .child(format!("Could not generate the final report: {error}")),
            )
            .child(
                Button::new("retry-report")
                    .secondary()
                    .label("Try again")
                    .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                        on_retry(cx);
                    }),
            )
            .into_any_element()
    } else if let Some(data) = data {
        report_content(data, theme).into_any_element()
    } else {
        centered_message(
            "The report is not available in the current simulation state.",
            muted,
        )
        .into_any_element()
    };

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
                .child(page_heading(
                    "Reporting",
                    "Final financial position and cumulative results for this simulation.",
                    theme.muted_foreground,
                ))
                .child(content),
        )
}

fn report_content(data: &ReportingViewData, theme: PageTheme) -> impl IntoElement {
    let muted = theme.muted_foreground;
    let subtitle = if data.completed_months == 0 {
        "Opening position · no completed simulation months".to_owned()
    } else {
        format!(
            "Through Month {} · {} completed months",
            data.as_of_month, data.completed_months
        )
    };
    let metrics = [
        ("Ending cash", data.ending_cash.as_str()),
        ("Total assets", data.total_assets.as_str()),
        ("Mortgage balance", data.mortgage_balance.as_str()),
        ("Total equity", data.total_equity.as_str()),
    ];

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_lg().child(data.simulation_name.clone()))
                .child(div().text_sm().text_color(muted).child(subtitle)),
        )
        .when(data.completed_months == 0, |this| {
            this.child(info_banner(
                "No monthly simulation results",
                "This report uses only the Month 0 entries saved during initialization.",
                theme,
            ))
        })
        .child(
            div().grid().grid_cols(4).gap_4().children(
                metrics
                    .into_iter()
                    .map(|(label, amount)| metric_card(label, amount, theme)),
            ),
        )
        .child(financial_position(data, theme))
        .when_some(data.period.as_ref(), |this, period| {
            this.child(period_performance(period, theme))
                .child(cumulative_cash_flow(period, theme))
        })
}

fn metric_card(label: &'static str, amount: &str, theme: PageTheme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .p_4()
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(div().text_lg().child(amount.to_owned()))
}

fn info_banner(
    title: &'static str,
    description: &'static str,
    theme: PageTheme,
) -> impl IntoElement {
    div()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .p_4()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_sm().child(title))
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(description),
        )
}

fn centered_message(message: &'static str, muted: gpui_kit::Hsla) -> impl IntoElement {
    div()
        .w_full()
        .rounded_lg()
        .border_1()
        .border_color(muted)
        .p_6()
        .text_sm()
        .text_color(muted)
        .child(message)
}
