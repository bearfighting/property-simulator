use gpui_kit::component::{ActiveTheme as _, TitleBar};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _, Window,
    div,
};

use crate::{
    InitializationHandler,
    app::{navigation, page::AppPage},
    pages::{InitializationPage, placeholder_page},
};

pub(crate) struct AppShell {
    page: AppPage,
    initialization: Entity<InitializationPage>,
}

impl AppShell {
    pub(super) fn new(
        handler: InitializationHandler,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let initialization = cx.new(|cx| InitializationPage::new(handler, window, cx));
        Self {
            page: AppPage::Initialization,
            initialization,
        }
    }

    pub(super) fn navigate(&mut self, page: AppPage, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }
}

impl Render for AppShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page_content = match self.page {
            AppPage::Initialization => div().child(self.initialization.clone()),
            AppPage::MonthlySimulation => div().child(placeholder_page(
                "Monthly simulation",
                "Monthly results and controls will appear here.",
                cx,
            )),
            AppPage::Summary => div().child(placeholder_page(
                "Simulation summary",
                "Simulation-wide results and reports will appear here.",
                cx,
            )),
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
                        .px_4()
                        .child(div().text_lg().child("Property Simulation")),
                ),
            )
            .child(navigation::render(self.page, cx.entity()))
            .child(page_content.flex_1().min_h_0())
    }
}
