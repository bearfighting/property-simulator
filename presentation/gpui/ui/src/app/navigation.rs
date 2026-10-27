use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::{App, ClickEvent, Entity, Window};
use gpui_kit::{IntoElement, ParentElement as _, Styled as _, div};

use super::{page::AppPage, shell::AppShell};

pub(super) fn render(
    current_page: AppPage,
    disabled: bool,
    app: Entity<AppShell>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(navigation_button(
            "Setup",
            AppPage::Initialization,
            current_page,
            disabled,
            app.clone(),
        ))
        .child(navigation_button(
            "Simulation",
            AppPage::MonthlySimulation,
            current_page,
            disabled,
            app.clone(),
        ))
        .child(navigation_button(
            "Reporting",
            AppPage::Summary,
            current_page,
            disabled,
            app,
        ))
}

fn navigation_button(
    label: &'static str,
    page: AppPage,
    current_page: AppPage,
    disabled: bool,
    app: Entity<AppShell>,
) -> impl IntoElement {
    let button = Button::new(format!(
        "navigation-{}",
        page.label().to_lowercase().replace(' ', "-")
    ))
    .label(label);
    let button = if page == current_page {
        button.secondary()
    } else {
        button.ghost()
    };

    button
        .disabled(disabled)
        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            app.update(cx, |shell, cx| shell.navigate(page, cx));
        })
}
