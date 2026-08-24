use app_runtime::{
    CapabilityState, ExecutionRequest, RuntimeAction, RuntimeCatalog, RuntimeError,
    SystemCommandRunner,
};
use gpui::{
    App, ClickEvent, Context, Corner, CursorStyle, DismissEvent, Entity, EventEmitter, FocusHandle,
    Focusable, Render, SharedString, Stateful, Task, WeakEntity, point, px,
};
use ui::{
    AnyElement, Clickable, Color, ContextMenu, ContextMenuEntry, Disableable, Icon, IconName,
    IconPosition, IconSize, Label, LabelSize, Modal, ModalFooter, ModalHeader, PopoverMenu,
    Toggleable, prelude::*,
};
use workspace::notifications::NotificationId;
use workspace::{ModalView, Toast, Workspace};

use crate::OpenRuntimeActions;
use crate::runtime_execution::execute_runtime_request;

#[derive(Clone, Copy)]
enum RuntimeActionButtonStyle {
    Secondary,
    Primary,
}

pub struct RuntimeActionsModal {
    focus_handle: FocusHandle,
    workspace: WeakEntity<Workspace>,
    catalog: Option<RuntimeCatalog>,
    selection: RuntimeSelectionState,
    loading: bool,
    _loading_task: Option<Task<()>>,
}

impl RuntimeActionsModal {
    pub fn toggle(
        workspace: &mut Workspace,
        _: &OpenRuntimeActions,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let workspace_paths = workspace
            .project()
            .read(cx)
            .visible_worktrees(cx)
            .map(|worktree| worktree.read(cx).abs_path().to_path_buf())
            .collect::<Vec<_>>();
        let workspace_handle = workspace.weak_handle();

        workspace.toggle_modal(window, cx, |window, cx| {
            Self::new(
                workspace_handle.clone(),
                workspace_paths.clone(),
                window,
                cx,
            )
        });
    }

    fn new(
        workspace: WeakEntity<Workspace>,
        workspace_paths: Vec<std::path::PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let load_task = cx.spawn_in(window, async move |this, cx| {
            let catalog = cx
                .background_spawn(async move {
                    let runner = SystemCommandRunner;
                    RuntimeCatalog::discover(&workspace_paths, &runner)
                })
                .await;

            this.update_in(cx, |this, _window, cx| {
                this.loading = false;
                this.selection = choose_initial_selection(&catalog);
                this.catalog = Some(catalog);
                cx.notify();
            })
            .ok();
        });

        Self {
            focus_handle,
            workspace,
            catalog: None,
            selection: RuntimeSelectionState::default(),
            loading: true,
            _loading_task: Some(load_task),
        }
    }

    fn selected_project(&self) -> Option<&app_runtime::DetectedProject> {
        let catalog = self.catalog.as_ref()?;
        selected_project(catalog, &self.selection)
    }

    fn selected_target(&self) -> Option<&app_runtime::RuntimeTarget> {
        let project = self.selected_project()?;
        project
            .targets
            .iter()
            .find(|target| Some(&target.id) == self.selection.target_id.as_ref())
    }

    fn selected_device(&self) -> Option<&app_runtime::RuntimeDevice> {
        let project = self.selected_project()?;
        project
            .devices
            .iter()
            .find(|device| Some(&device.id) == self.selection.device_id.as_ref())
    }

    fn select_project(&mut self, project_id: String, cx: &mut Context<Self>) {
        if let Some(catalog) = self.catalog.as_ref() {
            select_project(catalog, &mut self.selection, project_id);
            cx.notify();
        }
    }

    fn select_target(&mut self, target_id: String, cx: &mut Context<Self>) {
        if let Some(project) = self.selected_project().cloned() {
            select_target(&project, &mut self.selection, target_id);
            cx.notify();
        }
    }

    fn select_device(&mut self, device_id: String, cx: &mut Context<Self>) {
        self.selection.device_id = Some(device_id);
        cx.notify();
    }

    fn action_request(&self, action: RuntimeAction) -> Result<ExecutionRequest, RuntimeError> {
        let project = self
            .selected_project()
            .ok_or_else(|| RuntimeError::ProjectNotFound("selected-project".to_string()))?;
        let target = self
            .selected_target()
            .ok_or_else(|| RuntimeError::TargetNotFound("selected-target".to_string()))?;

        Ok(ExecutionRequest {
            project_id: project.id.clone(),
            target_id: target.id.clone(),
            device_id: match action {
                RuntimeAction::Build => None,
                RuntimeAction::Run => self.selected_device().map(|device| device.id.clone()),
            },
            action,
        })
    }

    fn action_reason(&self, action: RuntimeAction) -> Option<String> {
        let project = match self.selected_project() {
            Some(project) => project,
            None if self.loading => return Some("Detecting runtime capabilities.".to_string()),
            None => return Some("No runnable project was detected in this workspace.".to_string()),
        };
        if self.selected_target().is_none() {
            return Some("Choose a target.".to_string());
        }

        let capability = match action {
            RuntimeAction::Build => &project.capabilities.build,
            RuntimeAction::Run => &project.capabilities.run,
        };

        match capability {
            CapabilityState::Available => {
                if matches!(action, RuntimeAction::Run) && self.selected_device().is_none() {
                    Some("Choose a device.".to_string())
                } else {
                    None
                }
            }
            CapabilityState::RequiresSetup { reason } | CapabilityState::Unavailable { reason } => {
                Some(reason.clone())
            }
        }
    }

    fn run_action(&mut self, action: RuntimeAction, window: &mut Window, cx: &mut Context<Self>) {
        if self.action_reason(action).is_some() {
            return;
        }

        let request = match self.action_request(action) {
            Ok(request) => request,
            Err(error) => {
                self.show_error(error.to_string(), cx);
                return;
            }
        };
        let Some(catalog) = self.catalog.as_ref() else {
            return;
        };

        let launch_result = self
            .workspace
            .update(cx, |workspace, cx| {
                execute_runtime_request(workspace, catalog, &request, window, cx)
            })
            .ok()
            .and_then(Result::ok);

        match launch_result {
            Some(()) => cx.emit(DismissEvent),
            None => self.show_error("Could not start the runtime action.", cx),
        }
    }

    fn show_error(&self, message: impl Into<String>, cx: &mut Context<Self>) {
        let message = message.into();
        self.workspace
            .update(cx, |workspace, cx| {
                workspace.show_toast(
                    Toast::new(NotificationId::unique::<RuntimeActionsModal>(), message).autohide(),
                    cx,
                );
            })
            .ok();
    }

    fn cancel(&mut self, _: &menu::Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }

    fn render_project_dropdown(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let label = self
            .selected_project()
            .map(|project| project.label.clone())
            .unwrap_or_else(|| "Select project".to_string());
        let modal = cx.entity().downgrade();

        let menu = ContextMenu::build(window, cx, {
            let projects = self
                .catalog
                .as_ref()
                .map(|catalog| catalog.projects.clone())
                .unwrap_or_default();
            let selected_project_id = self.selection.project_id.clone();
            let menu_modal = modal.clone();
            move |mut menu, _, _| {
                for project in &projects {
                    let is_selected = selected_project_id.as_ref() == Some(&project.id);
                    let project_id = project.id.clone();
                    let modal = menu_modal.clone();
                    menu.push_item(
                        ContextMenuEntry::new(project.label.clone())
                            .toggleable(IconPosition::End, is_selected)
                            .handler(move |_, cx| {
                                modal
                                    .update(cx, |this, cx| {
                                        this.select_project(project_id.clone(), cx);
                                    })
                                    .ok();
                            }),
                    );
                }
                menu
            }
        });

        self.render_selector_menu(
            "runtime-project-selector",
            label,
            menu,
            self.loading
                || self
                    .catalog
                    .as_ref()
                    .is_none_or(|catalog| catalog.projects.is_empty()),
        )
    }

    fn render_target_dropdown(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let label = self
            .selected_target()
            .map(|target| target.label.clone())
            .unwrap_or_else(|| "Select target".to_string());
        let modal = cx.entity().downgrade();

        let targets = self
            .selected_project()
            .map(|project| project.targets.clone())
            .unwrap_or_default();
        let has_targets = !targets.is_empty();
        let selected_target_id = self.selection.target_id.clone();
        let menu_modal = modal.clone();
        let menu = ContextMenu::build(window, cx, move |mut menu, _, _| {
            for target in &targets {
                let is_selected = selected_target_id.as_ref() == Some(&target.id);
                let target_id = target.id.clone();
                let modal = menu_modal.clone();
                menu.push_item(
                    ContextMenuEntry::new(target.label.clone())
                        .toggleable(IconPosition::End, is_selected)
                        .handler(move |_, cx| {
                            modal
                                .update(cx, |this, cx| {
                                    this.select_target(target_id.clone(), cx);
                                })
                                .ok();
                        }),
                );
            }
            menu
        });

        self.render_selector_menu(
            "runtime-target-selector",
            label,
            menu,
            self.loading || !has_targets,
        )
    }

    fn render_device_dropdown(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let label = self
            .selected_device()
            .map(|device| {
                device
                    .os_version
                    .as_ref()
                    .map(|os_version| format!("{} ({os_version})", device.name))
                    .unwrap_or_else(|| device.name.clone())
            })
            .unwrap_or_else(|| "Select device".to_string());
        let modal = cx.entity().downgrade();

        let devices = self
            .selected_project()
            .map(|project| project.devices.clone())
            .unwrap_or_default();
        let has_devices = !devices.is_empty();
        let selected_device_id = self.selection.device_id.clone();
        let menu_modal = modal.clone();
        let menu = ContextMenu::build(window, cx, move |mut menu, _, _| {
            for device in &devices {
                let item_label = device
                    .os_version
                    .as_ref()
                    .map(|os_version| format!("{} ({os_version})", device.name))
                    .unwrap_or_else(|| device.name.clone());
                let is_selected = selected_device_id.as_ref() == Some(&device.id);
                let device_id = device.id.clone();
                let modal = menu_modal.clone();
                menu.push_item(
                    ContextMenuEntry::new(item_label)
                        .toggleable(IconPosition::End, is_selected)
                        .handler(move |_, cx| {
                            modal
                                .update(cx, |this, cx| {
                                    this.select_device(device_id.clone(), cx);
                                })
                                .ok();
                        }),
                );
            }
            menu
        });

        self.render_selector_menu(
            "runtime-device-selector",
            label,
            menu,
            self.loading || !has_devices,
        )
    }

    fn render_selector_menu(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        menu: Entity<ContextMenu>,
        disabled: bool,
    ) -> AnyElement {
        let id = id.to_string();
        let label = label.into();

        if disabled {
            return RuntimeSelectorMenuTrigger::new(id.clone(), label)
                .disabled(true)
                .into_any_element();
        }

        PopoverMenu::new(format!("{id}-popover"))
            .full_width(true)
            .window_overlay()
            .menu(move |_window, _cx| Some(menu.clone()))
            .trigger(RuntimeSelectorMenuTrigger::new(id, label))
            .attach(Corner::BottomLeft)
            .anchor(Corner::TopLeft)
            .offset(point(px(0.), px(4.)))
            .into_any_element()
    }

    fn render_empty_state(&self) -> Option<AnyElement> {
        match self.selected_project() {
            None if self.loading => Some(
                Label::new("Detecting runtime capabilities…")
                    .size(LabelSize::Small)
                    .color(Color::Muted)
                    .into_any_element(),
            ),
            None => Some(
                Label::new("No runnable project was detected in this workspace.")
                    .size(LabelSize::Small)
                    .color(Color::Muted)
                    .into_any_element(),
            ),
            Some(_) => None,
        }
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let build_reason = self.action_reason(RuntimeAction::Build);
        let run_reason = self.action_reason(RuntimeAction::Run);

        h_flex()
            .gap_2()
            .child(
                RuntimeActionButton::new(
                    "runtime-build",
                    "Build",
                    RuntimeActionButtonStyle::Secondary,
                )
                .disabled(self.loading || build_reason.is_some())
                .on_click(cx.listener(|this, _, window, cx| {
                    this.run_action(RuntimeAction::Build, window, cx);
                })),
            )
            .child(
                RuntimeActionButton::new("runtime-run", "Run", RuntimeActionButtonStyle::Primary)
                    .disabled(self.loading || run_reason.is_some())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.run_action(RuntimeAction::Run, window, cx);
                    })),
            )
    }
}

#[derive(IntoElement)]
struct RuntimeActionButton {
    div: Stateful<gpui::Div>,
    label: SharedString,
    style: RuntimeActionButtonStyle,
    disabled: bool,
}

impl RuntimeActionButton {
    fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        style: RuntimeActionButtonStyle,
    ) -> Self {
        Self {
            div: div().id(id.into()),
            label: label.into(),
            style,
            disabled: false,
        }
    }
}

impl Clickable for RuntimeActionButton {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.div = self.div.on_click(handler);
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.div = self.div.cursor(cursor_style);
        self
    }
}

impl Disableable for RuntimeActionButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for RuntimeActionButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let radius = theme.component_radius().tab.unwrap_or(px(6.0));
        let (text_color, background, border_color, hover_background, active_background) =
            match self.style {
                RuntimeActionButtonStyle::Secondary => (
                    if self.disabled {
                        theme.colors().text_muted.opacity(0.6)
                    } else {
                        theme.colors().text_muted
                    },
                    theme.colors().tab_inactive_background.opacity(0.0),
                    theme.colors().border_variant,
                    theme.colors().ghost_element_hover,
                    theme.colors().element_active,
                ),
                RuntimeActionButtonStyle::Primary => (
                    if self.disabled {
                        theme.colors().text_muted.opacity(0.6)
                    } else {
                        theme.colors().text
                    },
                    theme.colors().text.opacity(0.14),
                    theme.colors().text.opacity(0.0),
                    theme.colors().text.opacity(0.18),
                    theme.colors().text.opacity(0.22),
                ),
            };

        self.div
            .h(px(28.))
            .px_2()
            .border_1()
            .border_color(border_color)
            .bg(background)
            .rounded(radius)
            .when(!self.disabled, |this| {
                this.hover(move |style| style.bg(hover_background))
                    .active(move |style| style.bg(active_background))
                    .cursor_pointer()
            })
            .when(self.disabled, |this| this.opacity(0.5))
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    .justify_center()
                    .text_color(text_color)
                    .child(Label::new(self.label).size(LabelSize::Small)),
            )
    }
}

#[derive(IntoElement)]
struct RuntimeSelectorMenuTrigger {
    div: Stateful<gpui::Div>,
    label: SharedString,
    selected: bool,
    disabled: bool,
}

impl RuntimeSelectorMenuTrigger {
    fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            div: div().id(id.into()),
            label: label.into(),
            selected: false,
            disabled: false,
        }
    }
}

impl Clickable for RuntimeSelectorMenuTrigger {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.div = self.div.on_click(handler);
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.div = self.div.cursor(cursor_style);
        self
    }
}

impl Disableable for RuntimeSelectorMenuTrigger {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for RuntimeSelectorMenuTrigger {
    fn toggle_state(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl RenderOnce for RuntimeSelectorMenuTrigger {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let radius = theme.component_radius().tab.unwrap_or(px(6.0));
        let (text_color, background, hover_background, active_background) = match self.selected {
            false => (
                if self.disabled {
                    theme.colors().text_muted.opacity(0.6)
                } else {
                    theme.colors().text_muted
                },
                theme.colors().tab_inactive_background.opacity(0.0),
                theme.colors().text.opacity(0.09),
                theme.colors().text.opacity(0.14),
            ),
            true => (
                theme.colors().text,
                theme.colors().text.opacity(0.14),
                theme.colors().text.opacity(0.14),
                theme.colors().text.opacity(0.20),
            ),
        };

        self.div
            .w_full()
            .h(px(28.))
            .bg(background)
            .rounded(radius)
            .when(!self.selected && !self.disabled, |this| {
                this.hover(move |style| style.bg(hover_background))
            })
            .when(!self.disabled, |this| {
                this.active(move |style| style.bg(active_background))
                    .cursor_pointer()
            })
            .when(self.disabled, |this| this.opacity(0.5))
            .child(
                h_flex()
                    .w_full()
                    .h_full()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .gap_2()
                    .text_color(text_color)
                    .child(Label::new(self.label).size(LabelSize::Small).truncate())
                    .child(
                        Icon::new(IconName::ChevronUpDown)
                            .size(IconSize::XSmall)
                            .color(Color::Muted),
                    ),
            )
    }
}

impl EventEmitter<DismissEvent> for RuntimeActionsModal {}

impl Focusable for RuntimeActionsModal {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ModalView for RuntimeActionsModal {}

impl Render for RuntimeActionsModal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("RuntimeActionsModal")
            .occlude()
            .elevation_3(cx)
            .w(rems(42.))
            .on_action(cx.listener(Self::cancel))
            .track_focus(&self.focus_handle)
            .child(
                Modal::new("runtime-actions-modal", None::<gpui::ScrollHandle>)
                    .header(
                        ModalHeader::new()
                            .headline("Runtime")
                            .show_dismiss_button(true),
                    )
                    .child(
                        v_flex()
                            .gap_3()
                            .p_3()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_start()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .flex_1()
                                            .child(
                                                Label::new("Project")
                                                    .size(LabelSize::Small)
                                                    .color(Color::Muted),
                                            )
                                            .child(self.render_project_dropdown(window, cx)),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .flex_1()
                                            .child(
                                                Label::new("Target")
                                                    .size(LabelSize::Small)
                                                    .color(Color::Muted),
                                            )
                                            .child(self.render_target_dropdown(window, cx)),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .flex_1()
                                            .child(
                                                Label::new("Device")
                                                    .size(LabelSize::Small)
                                                    .color(Color::Muted),
                                            )
                                            .child(self.render_device_dropdown(window, cx)),
                                    ),
                            )
                            .when_some(self.render_empty_state(), |this, empty_state| {
                                this.child(empty_state)
                            }),
                    )
                    .footer(ModalFooter::new().end_slot(self.render_footer(cx))),
            )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RuntimeSelectionState {
    project_id: Option<String>,
    target_id: Option<String>,
    device_id: Option<String>,
}

fn choose_initial_selection(catalog: &RuntimeCatalog) -> RuntimeSelectionState {
    catalog
        .projects
        .first()
        .map(selection_for_project)
        .unwrap_or_default()
}

fn selection_for_project(project: &app_runtime::DetectedProject) -> RuntimeSelectionState {
    RuntimeSelectionState {
        project_id: Some(project.id.clone()),
        target_id: project.targets.first().map(|target| target.id.clone()),
        device_id: project.devices.first().map(|device| device.id.clone()),
    }
}

fn selected_project<'a>(
    catalog: &'a RuntimeCatalog,
    selection: &RuntimeSelectionState,
) -> Option<&'a app_runtime::DetectedProject> {
    selection.project_id.as_ref().and_then(|project_id| {
        catalog
            .projects
            .iter()
            .find(|project| &project.id == project_id)
    })
}

fn select_project(
    catalog: &RuntimeCatalog,
    selection: &mut RuntimeSelectionState,
    project_id: String,
) {
    if let Some(project) = catalog
        .projects
        .iter()
        .find(|project| project.id == project_id)
    {
        *selection = selection_for_project(project);
    }
}

fn select_target(
    project: &app_runtime::DetectedProject,
    selection: &mut RuntimeSelectionState,
    target_id: String,
) {
    if project.targets.iter().any(|target| target.id == target_id) {
        selection.target_id = Some(target_id);
        if selection.device_id.is_none() && !project.devices.is_empty() {
            selection.device_id = project.devices.first().map(|device| device.id.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use app_runtime::{
        CapabilityState, DetectedProject, ProjectKind, RuntimeCapabilitySet, RuntimeDevice,
        RuntimeDeviceKind, RuntimeDeviceState, RuntimeTarget,
    };

    use super::{
        RuntimeSelectionState, choose_initial_selection, select_project, select_target,
        selection_for_project,
    };

    fn project(id: &str, targets: &[&str], devices: &[&str]) -> DetectedProject {
        DetectedProject {
            id: id.to_string(),
            label: id.to_string(),
            kind: ProjectKind::GpuiApplication,
            workspace_root: std::path::PathBuf::from(format!("/tmp/{id}")),
            project_path: std::path::PathBuf::from(format!("/tmp/{id}/Cargo.toml")),
            targets: targets
                .iter()
                .map(|target| RuntimeTarget {
                    id: (*target).to_string(),
                    label: (*target).to_string(),
                })
                .collect(),
            devices: devices
                .iter()
                .map(|device| RuntimeDevice {
                    id: (*device).to_string(),
                    name: (*device).to_string(),
                    kind: RuntimeDeviceKind::Desktop,
                    state: RuntimeDeviceState::Unknown,
                    os_version: None,
                })
                .collect(),
            capabilities: RuntimeCapabilitySet {
                run: CapabilityState::Available,
                build: CapabilityState::Available,
            },
        }
    }

    #[test]
    fn chooses_initial_selection_from_first_project() {
        let catalog = app_runtime::RuntimeCatalog {
            projects: vec![
                project("alpha", &["app"], &["mac"]),
                project("beta", &["tool"], &[]),
            ],
        };

        let selection = choose_initial_selection(&catalog);

        assert_eq!(
            selection,
            RuntimeSelectionState {
                project_id: Some("alpha".to_string()),
                target_id: Some("app".to_string()),
                device_id: Some("mac".to_string()),
            }
        );
    }

    #[test]
    fn resets_target_and_device_when_project_changes() {
        let catalog = app_runtime::RuntimeCatalog {
            projects: vec![
                project("alpha", &["app"], &["mac"]),
                project("beta", &["tool"], &[]),
            ],
        };
        let mut selection = selection_for_project(&catalog.projects[0]);

        select_project(&catalog, &mut selection, "beta".to_string());

        assert_eq!(selection.project_id.as_deref(), Some("beta"));
        assert_eq!(selection.target_id.as_deref(), Some("tool"));
        assert!(selection.device_id.is_none());
    }

    #[test]
    fn keeps_first_device_available_when_target_changes() {
        let project = project("alpha", &["app", "worker"], &["mac"]);
        let mut selection = selection_for_project(&project);
        selection.device_id = None;

        select_target(&project, &mut selection, "worker".to_string());

        assert_eq!(selection.target_id.as_deref(), Some("worker"));
        assert_eq!(selection.device_id.as_deref(), Some("mac"));
    }
}
