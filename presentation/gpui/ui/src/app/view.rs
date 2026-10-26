use gpui_kit::component::{ActiveTheme as _, TitleBar};
use gpui_kit::{Context, Entity, IntoElement, ParentElement as _, Styled as _, div};
use property_simulation_lifecycle::SimulationState;

use crate::pages::{InitializationPage, placeholder_page};

use super::shell::AppShell;
use super::{navigation, page::AppPage, page_content};

/// Composes the persistent window layout from the selected route and content.
/// It does not own navigation or lifecycle state.
pub(super) fn render(
    page: AppPage,
    state: SimulationState,
    initialization: Entity<InitializationPage>,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let page_content = if page == AppPage::Initialization {
        div().child(initialization)
    } else {
        let guidance = page_content::for_route(page, state);
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
                    .justify_between()
                    .gap_8()
                    .px_4()
                    .child(div().text_lg().child("Property Simulation"))
                    .child(navigation::render(page, cx.entity())),
            ),
        )
        .child(page_content.flex_1().min_h_0())
}
