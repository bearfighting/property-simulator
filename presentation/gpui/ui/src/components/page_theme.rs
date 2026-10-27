use gpui_kit::Hsla;

/// Theme values resolved by the application shell and supplied to a page.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PageTheme {
    pub(crate) border: Hsla,
    pub(crate) muted_foreground: Hsla,
    pub(crate) panel: Hsla,
    pub(crate) danger_foreground: Hsla,
    pub(crate) chart_colors: [Hsla; 3],
}
