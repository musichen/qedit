use browser::{self, BrowserView};
use gpui::{
    App, Context, Entity, Hsla, NativePopover, NativePopoverAnchor, NativePopoverBehavior,
    NativePopoverClickableRow, NativePopoverContentItem, NativeSearchFieldTarget,
    NativeSearchSuggestionMenu, Window,
};

use crate::TitleBar;
use workspace_modes::ModeId;

impl TitleBar {
    fn browser_view(&self, cx: &App) -> Option<Entity<BrowserView>> {
        let workspace = self.workspace.upgrade()?;
        let view = workspace.read(cx).get_mode_view(ModeId::BROWSER)?;
        view.downcast::<BrowserView>().ok()
    }

    pub(super) fn active_tab_is_new_tab_page(&self, cx: &App) -> bool {
        self.browser_view(cx)
            .and_then(|browser_view| {
                let browser_view = browser_view.read(cx);
                browser_view
                    .active_tab()
                    .map(|tab| tab.read(cx).is_new_tab_page())
            })
            .unwrap_or(false)
    }

    pub(super) fn active_tab_page_chrome_color(&self, cx: &App) -> Option<Hsla> {
        self.browser_view(cx).and_then(|browser_view| {
            let browser_view = browser_view.read(cx);
            browser_view
                .active_tab()
                .and_then(|tab| tab.read(cx).page_chrome_color())
        })
    }

    pub(super) fn active_tab_url(&self, cx: &App) -> Option<String> {
        self.browser_view(cx).and_then(|browser_view| {
            let browser_view = browser_view.read(cx);
            browser_view
                .active_tab()
                .map(|tab| tab.read(cx).url().to_string())
        })
    }

    pub(super) fn sync_omnibox_url(&mut self, cx: &mut App) {
        if self.native_toolbar_state.omnibox_focused {
            return;
        }

        if let Some(url) = self.browser_view(cx).and_then(|browser_view| {
            let browser_view = browser_view.read(cx);
            browser_view
                .active_tab()
                .map(|tab| tab.read(cx).url().to_string())
        }) {
            let omnibox_text = display_omnibox_text(&url);
            if self.native_toolbar_state.omnibox_text != omnibox_text {
                self.native_toolbar_state.omnibox_text = omnibox_text;
            }
        }
    }

    pub(super) fn navigate_omnibox(&mut self, text: &str, cx: &mut Context<Self>) {
        if text.is_empty() {
            return;
        }
        let url = text_to_url(text);
        self.native_toolbar_state.omnibox_text = url.clone();
        self.native_toolbar_state.omnibox_focused = false;
        self.native_toolbar_state.omnibox_suggestions.clear();

        if let Some(browser_view) = self.browser_view(cx) {
            browser_view.update(cx, |browser_view, cx| {
                if let Some(tab) = browser_view.active_tab() {
                    tab.update(cx, |tab, cx| {
                        tab.navigate(&url, cx);
                    });
                }
            });
        }

        cx.notify();
    }

    pub(super) fn omnibox_row_count(&self) -> usize {
        let mut count = self.native_toolbar_state.omnibox_suggestions.len();
        if !self.native_toolbar_state.omnibox_text.is_empty() {
            count += 1;
        }
        count
    }

    pub(super) fn url_for_selected_row(&self, index: usize) -> Option<String> {
        let mut current = 0;
        if !self.native_toolbar_state.omnibox_text.is_empty() {
            if current == index {
                return Some(text_to_url(&self.native_toolbar_state.omnibox_text));
            }
            current += 1;
        }

        for suggestion in &self.native_toolbar_state.omnibox_suggestions {
            if current == index {
                return Some(suggestion.url.clone());
            }
            current += 1;
        }

        None
    }

    pub(super) fn show_search_suggestion_menu(&self, window: &mut Window) {
        if self.omnibox_row_count() == 0 {
            window.dismiss_native_search_suggestion_menu();
            return;
        }

        let workspace = self.workspace.clone();
        let selected = self.native_toolbar_state.omnibox_selected_index;
        let mut items: Vec<NativePopoverContentItem> = Vec::new();
        let mut row_count = 0usize;
        let mut row_index = 0usize;
        let has_search_row = !self.native_toolbar_state.omnibox_text.is_empty();

        if has_search_row {
            let query = self.native_toolbar_state.omnibox_text.clone();
            let search_workspace = workspace.clone();
            items.push(
                NativePopoverClickableRow::new(format!("Search \"{}\"", query))
                    .icon("magnifyingqedit")
                    .detail("Google")
                    .selected(selected == Some(row_index))
                    .on_click(move |window, cx| {
                        window.dismiss_native_search_suggestion_menu();
                        let url = text_to_url(&query);
                        if let Some(workspace) = search_workspace.upgrade()
                            && let Some(title_bar) = workspace
                                .read(cx)
                                .titlebar_item()
                                .and_then(|item| item.downcast::<TitleBar>().ok())
                        {
                            title_bar.update(cx, |title_bar, cx| {
                                title_bar.navigate_omnibox(&url, cx);
                            });
                        }
                    })
                    .into(),
            );
            row_count += 1;
            row_index += 1;
            items.push(NativePopoverContentItem::separator());
        }

        items.push(NativePopoverContentItem::heading("History"));
        for suggestion in &self.native_toolbar_state.omnibox_suggestions {
            let url = suggestion.url.clone();
            let navigate_workspace = workspace.clone();
            let title = if suggestion.title.is_empty() {
                suggestion.url.clone()
            } else {
                suggestion.title.clone()
            };
            let detail = extract_domain(&suggestion.url);
            items.push(
                NativePopoverClickableRow::new(title)
                    .icon("clock")
                    .detail(detail)
                    .selected(selected == Some(row_index))
                    .on_click(move |window, cx| {
                        window.dismiss_native_search_suggestion_menu();
                        if let Some(workspace) = navigate_workspace.upgrade()
                            && let Some(title_bar) = workspace
                                .read(cx)
                                .titlebar_item()
                                .and_then(|item| item.downcast::<TitleBar>().ok())
                        {
                            title_bar.update(cx, |title_bar, cx| {
                                title_bar.navigate_omnibox(&url, cx);
                            });
                        }
                    })
                    .into(),
            );
            row_count += 1;
            row_index += 1;
        }

        let content_height =
            (row_count as f64 * 28.0) + 28.0 + if has_search_row { 12.0 } else { 0.0 } + 32.0;
        let menu = NativeSearchSuggestionMenu::new(450.0, content_height.min(400.0)).items(items);
        window.update_native_search_suggestion_menu(
            menu,
            NativeSearchFieldTarget::ToolbarItem("qedit.omnibox".into()),
        );
    }

    pub(super) fn show_downloads_panel(&self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(browser_view) = self.browser_view(cx) else {
            return;
        };

        let (downloads, is_incognito_window) = browser_view.read_with(cx, |browser_view, _| {
            (
                browser_view.download_items(),
                browser_view.is_incognito_window(),
            )
        });

        let mut items = vec![NativePopoverContentItem::heading("Downloads")];
        let mut row_count = 1usize;

        if is_incognito_window {
            items.push(NativePopoverContentItem::small_label(
                "Incognito downloads stay in memory for this window only.".to_string(),
            ));
            row_count += 1;
            if !downloads.is_empty() {
                items.push(NativePopoverContentItem::separator());
                row_count += 1;
            }
        }

        if downloads.is_empty() {
            items.push(NativePopoverContentItem::small_label(
                "No downloads yet.".to_string(),
            ));
            row_count += 1;
        } else {
            for download in downloads {
                let browser_view = browser_view.clone();
                let detail = if download.is_incognito {
                    format!("{} • Incognito", download.status_text)
                } else {
                    download.status_text.clone()
                };

                if download.is_complete && download.full_path.is_some() {
                    let id = download.id;
                    items.push(
                        NativePopoverClickableRow::new(download.display_name)
                            .icon("arrow.down.doc")
                            .detail(detail)
                            .on_click(move |window, cx| {
                                window.dismiss_native_popover();
                                browser_view.update(cx, |browser_view, cx| {
                                    browser_view.open_download_with_system(id, cx);
                                });
                            })
                            .into(),
                    );
                } else {
                    items.push(NativePopoverContentItem::small_label(format!(
                        "{} — {}",
                        download.display_name, detail
                    )));
                }
                row_count += 1;
            }
        }

        window.dismiss_native_popover();
        window.show_native_popover(
            NativePopover::new(360.0, (row_count as f64 * 28.0 + 32.0).clamp(120.0, 360.0))
                .behavior(NativePopoverBehavior::Transient)
                .items(items),
            NativePopoverAnchor::ToolbarItem("qedit.browser.downloads".into()),
        );
    }

    pub(super) fn search_history(&mut self, query: String, cx: &mut Context<Self>) {
        let Some(entries) = self
            .browser_view(cx)
            .map(|browser_view| browser_view.read(cx).history().read(cx).entries().to_vec())
        else {
            return;
        };

        let executor = cx.background_executor().clone();
        let requested_query = query.clone();
        cx.spawn(async move |this, cx| {
            let matches =
                browser::history::BrowserHistory::search(entries, query, 8, executor).await;
            let _ = cx.update(|cx| {
                let _ = this.update(cx, |title_bar, cx| {
                    if title_bar.native_toolbar_state.omnibox_text != requested_query {
                        return;
                    }
                    title_bar.native_toolbar_state.omnibox_suggestions = matches;
                    title_bar.native_toolbar_state.omnibox_panel_dirty = true;
                    cx.notify();
                });
            });
        })
        .detach();
    }
}

fn text_to_url(text: &str) -> String {
    if text.starts_with("http://") || text.starts_with("https://") {
        text.to_string()
    } else if text.contains('.') && !text.contains(' ') {
        format!("https://{text}")
    } else {
        let encoded: String = url::form_urlencoded::byte_serialize(text.as_bytes()).collect();
        format!("https://www.google.com/search?q={encoded}")
    }
}

fn display_omnibox_text(url: &str) -> String {
    if url == "qedit://newtab" {
        String::new()
    } else {
        url.to_string()
    }
}

fn extract_domain(url: &str) -> String {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}
