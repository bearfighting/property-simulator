use gpui_kit::{AppContext as _, Context, Entity, IntoElement, Render, Window};
use property_simulation_lifecycle::SimulationState;

use crate::{InitializationHandler, app::page::AppPage, pages::InitializationPage};

pub(crate) struct AppShell {
    page: AppPage,
    initialization: Entity<InitializationPage>,
    state: SimulationState,
}

impl AppShell {
    pub(super) fn new(
        handler: InitializationHandler,
        initial_state: SimulationState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let shell = cx.entity().downgrade();
        let on_state_changed = {
            let shell = shell.clone();
            std::rc::Rc::new(
                move |state, cx: &mut gpui_kit::Context<InitializationPage>| {
                    let _ = shell.update(cx, |shell, cx| shell.initialization_succeeded(state, cx));
                },
            )
        };
        let on_continue = {
            let shell = shell.clone();
            std::rc::Rc::new(move |cx: &mut gpui_kit::Context<InitializationPage>| {
                let _ = shell.update(cx, |shell, cx| shell.navigate_to_simulation(cx));
            })
        };
        let initialization = cx
            .new(|cx| InitializationPage::new(handler, on_state_changed, on_continue, window, cx));
        Self {
            page: AppPage::Initialization,
            initialization,
            state: initial_state,
        }
    }

    pub(crate) fn initialization_succeeded(
        &mut self,
        state: SimulationState,
        cx: &mut Context<Self>,
    ) {
        self.state = state;
        cx.notify();
    }

    pub(super) fn navigate(&mut self, page: AppPage, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    pub(crate) fn navigate_to_simulation(&mut self, cx: &mut Context<Self>) {
        self.navigate(AppPage::MonthlySimulation, cx);
    }
}

impl Render for AppShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::view::render(self.page, self.state, self.initialization.clone(), cx)
    }
}
