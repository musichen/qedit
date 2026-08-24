use editor::EditorSettings;
use gpui::{Context, FocusHandle, IntoElement, Render, Window, div, native_icon_button};
use settings::Settings as _;
use ui::prelude::*;
use workspace::ItemHandle;
use workspace::TitleBarItemView;

pub struct SearchButton {
    pane_item_focus_handle: Option<FocusHandle>,
}

impl SearchButton {
    pub fn new() -> Self {
        Self {
            pane_item_focus_handle: None,
        }
    }
}

impl Render for SearchButton {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let button = div();

        if !EditorSettings::get_global(cx).search.button {
            return button.hidden();
        }

        button.child(
            native_icon_button("project-search-indicator", "magnifyingqedit")
                .tooltip("Project Search")
                .on_click(cx.listener(|_this, _, window, cx| {
                    window.dispatch_action(Box::new(workspace::DeploySearch::default()), cx);
                })),
        )
    }
}

impl TitleBarItemView for SearchButton {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pane_item_focus_handle = active_pane_item.map(|item| item.item_focus_handle(cx));
    }
}
