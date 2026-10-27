use gpui_kit::{Hsla, IntoElement, ParentElement as _, Styled as _, div, px};

use crate::{ReportAmountView, ReportingViewData, components::PageTheme};

pub(super) fn financial_position(data: &ReportingViewData, theme: PageTheme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(div().text_lg().child("Financial position"))
        .child(
            div()
                .grid()
                .grid_cols(3)
                .gap_4()
                .child(amount_panel(
                    "Assets",
                    &data.assets,
                    "Total assets",
                    &data.total_assets,
                    theme,
                ))
                .child(amount_panel(
                    "Liabilities",
                    &data.liabilities,
                    "Total liabilities",
                    &data.total_liabilities,
                    theme,
                ))
                .child(equity_panel(data, theme)),
        )
}

fn amount_panel(
    title: &'static str,
    rows: &[ReportAmountView],
    total_label: &'static str,
    total: &str,
    theme: PageTheme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .p_5()
        .child(div().text_base().child(title))
        .children(
            rows.iter()
                .map(|row| amount_row(&row.label, &row.amount, theme.muted_foreground)),
        )
        .child(div().h(px(1.0)).w_full().bg(theme.border))
        .child(amount_row(total_label, total, theme.muted_foreground))
}

fn equity_panel(data: &ReportingViewData, theme: PageTheme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.panel)
        .p_5()
        .child(div().text_base().child("Equity"))
        .children(
            data.equity_accounts
                .iter()
                .map(|row| amount_row(&row.label, &row.amount, theme.muted_foreground)),
        )
        .child(amount_row(
            "Cumulative net income",
            &data.cumulative_net_income,
            theme.muted_foreground,
        ))
        .child(div().h(px(1.0)).w_full().bg(theme.border))
        .child(amount_row(
            "Total equity",
            &data.total_equity,
            theme.muted_foreground,
        ))
}

fn amount_row(label: &str, amount: &str, muted: Hsla) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(div().text_sm().text_color(muted).child(label.to_owned()))
        .child(div().text_sm().child(amount.to_owned()))
}
