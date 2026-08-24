use editor::{Editor, EditorEvent};
use gpui::{Action, Context, Corner, Entity, EventEmitter, Subscription, Window, div};
use language::{LineEnding, Point};
use ui::ContextMenu;
use ui::{
    Button, Color, Icon, IconName, IconSize, LabelSize, ParentElement, PopoverMenu, Render,
    Tooltip, prelude::*,
};
use workspace::{ToolbarItemEvent, ToolbarItemLocation, ToolbarItemView, item::ItemHandle};
use zed_actions::OpenSettingsAt;

#[derive(Clone)]
struct DocumentStatus {
    label: SharedString,
    language: SharedString,
    position: SharedString,
    encoding: Option<SharedString>,
    line_ending: Option<SharedString>,
}

pub struct DocumentStatusButton {
    status: Option<DocumentStatus>,
    active_editor_subscription: Option<Subscription>,
}

impl DocumentStatusButton {
    pub fn new() -> Self {
        Self {
            status: None,
            active_editor_subscription: None,
        }
    }

    fn update_status(&mut self, editor: Entity<Editor>, _: &mut Window, cx: &mut Context<Self>) {
        self.status = editor.update(cx, |editor, cx| {
            if !matches!(editor.mode(), editor::EditorMode::Full { .. }) {
                return None;
            }

            let snapshot = editor.display_snapshot(cx);
            if snapshot.buffer_snapshot().excerpts().count() == 0 {
                return None;
            }

            let newest = editor.selections.newest::<Point>(&snapshot);
            let head = newest.head();
            let position = snapshot
                .buffer_snapshot()
                .point_to_buffer_point(head)
                .map(|(_, point)| format!("{}:{}", point.row + 1, point.column + 1))?;

            let (language, encoding, line_ending) =
                if let Some(buffer) = editor.buffer().read(cx).as_singleton() {
                    let buffer = buffer.read(cx);
                    let language = buffer
                        .language()
                        .map(|language| language.name().to_string())
                        .unwrap_or_else(|| "Plain Text".to_string());

                    let encoding = {
                        let encoding = buffer.encoding();
                        let has_bom = buffer.has_bom();
                        if encoding.name() != "UTF-8" || has_bom {
                            let mut text = encoding.name().to_string();
                            if has_bom {
                                text.push_str(" (BOM)");
                            }
                            Some(text.into())
                        } else {
                            None
                        }
                    };

                    let line_ending = {
                        let line_ending = buffer.line_ending();
                        if line_ending != LineEnding::Unix {
                            Some(line_ending.label().to_string().into())
                        } else {
                            None
                        }
                    };
                    (language, encoding, line_ending)
                } else {
                    ("Plain Text".to_string(), None, None)
                };

            Some(DocumentStatus {
                label: format!("{language}  {position}").into(),
                language: language.into(),
                position: position.into(),
                encoding,
                line_ending,
            })
        });

        cx.notify();
    }
}

impl Render for DocumentStatusButton {
    fn render(&mut self, _window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let Some(status) = self.status.clone() else {
            return div().hidden();
        };

        div().child(
            PopoverMenu::new("document-status")
                .anchor(Corner::TopRight)
                .trigger_with_tooltip(
                    Button::new("document-status-trigger", status.label.clone())
                        .label_size(LabelSize::Small)
                        .color(Color::Muted)
                        .end_icon(
                            Icon::new(IconName::ChevronDown)
                                .size(IconSize::XSmall)
                                .color(Color::Muted),
                        ),
                    Tooltip::text("Document Status"),
                )
                .menu(move |window, cx| {
                    let status = status.clone();
                    Some(ContextMenu::build(window, cx, move |menu, _, _| {
                        let mut menu = menu.header(format!(
                            "{}  {}",
                            status.language.clone(),
                            status.position.clone()
                        ));

                        menu = menu
                            .separator()
                            .entry(
                                "Select Language",
                                Some(language_selector::Toggle.boxed_clone()),
                                move |window, cx| {
                                    window.dispatch_action(
                                        language_selector::Toggle.boxed_clone(),
                                        cx,
                                    );
                                },
                            )
                            .entry(
                                "Go to Line/Column",
                                Some(editor::actions::ToggleGoToLine.boxed_clone()),
                                move |window, cx| {
                                    window.dispatch_action(
                                        editor::actions::ToggleGoToLine.boxed_clone(),
                                        cx,
                                    );
                                },
                            );

                        if let Some(encoding) = status.encoding.clone() {
                            menu = menu.entry(
                                format!("Encoding: {encoding}"),
                                Some(encoding_selector::Toggle.boxed_clone()),
                                move |window, cx| {
                                    window.dispatch_action(
                                        encoding_selector::Toggle.boxed_clone(),
                                        cx,
                                    );
                                },
                            );
                        }

                        if let Some(line_ending) = status.line_ending.clone() {
                            menu = menu.entry(
                                format!("Line Endings: {line_ending}"),
                                Some(line_ending_selector::Toggle.boxed_clone()),
                                move |window, cx| {
                                    window.dispatch_action(
                                        line_ending_selector::Toggle.boxed_clone(),
                                        cx,
                                    );
                                },
                            );
                        }

                        menu.entry(
                            "Configure Edit Predictions",
                            Some(
                                OpenSettingsAt {
                                    path: "edit_predictions.providers".to_string(),
                                }
                                .boxed_clone(),
                            ),
                            move |window, cx| {
                                window.dispatch_action(
                                    OpenSettingsAt {
                                        path: "edit_predictions.providers".to_string(),
                                    }
                                    .boxed_clone(),
                                    cx,
                                );
                            },
                        )
                    }))
                }),
        )
    }
}

impl EventEmitter<ToolbarItemEvent> for DocumentStatusButton {}

impl ToolbarItemView for DocumentStatusButton {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> ToolbarItemLocation {
        if let Some(editor) = active_pane_item.and_then(|item| item.downcast::<Editor>()) {
            self.active_editor_subscription =
                Some(
                    cx.subscribe_in(&editor, window, |this, editor, event, window, cx| {
                        if matches!(
                            event,
                            EditorEvent::SelectionsChanged { .. } | EditorEvent::BufferEdited
                        ) {
                            this.update_status(editor.clone(), window, cx);
                        }
                    }),
                );
            self.update_status(editor, window, cx);
            ToolbarItemLocation::PrimaryRight
        } else {
            self.status = None;
            self.active_editor_subscription = None;
            ToolbarItemLocation::Hidden
        }
    }
}
