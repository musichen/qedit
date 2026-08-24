use crate::browser_view::BrowserView;
use crate::history::HistoryMatch;
use editor::Editor;
use gpui::{
    App, Entity, Focusable, InteractiveElement, IntoElement, ParentElement, SharedString, Styled,
    Window, div, prelude::*, px, rems,
};
use ui::{Color, Icon, IconName, IconSize, h_flex, prelude::*, v_flex};

pub fn render_new_tab_page(
    browser_view: Entity<BrowserView>,
    search_editor: Entity<Editor>,
    search_text: String,
    suggestions: Vec<HistoryMatch>,
    selected_index: Option<usize>,
    is_incognito_window: bool,
    window: &mut Window,
    cx: &mut gpui::Context<BrowserView>,
) -> impl IntoElement {
    let theme = cx.theme();
    let radius = theme.component_radius().panel.unwrap_or(px(10.0));
    let content_width = px(560.0);
    let horizontal_padding = px(16.0);
    let menu_width = content_width - horizontal_padding * 2.0 - px(2.0);
    let is_focused = search_editor.focus_handle(cx).contains_focused(window, cx);
    let browser_view_for_confirm = browser_view.downgrade();
    let browser_view_for_cancel = browser_view.downgrade();
    let browser_view_for_up = browser_view.downgrade();
    let browser_view_for_down = browser_view.downgrade();
    let row_count = suggestions.len() + usize::from(!search_text.is_empty());

    let search_box = div()
        .w_full()
        .max_w_full()
        .min_h(px(52.0))
        .px_4()
        .py_3()
        .gap_3()
        .flex()
        .items_center()
        .overflow_hidden()
        .bg(theme.colors().elevated_surface_background)
        .border_1()
        .border_color(if is_focused {
            theme.colors().border_focused
        } else {
            theme.colors().border_variant
        })
        .rounded(radius)
        .shadow_sm()
        .child(
            Icon::new(IconName::MagnifyingGlass)
                .size(IconSize::Medium)
                .color(Color::Muted),
        )
        .child(
            div()
                .w_full()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_size(rems(1.0))
                .text_color(theme.colors().text)
                .child(search_editor.clone()),
        );

    let dropdown = if !is_focused || row_count == 0 {
        None
    } else {
        Some(render_search_menu(
            browser_view,
            search_text,
            suggestions,
            selected_index,
            menu_width,
            radius,
            cx,
        ))
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .bg(theme.colors().editor_background)
        .child(
            div()
                .w(content_width)
                .max_w_full()
                .px_4()
                .flex()
                .flex_col()
                .items_center()
                .gap_6()
                .when(is_incognito_window, |this| {
                    this.child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded(radius)
                            .border_1()
                            .border_color(theme.colors().border)
                            .bg(theme.colors().element_background)
                            .text_size(rems(0.8125))
                            .text_color(theme.colors().text)
                            .child("Incognito Window"),
                    )
                    .child(
                        div()
                            .text_size(rems(0.75))
                            .text_color(theme.colors().text_muted)
                            .text_center()
                            .max_w(px(500.0))
                            .child("Your browsing activity in this window is not saved to browser history or session restore."),
                    )
                })
                .child(
                    div()
                        .relative()
                        .w_full()
                        .max_w_full()
                        .track_focus(&search_editor.focus_handle(cx))
                        .key_context("NewTabSearch")
                        .on_action(move |_: &menu::Confirm, window, cx| {
                            let _ = browser_view_for_confirm.update(cx, |browser_view, cx| {
                                let query = browser_view.new_tab_search_text().to_string();
                                browser_view.submit_new_tab_search(&query, window, cx);
                            });
                            cx.stop_propagation();
                        })
                        .on_action(move |_: &menu::Cancel, window, cx| {
                            let _ = browser_view_for_cancel.update(cx, |browser_view, cx| {
                                browser_view.new_tab_cancel(cx);
                                window.focus(&browser_view.focus_handle(cx), cx);
                            });
                            cx.stop_propagation();
                        })
                        .on_action(move |_: &zed_actions::editor::MoveUp, _window, cx| {
                            let _ = browser_view_for_up.update(cx, |browser_view, cx| {
                                browser_view.new_tab_move_up(cx);
                            });
                            cx.stop_propagation();
                        })
                        .on_action(move |_: &zed_actions::editor::MoveDown, _window, cx| {
                            let _ = browser_view_for_down.update(cx, |browser_view, cx| {
                                browser_view.new_tab_move_down(cx);
                            });
                            cx.stop_propagation();
                        })
                        .child(search_box)
                        .when_some(dropdown, |this, dropdown| this.child(dropdown)),
                ),
        )
}

fn render_search_menu(
    browser_view: Entity<BrowserView>,
    search_text: String,
    suggestions: Vec<HistoryMatch>,
    selected_index: Option<usize>,
    width: gpui::Pixels,
    radius: gpui::Pixels,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let mut rows = Vec::new();
    let mut row_index = 0usize;

    if !search_text.is_empty() {
        rows.push(
            render_search_row(
                browser_view.clone(),
                row_index,
                selected_index == Some(row_index),
                IconName::MagnifyingGlass,
                format!("Search Google for \"{}\"", truncate_label(&search_text, 72)),
                Some(SharedString::from("Google")),
                None,
                cx,
            )
            .into_any_element(),
        );
        row_index += 1;
    }

    if !suggestions.is_empty() {
        if !search_text.is_empty() {
            rows.push(
                div()
                    .mx_3()
                    .h(px(1.0))
                    .bg(theme.colors().border_variant)
                    .into_any_element(),
            );
        }

        rows.push(
            div()
                .px_3()
                .pt_2()
                .pb_1()
                .text_size(rems(0.6875))
                .text_color(theme.colors().text_muted)
                .child("History")
                .into_any_element(),
        );

        for suggestion in suggestions {
            let title = if suggestion.title.is_empty() {
                SharedString::from(suggestion.url.clone())
            } else {
                SharedString::from(suggestion.title.clone())
            };
            let detail = SharedString::from(extract_domain(&suggestion.url));
            rows.push(
                render_search_row(
                    browser_view.clone(),
                    row_index,
                    selected_index == Some(row_index),
                    IconName::HistoryRerun,
                    title,
                    Some(detail),
                    None,
                    cx,
                )
                .into_any_element(),
            );
            row_index += 1;
        }
    }

    div().absolute().top_full().left_0().right_0().mt_2().child(
        v_flex()
            .id("new-tab-search-menu")
            .overflow_y_scroll()
            .w(width)
            .max_h(px(320.0))
            .bg(theme.colors().elevated_surface_background)
            .border_1()
            .border_color(theme.colors().border_variant)
            .rounded(radius)
            .shadow_md()
            .py_2()
            .children(rows),
    )
}

fn render_search_row(
    browser_view: Entity<BrowserView>,
    index: usize,
    is_selected: bool,
    icon: IconName,
    title: impl Into<SharedString>,
    detail: Option<SharedString>,
    trailing: Option<SharedString>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let background = if is_selected {
        theme.colors().ghost_element_selected
    } else {
        theme.colors().ghost_element_background
    };

    div()
        .id(("new-tab-search-row", index))
        .mx_2()
        .px_3()
        .py_2()
        .rounded(theme.component_radius().tab.unwrap_or(px(8.0)))
        .bg(background)
        .when(!is_selected, |this| {
            this.hover(|style| style.bg(theme.colors().ghost_element_hover))
        })
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
            let _ = browser_view.update(cx, |browser_view, cx| {
                browser_view.activate_new_tab_row(index, window, cx);
            });
        })
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap_3()
                .child(Icon::new(icon).size(IconSize::Small).color(Color::Muted))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_0p5()
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_size(rems(0.875))
                                .text_color(theme.colors().text)
                                .child(title.into()),
                        )
                        .when_some(detail, |this, detail| {
                            this.child(
                                div()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .text_size(rems(0.75))
                                    .text_color(theme.colors().text_muted)
                                    .child(detail),
                            )
                        }),
                )
                .when_some(trailing, |this, trailing| {
                    this.child(
                        div()
                            .max_w(px(220.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(rems(0.75))
                            .text_color(theme.colors().text_muted)
                            .child(trailing),
                    )
                }),
        )
}

fn truncate_label(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    let truncated: String = input.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{truncated}...")
}

pub(crate) fn extract_domain(url: &str) -> String {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}
