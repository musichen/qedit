use browser::BrowserView;
use gpui::{
    NativeToolbarItem, NativeToolbarSearchEvent, NativeToolbarSearchField, SharedString, px,
};
use workspace_modes::ModeId;

use crate::TitleBar;

impl TitleBar {
    pub(crate) fn build_omnibox_item(&self) -> NativeToolbarItem {
        let workspace_for_change = self.workspace.clone();
        let workspace_for_submit = self.workspace.clone();
        let workspace_for_move_up = self.workspace.clone();
        let workspace_for_move_down = self.workspace.clone();
        let workspace_for_cancel = self.workspace.clone();
        let workspace_for_begin = self.workspace.clone();
        let workspace_for_end = self.workspace.clone();

        NativeToolbarItem::SearchField(
            NativeToolbarSearchField::new("qedit.omnibox")
                .placeholder("Search or enter URL")
                .text(SharedString::from(
                    self.native_toolbar_state.omnibox_text.clone(),
                ))
                .min_width(px(300.0))
                .max_width(px(600.0))
                .on_change(move |event: &NativeToolbarSearchEvent, window, cx| {
                    if let Some(workspace) = workspace_for_change.upgrade()
                        && let Some(title_bar) = workspace
                            .read(cx)
                            .titlebar_item()
                            .and_then(|item| item.downcast::<TitleBar>().ok())
                    {
                        let text = event.text.clone();
                        title_bar.update(cx, |title_bar, cx| {
                            title_bar.native_toolbar_state.omnibox_text = text.clone();
                            title_bar.native_toolbar_state.omnibox_selected_index = None;
                            title_bar.native_toolbar_state.omnibox_suggestions.clear();
                            title_bar.show_search_suggestion_menu(window);
                            title_bar.search_history(text, cx);
                            cx.notify();
                        });
                    }
                })
                .on_submit(move |event: &NativeToolbarSearchEvent, window, cx| {
                    window.dismiss_native_search_suggestion_menu();
                    if let Some(workspace) = workspace_for_submit.upgrade()
                        && let Some(title_bar) = workspace
                            .read(cx)
                            .titlebar_item()
                            .and_then(|item| item.downcast::<TitleBar>().ok())
                    {
                        let text = event.text.clone();
                        title_bar.update(cx, |title_bar, cx| {
                            if let Some(selected) =
                                title_bar.native_toolbar_state.omnibox_selected_index
                                && let Some(url) = title_bar.url_for_selected_row(selected)
                            {
                                title_bar.native_toolbar_state.omnibox_selected_index = None;
                                title_bar.navigate_omnibox(&url, cx);
                                return;
                            }
                            title_bar.native_toolbar_state.omnibox_selected_index = None;
                            title_bar.navigate_omnibox(&text, cx);
                        });
                    }
                })
                .on_move_down(move |_event, window, cx| {
                    if let Some(workspace) = workspace_for_move_down.upgrade()
                        && let Some(title_bar) = workspace
                            .read(cx)
                            .titlebar_item()
                            .and_then(|item| item.downcast::<TitleBar>().ok())
                    {
                        title_bar.update(cx, |title_bar, cx| {
                            let total = title_bar.omnibox_row_count();
                            if total == 0 {
                                return;
                            }
                            title_bar.native_toolbar_state.omnibox_selected_index = Some(
                                match title_bar.native_toolbar_state.omnibox_selected_index {
                                    Some(index) => (index + 1) % total,
                                    None => 0,
                                },
                            );
                            title_bar.show_search_suggestion_menu(window);
                            cx.notify();
                        });
                    }
                })
                .on_move_up(move |_event, window, cx| {
                    if let Some(workspace) = workspace_for_move_up.upgrade()
                        && let Some(title_bar) = workspace
                            .read(cx)
                            .titlebar_item()
                            .and_then(|item| item.downcast::<TitleBar>().ok())
                    {
                        title_bar.update(cx, |title_bar, cx| {
                            let total = title_bar.omnibox_row_count();
                            if total == 0 {
                                return;
                            }
                            title_bar.native_toolbar_state.omnibox_selected_index = Some(
                                match title_bar.native_toolbar_state.omnibox_selected_index {
                                    Some(0) | None => total.saturating_sub(1),
                                    Some(index) => index - 1,
                                },
                            );
                            title_bar.show_search_suggestion_menu(window);
                            cx.notify();
                        });
                    }
                })
                .on_cancel(move |_event, window, cx| {
                    window.dismiss_native_search_suggestion_menu();
                    window.blur_native_field_editor();
                    if let Some(workspace) = workspace_for_cancel.upgrade()
                        && let Some(title_bar) = workspace
                            .read(cx)
                            .titlebar_item()
                            .and_then(|item| item.downcast::<TitleBar>().ok())
                    {
                        title_bar.update(cx, |title_bar, cx| {
                            title_bar.native_toolbar_state.omnibox_focused = false;
                            title_bar.native_toolbar_state.omnibox_selected_index = None;
                            cx.notify();
                        });
                    }
                })
                .on_begin_editing(move |_event, _window, cx| {
                    if let Some(workspace) = workspace_for_begin.upgrade()
                        && let Some(title_bar) = workspace
                            .read(cx)
                            .titlebar_item()
                            .and_then(|item| item.downcast::<TitleBar>().ok())
                    {
                        title_bar.update(cx, |title_bar, _cx| {
                            title_bar.native_toolbar_state.omnibox_focused = true;
                        });
                    }
                })
                .on_end_editing(move |_event, window, cx| {
                    window.dismiss_native_search_suggestion_menu();
                    if let Some(workspace) = workspace_for_end.upgrade()
                        && let Some(title_bar) = workspace
                            .read(cx)
                            .titlebar_item()
                            .and_then(|item| item.downcast::<TitleBar>().ok())
                    {
                        title_bar.update(cx, |title_bar, cx| {
                            title_bar.native_toolbar_state.omnibox_focused = false;
                            title_bar.native_toolbar_state.omnibox_selected_index = None;
                            cx.notify();
                        });
                    }
                }),
        )
    }

    pub(crate) fn build_back_item(&self) -> NativeToolbarItem {
        let workspace = self.workspace.clone();
        self.build_simple_action_button(
            "qedit.browser.back",
            "chevron.left",
            "Go Back",
            move |_window, cx| {
                if let Some(workspace) = workspace.upgrade() {
                    let browser_view = workspace
                        .read(cx)
                        .get_mode_view(ModeId::BROWSER)
                        .and_then(|view| view.downcast::<BrowserView>().ok());
                    if let Some(browser_view) = browser_view {
                        browser_view.update(cx, |browser_view, cx| {
                            if let Some(tab) = browser_view.active_tab() {
                                tab.update(cx, |tab, _| tab.go_back());
                            }
                        });
                    }
                }
            },
        )
    }

    pub(crate) fn build_forward_item(&self) -> NativeToolbarItem {
        let workspace = self.workspace.clone();
        self.build_simple_action_button(
            "qedit.browser.forward",
            "chevron.right",
            "Go Forward",
            move |_window, cx| {
                if let Some(workspace) = workspace.upgrade() {
                    let browser_view = workspace
                        .read(cx)
                        .get_mode_view(ModeId::BROWSER)
                        .and_then(|view| view.downcast::<BrowserView>().ok());
                    if let Some(browser_view) = browser_view {
                        browser_view.update(cx, |browser_view, cx| {
                            if let Some(tab) = browser_view.active_tab() {
                                tab.update(cx, |tab, _| tab.go_forward());
                            }
                        });
                    }
                }
            },
        )
    }

    pub(crate) fn build_reload_item(&self) -> NativeToolbarItem {
        let workspace = self.workspace.clone();
        self.build_simple_action_button(
            "qedit.browser.reload",
            "arrow.clockwise",
            "Reload",
            move |_window, cx| {
                if let Some(workspace) = workspace.upgrade() {
                    let browser_view = workspace
                        .read(cx)
                        .get_mode_view(ModeId::BROWSER)
                        .and_then(|view| view.downcast::<BrowserView>().ok());
                    if let Some(browser_view) = browser_view {
                        browser_view.update(cx, |browser_view, cx| {
                            if let Some(tab) = browser_view.active_tab() {
                                tab.update(cx, |tab, _| tab.reload());
                            }
                        });
                    }
                }
            },
        )
    }

    pub(crate) fn build_downloads_item(&self) -> NativeToolbarItem {
        let workspace = self.workspace.clone();
        self.build_simple_action_button(
            "qedit.browser.downloads",
            "arrow.down.circle",
            "Downloads",
            move |window, cx| {
                if let Some(workspace) = workspace.upgrade()
                    && let Some(title_bar) = workspace
                        .read(cx)
                        .titlebar_item()
                        .and_then(|item| item.downcast::<TitleBar>().ok())
                {
                    title_bar.update(cx, |title_bar, cx| {
                        title_bar.show_downloads_panel(window, cx);
                    });
                }
            },
        )
    }
}
