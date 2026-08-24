use browser::history::HistoryMatch;
use gpui::{Hsla, Subscription};

#[derive(Default)]
pub(crate) struct NativeToolbarState {
    pub(crate) omnibox_text: String,
    pub(crate) omnibox_focused: bool,
    pub(crate) omnibox_panel_dirty: bool,
    pub(crate) omnibox_suggestions: Vec<HistoryMatch>,
    pub(crate) omnibox_selected_index: Option<usize>,
    pub(crate) last_toolbar_key: String,
    pub(crate) status_encoding: Option<String>,
    pub(crate) status_line_ending: Option<String>,
    pub(crate) status_toolchain: Option<String>,
    pub(crate) status_image_info: Option<String>,
    pub(crate) active_editor_subscription: Option<Subscription>,
    pub(crate) active_image_subscription: Option<Subscription>,
    pub(crate) active_page_chrome_color: Option<Hsla>,
    pub(crate) active_page_chrome_url: Option<String>,
}
