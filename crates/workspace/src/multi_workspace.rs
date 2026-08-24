use agent_settings::AgentSettings;
use anyhow::Result;
use gpui::PathPromptOptions;
#[cfg(target_os = "macos")]
use gpui::native_sidebar;
#[cfg(not(target_os = "macos"))]
use gpui::px;
use gpui::{
    AnyView, App, Context, DragMoveEvent, Entity, EntityId, EventEmitter, FocusHandle, Focusable,
    ManagedView, Pixels, Render, Subscription, Task, Tiling, Window, WindowId, actions,
};
use gpui::{MouseButton, deferred};
use project::{DirectoryLister, Project, ProjectGroupKey};
use remote::RemoteConnectionOptions;
use settings::Settings;
use settings::SidebarDockPosition;
pub use settings::SidebarSide;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use theme::ActiveTheme;
use ui::prelude::*;
use ui::{ContextMenu, right_click_menu};
use util::ResultExt;
use util::path_list::PathList;
use workspace_modes::{ModeId, ModeViewRegistry, RegisteredModeView};
use zed_actions::agents_sidebar::ToggleThreadSwitcher;

#[cfg(not(target_os = "macos"))]
pub const SIDEBAR_RESIZE_HANDLE_SIZE: Pixels = px(6.0);

use crate::WorkspaceSidebarHost;
use crate::open_remote_project_with_existing_connection;
use crate::{
    CloseIntent, CloseWindow, DockPosition, Event as WorkspaceEvent, Item, ModalView, OpenMode,
    Panel, Workspace, WorkspaceId, client_side_decorations,
    persistence::model::MultiWorkspaceState,
};

actions!(
    multi_workspace,
    [
        /// Creates a new workspace within the current window.
        NewWorkspaceInWindow,
        /// Switches to the next workspace within the current window.
        NextWorkspaceInWindow,
        /// Switches to the previous workspace within the current window.
        PreviousWorkspaceInWindow,
        /// Toggles project navigation.
        ToggleProjectNavigation,
        /// Closes project navigation.
        CloseProjectNavigation,
        /// Moves focus to or from project navigation without closing it.
        FocusProjectNavigation,
        /// Toggles the workspace sidebar.
        ToggleWorkspaceSidebar,
        /// Closes the workspace sidebar.
        CloseWorkspaceSidebar,
        /// Moves focus to or from the workspace sidebar without closing it.
        FocusWorkspaceSidebar,
        /// Switches to the next workspace.
        NextWorkspace,
        /// Switches to the previous workspace.
        PreviousWorkspace,
    ]
);

#[derive(Default)]
pub struct SidebarRenderState {
    pub open: bool,
    pub side: SidebarSide,
}

pub fn sidebar_side_context_menu(
    id: impl Into<ElementId>,
    cx: &App,
) -> ui::RightClickMenu<ContextMenu> {
    let current_position = AgentSettings::get_global(cx).sidebar_side;
    right_click_menu(id).menu(move |window, cx| {
        let fs = <dyn fs::Fs>::global(cx);
        ContextMenu::build(window, cx, move |mut menu, _, _cx| {
            let positions: [(SidebarDockPosition, &str); 2] = [
                (SidebarDockPosition::Left, "Left"),
                (SidebarDockPosition::Right, "Right"),
            ];
            for (position, label) in positions {
                let fs = fs.clone();
                menu = menu.toggleable_entry(
                    label,
                    position == current_position,
                    IconPosition::Start,
                    None,
                    move |_window, cx| {
                        settings::update_settings_file(fs.clone(), cx, move |settings, _cx| {
                            settings
                                .agent
                                .get_or_insert_default()
                                .set_sidebar_side(position);
                        });
                    },
                );
            }
            menu
        })
    })
}

pub enum MultiWorkspaceEvent {
    ActiveWorkspaceChanged,
    WorkspaceAdded(Entity<Workspace>),
    WorkspaceRemoved(EntityId),
}

pub enum SidebarEvent {
    SerializeNeeded,
}

pub trait Sidebar: Focusable + Render + EventEmitter<SidebarEvent> + Sized {
    fn width(&self, cx: &App) -> Pixels;
    fn set_width(&mut self, width: Option<Pixels>, cx: &mut Context<Self>);
    fn has_notifications(&self, cx: &App) -> bool;
    fn side(&self, _cx: &App) -> SidebarSide {
        SidebarSide::Left
    }

    fn is_threads_list_view_active(&self) -> bool {
        true
    }
    fn show_project_files(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}
    fn show_project_threads(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}
    /// Makes focus reset back to the search editor upon toggling the sidebar from outside
    fn prepare_for_focus(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}
    /// Opens or cycles the thread switcher popup.
    fn toggle_thread_switcher(
        &mut self,
        _select_last: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    /// Return an opaque JSON blob of sidebar-specific state to persist.
    fn serialized_state(&self, _cx: &App) -> Option<String> {
        None
    }

    /// Restore sidebar state from a previously-serialized blob.
    fn restore_serialized_state(
        &mut self,
        _state: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

pub trait SidebarHandle: 'static + Send + Sync {
    fn width(&self, cx: &App) -> Pixels;
    fn set_width(&self, width: Option<Pixels>, cx: &mut App);
    fn focus_handle(&self, cx: &App) -> FocusHandle;
    fn focus(&self, window: &mut Window, cx: &mut App);
    fn prepare_for_focus(&self, window: &mut Window, cx: &mut App);
    fn show_project_files(&self, window: &mut Window, cx: &mut App);
    fn show_project_threads(&self, window: &mut Window, cx: &mut App);
    fn has_notifications(&self, cx: &App) -> bool;
    fn to_any(&self) -> AnyView;
    fn entity_id(&self) -> EntityId;
    fn toggle_thread_switcher(&self, select_last: bool, window: &mut Window, cx: &mut App);

    fn is_threads_list_view_active(&self, cx: &App) -> bool;
    fn side(&self, cx: &App) -> SidebarSide;
    fn serialized_state(&self, cx: &App) -> Option<String>;
    fn restore_serialized_state(&self, state: &str, window: &mut Window, cx: &mut App);
}

#[derive(Clone)]
pub struct DraggedSidebar;

impl Render for DraggedSidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui::Empty
    }
}

impl<T: Sidebar> SidebarHandle for Entity<T> {
    fn width(&self, cx: &App) -> Pixels {
        self.read(cx).width(cx)
    }

    fn set_width(&self, width: Option<Pixels>, cx: &mut App) {
        self.update(cx, |this, cx| this.set_width(width, cx))
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.read(cx).focus_handle(cx)
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        let handle = self.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
    }

    fn prepare_for_focus(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| this.prepare_for_focus(window, cx));
    }

    fn show_project_files(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| this.show_project_files(window, cx));
    }

    fn show_project_threads(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| this.show_project_threads(window, cx));
    }

    fn has_notifications(&self, cx: &App) -> bool {
        self.read(cx).has_notifications(cx)
    }

    fn to_any(&self) -> AnyView {
        self.clone().into()
    }

    fn entity_id(&self) -> EntityId {
        Entity::entity_id(self)
    }

    fn toggle_thread_switcher(&self, select_last: bool, window: &mut Window, cx: &mut App) {
        let entity = self.clone();
        window.defer(cx, move |window, cx| {
            entity.update(cx, |this, cx| {
                this.toggle_thread_switcher(select_last, window, cx);
            });
        });
    }

    fn is_threads_list_view_active(&self, cx: &App) -> bool {
        self.read(cx).is_threads_list_view_active()
    }

    fn side(&self, cx: &App) -> SidebarSide {
        self.read(cx).side(cx)
    }

    fn serialized_state(&self, cx: &App) -> Option<String> {
        self.read(cx).serialized_state(cx)
    }

    fn restore_serialized_state(&self, state: &str, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| {
            this.restore_serialized_state(state, window, cx)
        })
    }
}

pub struct MultiWorkspace {
    window_id: WindowId,
    workspaces: Vec<Entity<Workspace>>,
    active_workspace_index: usize,
    project_group_keys: Vec<ProjectGroupKey>,
    provisional_project_group_keys: HashMap<EntityId, ProjectGroupKey>,
    sidebar: Option<Box<dyn SidebarHandle>>,
    workspace_sidebar_host: Entity<WorkspaceSidebarHost>,
    sidebar_open: bool,
    sidebar_has_notifications: bool,
    sidebar_overlay: Option<AnyView>,
    pending_removal_tasks: Vec<Task<()>>,
    _serialize_task: Option<Task<()>>,
    _create_task: Option<Task<()>>,
    shared_mode_views: collections::HashMap<ModeId, RegisteredModeView>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<MultiWorkspaceEvent> for MultiWorkspace {}

pub fn multi_workspace_enabled(_cx: &App) -> bool {
    true
}

impl MultiWorkspace {
    pub fn new(workspace: Entity<Workspace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let shared_mode_views = Self::shared_mode_views(cx);
        for (mode_id, mode_view) in &shared_mode_views {
            workspace.update(cx, |workspace, cx| {
                workspace.set_shared_mode_view(*mode_id, mode_view.clone(), cx);
            });
        }

        let release_subscription = cx.on_release(|this: &mut MultiWorkspace, _cx| {
            if let Some(task) = this._serialize_task.take() {
                task.detach();
            }
            for task in std::mem::take(&mut this.pending_removal_tasks) {
                task.detach();
            }
        });
        let quit_subscription = cx.on_app_quit(Self::app_will_quit);
        Self::subscribe_to_workspace(&workspace, cx);
        let workspace_sidebar_host = {
            let left_dock = workspace.read(cx).left_dock().clone();
            let bottom_dock = workspace.read(cx).bottom_dock().clone();
            let right_dock = workspace.read(cx).right_dock().clone();
            cx.new(|_cx| WorkspaceSidebarHost::new(left_dock, bottom_dock, right_dock))
        };
        Self {
            window_id: window.window_handle().window_id(),
            project_group_keys: vec![workspace.read(cx).project_group_key(cx)],
            workspaces: vec![workspace],
            active_workspace_index: 0,
            provisional_project_group_keys: HashMap::default(),
            sidebar: None,
            workspace_sidebar_host,
            sidebar_open: false,
            sidebar_has_notifications: false,
            sidebar_overlay: None,
            pending_removal_tasks: Vec::new(),
            _serialize_task: None,
            _create_task: None,
            shared_mode_views,
            _subscriptions: vec![release_subscription, quit_subscription],
        }
    }

    fn shared_mode_views(cx: &mut App) -> collections::HashMap<ModeId, RegisteredModeView> {
        let mut views = collections::HashMap::default();

        if let Some(mode_view) = Self::create_shared_mode_view(ModeId::BROWSER, cx) {
            views.insert(ModeId::BROWSER, mode_view);
        }

        views
    }

    fn create_shared_mode_view(mode_id: ModeId, cx: &mut App) -> Option<RegisteredModeView> {
        if let Some(factory) = ModeViewRegistry::try_global(cx)
            .and_then(|registry| registry.factory(mode_id))
            .cloned()
        {
            return Some(factory(cx));
        }

        ModeViewRegistry::try_global(cx)
            .and_then(|registry| registry.get(mode_id))
            .cloned()
    }

    pub fn workspace_sidebar_host(&self) -> &Entity<WorkspaceSidebarHost> {
        &self.workspace_sidebar_host
    }

    pub fn register_sidebar<T: Sidebar>(&mut self, sidebar: Entity<T>, cx: &mut Context<Self>) {
        self._subscriptions
            .push(cx.observe(&sidebar, |_this, _, cx| {
                cx.notify();
            }));
        self._subscriptions
            .push(cx.subscribe(&sidebar, |this, _, event, cx| match event {
                SidebarEvent::SerializeNeeded => {
                    this.serialize(cx);
                }
            }));
        self.sidebar = Some(Box::new(sidebar));
    }

    pub fn sidebar(&self) -> Option<&dyn SidebarHandle> {
        self.sidebar.as_deref()
    }

    pub fn sidebar_side(&self, cx: &App) -> SidebarSide {
        self.sidebar
            .as_ref()
            .map_or(SidebarSide::Left, |sidebar| sidebar.side(cx))
    }

    pub fn sidebar_render_state(&self, cx: &App) -> SidebarRenderState {
        SidebarRenderState {
            open: self.sidebar_open() && self.multi_workspace_enabled(cx),
            side: self.sidebar_side(cx),
        }
    }

    pub fn sidebar_has_notifications(&self, cx: &App) -> bool {
        self.sidebar_has_notifications && multi_workspace_enabled(cx)
    }

    pub fn set_sidebar_overlay(&mut self, overlay: Option<AnyView>, cx: &mut Context<Self>) {
        self.sidebar_overlay = overlay;
        cx.notify();
    }

    pub fn sidebar_open(&self) -> bool {
        self.sidebar_open
    }

    pub fn is_sidebar_open(&self) -> bool {
        self.sidebar_open
    }

    pub fn set_sidebar_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.sidebar_open == open {
            return;
        }

        self.sidebar_open = open;
        cx.notify();
    }

    pub fn set_sidebar_has_notifications(
        &mut self,
        has_notifications: bool,
        cx: &mut Context<Self>,
    ) {
        if self.sidebar_has_notifications == has_notifications {
            return;
        }

        self.sidebar_has_notifications = has_notifications;
        cx.notify();
    }
    pub fn is_threads_list_view_active(&self, cx: &App) -> bool {
        self.sidebar
            .as_ref()
            .map_or(false, |s| s.is_threads_list_view_active(cx))
    }

    pub fn multi_workspace_enabled(&self, cx: &App) -> bool {
        multi_workspace_enabled(cx)
    }

    pub fn toggle_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sidebar_open {
            self.close_sidebar(window, cx);
        } else {
            self.open_sidebar(cx);
            if let Some(sidebar) = &self.sidebar {
                sidebar.prepare_for_focus(window, cx);
                sidebar.focus(window, cx);
            }
        }
    }

    pub fn close_sidebar_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.multi_workspace_enabled(cx) {
            return;
        }

        if self.sidebar_open {
            self.close_sidebar(window, cx);
        }
    }

    pub fn focus_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sidebar_open {
            let sidebar_is_focused = self
                .sidebar
                .as_ref()
                .is_some_and(|sidebar| sidebar.focus_handle(cx).contains_focused(window, cx));

            if sidebar_is_focused {
                let pane = self.workspace().read(cx).active_pane().clone();
                let pane_focus = pane.read(cx).focus_handle(cx);
                window.focus(&pane_focus, cx);
            } else if let Some(sidebar) = &self.sidebar {
                sidebar.prepare_for_focus(window, cx);
                sidebar.focus(window, cx);
            }
        } else {
            self.open_sidebar(cx);
            if let Some(sidebar) = &self.sidebar {
                sidebar.prepare_for_focus(window, cx);
                sidebar.focus(window, cx);
            }
        }
    }

    pub fn open_sidebar(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_open {
            return;
        }

        self.sidebar_open = true;
        self.serialize(cx);
        cx.notify();
    }

    pub fn close_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sidebar_open {
            return;
        }

        self.sidebar_open = false;
        let pane = self.workspace().read(cx).active_pane().clone();
        let pane_focus = pane.read(cx).focus_handle(cx);
        window.focus(&pane_focus, cx);
        self.serialize(cx);
        cx.notify();
    }

    pub fn close_window(&mut self, _: &CloseWindow, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let workspaces = this.update(cx, |multi_workspace, _cx| {
                multi_workspace.workspaces().to_vec()
            })?;

            for workspace in workspaces {
                let should_continue = workspace
                    .update_in(cx, |workspace, window, cx| {
                        workspace.prepare_to_close(CloseIntent::CloseWindow, window, cx)
                    })?
                    .await?;
                if !should_continue {
                    return anyhow::Ok(());
                }
            }

            cx.update(|window, _cx| {
                window.remove_window();
            })?;

            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn subscribe_to_workspace(workspace: &Entity<Workspace>, cx: &mut Context<Self>) {
        let project = workspace.read(cx).project().clone();
        cx.subscribe(&project, {
            let workspace = workspace.downgrade();
            move |this, _project, event: &project::Event, cx| match event {
                project::Event::WorktreeAdded(_) | project::Event::WorktreeRemoved(_) => {
                    if let Some(workspace) = workspace.upgrade() {
                        this.add_project_group_key(workspace.read(cx).project_group_key(cx));
                    }
                }
                project::Event::WorktreeUpdatedRootRepoCommonDir(_) => {
                    if let Some(workspace) = workspace.upgrade() {
                        this.maybe_clear_provisional_project_group_key(&workspace, cx);
                        this.add_project_group_key(
                            this.project_group_key_for_workspace(&workspace, cx),
                        );
                        this.remove_stale_project_group_keys(cx);
                        cx.notify();
                    }
                }
                _ => {}
            }
        })
        .detach();

        cx.observe(workspace, |_this, _observed_workspace, cx| {
            if *_this.workspace() == _observed_workspace {
                _this.sync_workspace_sidebar_host(cx);
            }
            cx.notify();
        })
        .detach();

        cx.subscribe(workspace, |this, workspace, event, cx| {
            if let WorkspaceEvent::Activate = event {
                this.set_active_workspace(workspace.clone(), cx);
                this.serialize(cx);
            }
        })
        .detach();
    }

    /// Sync the shared unified sidebar to point at the active workspace's left dock,
    /// selected section, dock roots, and hosted sidebar view. Width is NOT synced because the
    /// sidebar host manages its own divider position independently.
    fn sync_workspace_sidebar_host(&self, cx: &mut App) {
        let active_ws = self.workspace().clone();
        let (workspace_sidebar_surface, workspace_sidebar_view) = {
            let workspace = active_ws.read(cx);
            let workspace_sidebar_view = self.sidebar.as_ref().map(|sidebar| sidebar.to_any());
            (
                workspace.workspace_sidebar_host.read(cx).surface(),
                workspace_sidebar_view,
            )
        };
        self.workspace_sidebar_host.update(cx, |sidebar, cx| {
            sidebar.apply_surface(&workspace_sidebar_surface, workspace_sidebar_view, cx);
        });
    }

    pub fn add_project_group_key(&mut self, project_group_key: ProjectGroupKey) {
        if project_group_key.path_list().paths().is_empty() {
            return;
        }
        if self.project_group_keys.contains(&project_group_key) {
            return;
        }
        self.project_group_keys.push(project_group_key);
    }

    pub fn set_provisional_project_group_key(
        &mut self,
        workspace: &Entity<Workspace>,
        project_group_key: ProjectGroupKey,
    ) {
        self.provisional_project_group_keys
            .insert(workspace.entity_id(), project_group_key.clone());
        self.add_project_group_key(project_group_key);
    }

    pub(crate) fn set_workspace_group_key(
        &mut self,
        workspace: &Entity<Workspace>,
        project_group_key: ProjectGroupKey,
    ) {
        self.set_provisional_project_group_key(workspace, project_group_key);
    }

    pub fn project_group_key_for_workspace(
        &self,
        workspace: &Entity<Workspace>,
        cx: &App,
    ) -> ProjectGroupKey {
        self.provisional_project_group_keys
            .get(&workspace.entity_id())
            .cloned()
            .unwrap_or_else(|| workspace.read(cx).project_group_key(cx))
    }

    fn maybe_clear_provisional_project_group_key(
        &mut self,
        workspace: &Entity<Workspace>,
        cx: &App,
    ) {
        let live_key = workspace.read(cx).project_group_key(cx);
        if self
            .provisional_project_group_keys
            .get(&workspace.entity_id())
            .is_some_and(|key| *key == live_key)
        {
            self.provisional_project_group_keys
                .remove(&workspace.entity_id());
        }
    }

    fn remove_stale_project_group_keys(&mut self, cx: &App) {
        let workspace_keys: HashSet<ProjectGroupKey> = self
            .workspaces
            .iter()
            .map(|workspace| self.project_group_key_for_workspace(workspace, cx))
            .collect();
        self.project_group_keys
            .retain(|key| workspace_keys.contains(key));
    }

    pub fn restore_project_group_keys(&mut self, keys: Vec<ProjectGroupKey>) {
        let mut restored = keys;
        for existing_key in &self.project_group_keys {
            if !restored.contains(existing_key) {
                restored.push(existing_key.clone());
            }
        }
        self.project_group_keys = restored;
    }

    pub fn project_group_keys(&self) -> impl Iterator<Item = &ProjectGroupKey> {
        self.project_group_keys.iter()
    }

    /// Returns the project groups, ordered by most recently added.
    pub fn project_groups(
        &self,
        cx: &App,
    ) -> impl Iterator<Item = (ProjectGroupKey, Vec<Entity<Workspace>>)> {
        let mut groups = self
            .project_group_keys
            .iter()
            .rev()
            .map(|key| (key.clone(), Vec::new()))
            .collect::<Vec<_>>();
        for workspace in &self.workspaces {
            let key = self.project_group_key_for_workspace(workspace, cx);
            if let Some((_, workspaces)) = groups.iter_mut().find(|(k, _)| k == &key) {
                workspaces.push(workspace.clone());
            }
        }
        groups.into_iter()
    }

    pub fn workspaces_for_project_group(
        &self,
        project_group_key: &ProjectGroupKey,
        cx: &App,
    ) -> impl Iterator<Item = &Entity<Workspace>> {
        self.workspaces.iter().filter(move |workspace| {
            self.project_group_key_for_workspace(workspace, cx) == *project_group_key
        })
    }

    pub fn remove_folder_from_project_group(
        &mut self,
        project_group_key: &ProjectGroupKey,
        path: &Path,
        cx: &mut Context<Self>,
    ) {
        let new_path_list = project_group_key.path_list().without_path(path);
        if new_path_list.is_empty() {
            return;
        }

        let new_key = ProjectGroupKey::new(project_group_key.host(), new_path_list);

        let workspaces: Vec<_> = self
            .workspaces_for_project_group(project_group_key, cx)
            .cloned()
            .collect();

        self.add_project_group_key(new_key);

        for workspace in workspaces {
            let project = workspace.read(cx).project().clone();
            project.update(cx, |project, cx| {
                project.remove_worktree_for_main_worktree_path(path, cx);
            });
        }

        self.serialize(cx);
        cx.notify();
    }

    pub fn prompt_to_add_folders_to_project_group(
        &mut self,
        key: &ProjectGroupKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let paths = self.workspace().update(cx, |workspace, cx| {
            workspace.prompt_for_open_path(
                PathPromptOptions {
                    files: false,
                    directories: true,
                    multiple: true,
                    prompt: None,
                },
                DirectoryLister::Project(workspace.project().clone()),
                window,
                cx,
            )
        });

        let key = key.clone();
        cx.spawn_in(window, async move |this, cx| {
            if let Some(new_paths) = paths.await.ok().flatten() {
                if !new_paths.is_empty() {
                    this.update(cx, |multi_workspace, cx| {
                        multi_workspace.add_folders_to_project_group(&key, new_paths, cx);
                    })?;
                }
            }
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    pub fn add_folders_to_project_group(
        &mut self,
        project_group_key: &ProjectGroupKey,
        new_paths: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let mut all_paths: Vec<PathBuf> = project_group_key.path_list().paths().to_vec();
        all_paths.extend(new_paths.iter().cloned());
        let new_path_list = PathList::new(&all_paths);
        let new_key = ProjectGroupKey::new(project_group_key.host(), new_path_list);

        let workspaces: Vec<_> = self
            .workspaces_for_project_group(project_group_key, cx)
            .cloned()
            .collect();

        self.add_project_group_key(new_key);

        for workspace in workspaces {
            let project = workspace.read(cx).project().clone();
            for path in &new_paths {
                project
                    .update(cx, |project, cx| {
                        project.find_or_create_worktree(path, true, cx)
                    })
                    .detach_and_log_err(cx);
            }
        }

        self.serialize(cx);
        cx.notify();
    }

    pub fn remove_project_group(
        &mut self,
        key: &ProjectGroupKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_group_keys.retain(|k| k != key);

        let workspaces: Vec<_> = self
            .workspaces_for_project_group(key, cx)
            .cloned()
            .collect();
        for workspace in workspaces {
            self.remove(&workspace, window, cx);
        }

        self.serialize(cx);
        cx.notify();
    }

    /// Finds an existing workspace whose root paths and host exactly match.
    pub fn workspace_for_paths(
        &self,
        path_list: &PathList,
        host: Option<&RemoteConnectionOptions>,
        cx: &App,
    ) -> Option<Entity<Workspace>> {
        self.workspaces
            .iter()
            .find(|ws| {
                let key = self.project_group_key_for_workspace(ws, cx);
                key.host().as_ref() == host
                    && PathList::new(&ws.read(cx).root_paths(cx)) == *path_list
            })
            .cloned()
    }

    /// Finds an existing workspace whose paths match, or creates a new one.
    ///
    /// For local projects (`host` is `None`), this delegates to
    /// [`Self::find_or_create_local_workspace`]. For remote projects, it
    /// tries an exact path match and, if no existing workspace is found,
    /// calls `connect_remote` to establish a connection and creates a new
    /// remote workspace.
    ///
    /// The `connect_remote` closure is responsible for any user-facing
    /// connection UI (e.g. password prompts). It receives the connection
    /// options and should return a [`Task`] that resolves to the
    /// [`RemoteClient`] session, or `None` if the connection was
    /// cancelled.
    pub fn find_or_create_workspace(
        &mut self,
        paths: PathList,
        host: Option<RemoteConnectionOptions>,
        provisional_project_group_key: Option<ProjectGroupKey>,
        connect_remote: impl FnOnce(
            RemoteConnectionOptions,
            &mut Window,
            &mut Context<Self>,
        ) -> Task<Result<Option<Entity<remote::RemoteClient>>>>
        + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Result<Entity<Workspace>>> {
        if let Some(workspace) = self.workspace_for_paths(&paths, host.as_ref(), cx) {
            self.activate(workspace.clone(), cx);
            return Task::ready(Ok(workspace));
        }

        let Some(connection_options) = host else {
            return self.find_or_create_local_workspace(paths, window, cx);
        };

        let app_state = self.workspace().read(cx).app_state().clone();
        let window_handle = window.window_handle().downcast::<MultiWorkspace>();
        let connect_task = connect_remote(connection_options.clone(), window, cx);
        let paths_vec = paths.paths().to_vec();

        cx.spawn(async move |_this, cx| {
            let session = connect_task
                .await?
                .ok_or_else(|| anyhow::anyhow!("Remote connection was cancelled"))?;

            let new_project = cx.update(|cx| {
                Project::remote(
                    session,
                    app_state.client.clone(),
                    app_state.node_runtime.clone(),
                    app_state.user_store.clone(),
                    app_state.languages.clone(),
                    app_state.fs.clone(),
                    true,
                    cx,
                )
            });

            let window_handle =
                window_handle.ok_or_else(|| anyhow::anyhow!("Window is not a MultiWorkspace"))?;

            open_remote_project_with_existing_connection(
                connection_options,
                new_project,
                paths_vec,
                app_state,
                window_handle,
                provisional_project_group_key,
                cx,
            )
            .await?;

            window_handle.update(cx, |multi_workspace, _window, _cx| {
                multi_workspace.workspace().clone()
            })
        })
    }

    /// Finds an existing workspace in this multi-workspace whose paths match,
    /// or creates a new one (deserializing its saved state from the database).
    /// Never searches other windows or matches workspaces with a superset of
    /// the requested paths.
    pub fn find_or_create_local_workspace(
        &mut self,
        path_list: PathList,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Result<Entity<Workspace>>> {
        if let Some(workspace) = self.workspace_for_paths(&path_list, None, cx) {
            self.activate(workspace.clone(), cx);
            return Task::ready(Ok(workspace));
        }

        let paths = path_list.paths().to_vec();
        let app_state = self.workspace().read(cx).app_state().clone();
        let requesting_window = window.window_handle().downcast::<MultiWorkspace>();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .update(|cx| {
                    Workspace::new_local(
                        paths,
                        app_state,
                        requesting_window,
                        None,
                        None,
                        OpenMode::Activate,
                        cx,
                    )
                })
                .await?;
            Ok(result.workspace)
        })
    }

    pub fn workspace(&self) -> &Entity<Workspace> {
        &self.workspaces[self.active_workspace_index]
    }

    pub fn workspaces(&self) -> &[Entity<Workspace>] {
        &self.workspaces
    }

    pub fn active_workspace_index(&self) -> usize {
        self.active_workspace_index
    }

    pub fn activate(&mut self, workspace: Entity<Workspace>, cx: &mut Context<Self>) {
        if !multi_workspace_enabled(cx) {
            self.workspaces[0] = workspace;
            self.active_workspace_index = 0;
            cx.emit(MultiWorkspaceEvent::ActiveWorkspaceChanged);
            cx.notify();
            return;
        }
        let old_index = self.active_workspace_index;
        let new_index = self.set_active_workspace(workspace, cx);
        if old_index != new_index {
            self.serialize(cx);
        }
    }

    pub fn activate_in_window(
        &mut self,
        workspace: Entity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !multi_workspace_enabled(cx) {
            self.workspaces[0] = workspace;
            self.active_workspace_index = 0;
            cx.emit(MultiWorkspaceEvent::ActiveWorkspaceChanged);
            self.focus_active_workspace(window, cx);
            cx.notify();
            return;
        }
        let new_index = self.add_workspace(workspace.clone(), cx);
        let changed = self.active_workspace_index != new_index;
        self.active_workspace_index = new_index;
        self.workspace().update(cx, |workspace, cx| {
            workspace.invalidate_window_caches(window, cx);
            cx.notify();
        });
        self.sync_workspace_sidebar_host(cx);
        self.focus_active_workspace(window, cx);
        if changed {
            cx.emit(MultiWorkspaceEvent::ActiveWorkspaceChanged);
            self.serialize(cx);
        }
        cx.notify();
    }

    fn set_active_workspace(
        &mut self,
        workspace: Entity<Workspace>,
        cx: &mut Context<Self>,
    ) -> usize {
        let index = self.add_workspace(workspace.clone(), cx);
        let changed = self.active_workspace_index != index;
        self.active_workspace_index = index;
        if changed {
            cx.emit(MultiWorkspaceEvent::ActiveWorkspaceChanged);
        }
        // Force the workspace to re-render when it becomes active.
        workspace.update(cx, |_, cx| cx.notify());
        cx.notify();
        index
    }

    /// Adds a workspace to this window without changing which workspace is active.
    /// Returns the index of the workspace (existing or newly inserted).
    pub fn add_workspace(&mut self, workspace: Entity<Workspace>, cx: &mut Context<Self>) -> usize {
        if let Some(index) = self.workspaces.iter().position(|w| *w == workspace) {
            index
        } else {
            for (mode_id, mode_view) in &self.shared_mode_views {
                workspace.update(cx, |workspace, cx| {
                    workspace.set_shared_mode_view(*mode_id, mode_view.clone(), cx);
                });
            }
            let project_group_key = self.project_group_key_for_workspace(&workspace, cx);

            Self::subscribe_to_workspace(&workspace, cx);
            self.add_project_group_key(project_group_key);
            self.workspaces.push(workspace.clone());
            cx.emit(MultiWorkspaceEvent::WorkspaceAdded(workspace));
            cx.notify();
            self.workspaces.len() - 1
        }
    }

    pub fn replace(
        &mut self,
        workspace: Entity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.workspaces.iter().position(|w| *w == workspace) {
            self.activate_index(index, window, cx);
            return;
        }

        for (mode_id, mode_view) in &self.shared_mode_views {
            workspace.update(cx, |workspace, cx| {
                workspace.set_shared_mode_view(*mode_id, mode_view.clone(), cx);
            });
        }
        Self::subscribe_to_workspace(&workspace, cx);

        let removed_id = self.workspaces[self.active_workspace_index].entity_id();
        self.workspaces[self.active_workspace_index] = workspace.clone();
        cx.emit(MultiWorkspaceEvent::WorkspaceRemoved(removed_id));
        cx.emit(MultiWorkspaceEvent::WorkspaceAdded(workspace));
        self.workspace().update(cx, |workspace, cx| {
            workspace.invalidate_window_caches(window, cx);
            cx.notify();
        });
        self.sync_workspace_sidebar_host(cx);
        self.serialize(cx);
        self.focus_active_workspace(window, cx);
        cx.emit(MultiWorkspaceEvent::ActiveWorkspaceChanged);
        cx.notify();
    }

    pub fn activate_index(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        debug_assert!(
            index < self.workspaces.len(),
            "workspace index out of bounds"
        );
        let changed = self.active_workspace_index != index;
        self.active_workspace_index = index;
        // Force the workspace to re-render and push its window title/toolbar,
        // which may be stale if this workspace was previously inactive.
        self.workspace().update(cx, |workspace, cx| {
            workspace.invalidate_window_caches(window, cx);
            cx.notify();
        });
        self.sync_workspace_sidebar_host(cx);
        self.serialize(cx);
        self.focus_active_workspace(window, cx);
        if changed {
            cx.emit(MultiWorkspaceEvent::ActiveWorkspaceChanged);
        }
        cx.notify();
    }

    pub fn activate_next_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_workspace(1, window, cx);
    }

    pub fn activate_previous_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_workspace(-1, window, cx);
    }

    fn cycle_workspace(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.workspaces.len() as isize;
        if count <= 1 {
            return;
        }
        let current = self.active_workspace_index as isize;
        let next = ((current + delta).rem_euclid(count)) as usize;
        self.activate_index(next, window, cx);
    }

    fn next_workspace(&mut self, _: &NextWorkspace, window: &mut Window, cx: &mut Context<Self>) {
        self.activate_next_workspace(window, cx);
    }

    fn previous_workspace(
        &mut self,
        _: &PreviousWorkspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_previous_workspace(window, cx);
    }

    pub(crate) fn serialize(&mut self, cx: &mut Context<Self>) {
        self._serialize_task = Some(cx.spawn(async move |this, cx| {
            let Some((window_id, state)) = this
                .read_with(cx, |this, cx| {
                    let state = MultiWorkspaceState {
                        active_workspace_id: this.workspace().read(cx).database_id(),
                        project_group_keys: this
                            .project_group_keys()
                            .cloned()
                            .map(Into::into)
                            .collect::<Vec<_>>(),
                        sidebar_open: this.sidebar_open,
                        sidebar_state: this.sidebar.as_ref().and_then(|s| s.serialized_state(cx)),
                    };
                    (this.window_id, state)
                })
                .ok()
            else {
                return;
            };
            let kvp = cx.update(|cx| db::kvp::KeyValueStore::global(cx));
            crate::persistence::write_multi_workspace_state(&kvp, window_id, state).await;
        }));
    }

    /// Returns the in-flight serialization task (if any) so the caller can
    /// await it. Used by the quit handler to ensure pending DB writes
    /// complete before the process exits.
    pub fn flush_serialization(&mut self) -> Task<()> {
        self._serialize_task.take().unwrap_or(Task::ready(()))
    }

    fn app_will_quit(&mut self, _cx: &mut Context<Self>) -> impl Future<Output = ()> + use<> {
        let mut tasks: Vec<Task<()>> = Vec::new();
        if let Some(task) = self._serialize_task.take() {
            tasks.push(task);
        }
        tasks.extend(std::mem::take(&mut self.pending_removal_tasks));

        async move {
            futures::future::join_all(tasks).await;
        }
    }

    pub fn focus_active_workspace(&self, window: &mut Window, cx: &mut App) {
        // If a dock panel is zoomed, focus it instead of the center pane.
        // Otherwise, focusing the center pane triggers dismiss_zoomed_items_to_reveal
        // which closes the zoomed dock.
        let focus_handle = {
            let workspace = self.workspace().read(cx);
            let mut target = None;
            for dock in workspace.all_docks() {
                let dock = dock.read(cx);
                if dock.is_open() {
                    if let Some(panel) = dock.active_panel() {
                        if panel.is_zoomed(window, cx) {
                            target = Some(panel.panel_focus_handle(cx));
                            break;
                        }
                    }
                }
            }
            target.unwrap_or_else(|| {
                let pane = workspace.active_pane().clone();
                pane.read(cx).focus_handle(cx)
            })
        };
        window.focus(&focus_handle, cx);
    }

    pub fn panel<T: Panel>(&self, cx: &App) -> Option<Entity<T>> {
        self.workspace().read(cx).panel::<T>(cx)
    }

    pub fn active_modal<V: ManagedView + 'static>(&self, cx: &App) -> Option<Entity<V>> {
        self.workspace().read(cx).active_modal::<V>(cx)
    }

    pub fn add_panel<T: Panel>(
        &mut self,
        panel: Entity<T>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace().update(cx, |workspace, cx| {
            workspace.add_panel(panel, window, cx);
        });
    }

    pub fn focus_panel<T: Panel>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<T>> {
        self.workspace()
            .update(cx, |workspace, cx| workspace.focus_panel::<T>(window, cx))
    }

    // used in a test
    pub fn toggle_modal<V: ModalView, B>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        build: B,
    ) where
        B: FnOnce(&mut Window, &mut gpui::Context<V>) -> V,
    {
        self.workspace().update(cx, |workspace, cx| {
            workspace.toggle_modal(window, cx, build);
        });
    }

    pub fn toggle_dock(
        &mut self,
        dock_side: DockPosition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace().update(cx, |workspace, cx| {
            workspace.toggle_dock(dock_side, window, cx);
        });
    }

    pub fn active_item_as<I: 'static>(&self, cx: &App) -> Option<Entity<I>> {
        self.workspace().read(cx).active_item_as::<I>(cx)
    }

    pub fn items_of_type<'a, T: Item>(
        &'a self,
        cx: &'a App,
    ) -> impl 'a + Iterator<Item = Entity<T>> {
        self.workspace().read(cx).items_of_type::<T>(cx)
    }

    pub fn active_workspace_database_id(&self, cx: &App) -> Option<WorkspaceId> {
        self.workspace().read(cx).database_id()
    }

    pub fn take_pending_removal_tasks(&mut self) -> Vec<Task<()>> {
        let tasks: Vec<Task<()>> = std::mem::take(&mut self.pending_removal_tasks)
            .into_iter()
            .filter(|task| !task.is_ready())
            .collect();
        tasks
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_random_database_id(&mut self, cx: &mut Context<Self>) {
        self.workspace().update(cx, |workspace, _cx| {
            workspace.set_random_database_id();
        });
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn test_new(project: Entity<Project>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspace = cx.new(|cx| Workspace::test_new(project, window, cx));
        Self::new(workspace, window, cx)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn test_add_workspace(
        &mut self,
        project: Entity<Project>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<Workspace> {
        let workspace = cx.new(|cx| Workspace::test_new(project, window, cx));
        self.activate_in_window(workspace.clone(), window, cx);
        workspace
    }

    pub fn create_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let app_state = self.workspace().read(cx).app_state().clone();
        let project = Project::local(
            app_state.client.clone(),
            app_state.node_runtime.clone(),
            app_state.user_store.clone(),
            app_state.languages.clone(),
            app_state.fs.clone(),
            None,
            project::LocalProjectFlags::default(),
            cx,
        );
        let new_workspace = cx.new(|cx| Workspace::new(None, project, app_state, window, cx));
        self.set_active_workspace(new_workspace.clone(), cx);
        self.focus_active_workspace(window, cx);

        let weak_workspace = new_workspace.downgrade();
        let db = crate::persistence::WorkspaceDb::global(cx);
        self._create_task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = db.next_id().await;
            this.update_in(cx, |this, window, cx| match result {
                Ok(workspace_id) => {
                    if let Some(workspace) = weak_workspace.upgrade() {
                        let session_id = workspace.read(cx).session_id();
                        let window_id = window.window_handle().window_id().as_u64();
                        workspace.update(cx, |workspace, _cx| {
                            workspace.set_database_id(workspace_id);
                        });
                        let db = db.clone();
                        cx.background_spawn(async move {
                            db.set_session_binding(workspace_id, session_id, Some(window_id))
                                .await
                                .log_err();
                        })
                        .detach();
                        this.serialize(cx);
                    }
                }
                Err(err) => {
                    let err = err.context("failed to create workspace");
                    log::error!("{err:#}");
                }
            })
            .ok();
        }));
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn create_test_workspace(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        self.create_workspace(window, cx);
        self._create_task.take().unwrap_or_else(|| Task::ready(()))
    }

    pub fn remove_workspace(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<Workspace>> {
        if self.workspaces.len() <= 1 || index >= self.workspaces.len() {
            return None;
        }

        let removed_workspace = self.workspaces.remove(index);
        if self.active_workspace_index >= self.workspaces.len() {
            self.active_workspace_index = self.workspaces.len() - 1;
        } else if self.active_workspace_index > index {
            self.active_workspace_index -= 1;
        }
        let old_key = removed_workspace.read(cx).project_group_key(cx);

        let key_still_in_use = self
            .workspaces
            .iter()
            .any(|ws| ws.read(cx).project_group_key(cx) == old_key);

        if !key_still_in_use {
            self.project_group_keys.retain(|k| k != &old_key);
        }

        // Clear session_id and cancel any in-flight serialization on the
        // removed workspace. Without this, a pending throttle timer from
        // `serialize_workspace` could fire and write the old session_id
        // back to the DB, resurrecting the workspace on next launch.
        removed_workspace.update(cx, |workspace, _cx| {
            workspace.session_id.take();
            workspace._schedule_serialize_workspace.take();
            workspace._serialize_workspace_task.take();
        });

        if let Some(workspace_id) = removed_workspace.read(cx).database_id() {
            let db = crate::persistence::WorkspaceDb::global(cx);
            self.pending_removal_tasks.retain(|task| !task.is_ready());
            self.pending_removal_tasks
                .push(cx.background_spawn(async move {
                    // Clear the session binding instead of deleting the row so
                    // the workspace still appears in the recent-projects list.
                    db.set_session_binding(workspace_id, None, None)
                        .await
                        .log_err();
                }));
        }
        self.serialize(cx);
        self.focus_active_workspace(window, cx);
        cx.emit(MultiWorkspaceEvent::WorkspaceRemoved(
            removed_workspace.entity_id(),
        ));
        cx.emit(MultiWorkspaceEvent::ActiveWorkspaceChanged);
        cx.notify();

        Some(removed_workspace)
    }

    pub fn remove(
        &mut self,
        workspace: &Entity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<Workspace>> {
        let index = self
            .workspaces
            .iter()
            .position(|existing| existing == workspace)?;
        self.remove_workspace(index, window, cx)
    }

    pub fn move_workspace_to_new_window(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspaces.len() <= 1 || index >= self.workspaces.len() {
            return;
        }

        let Some(workspace) = self.remove_workspace(index, window, cx) else {
            return;
        };

        let app_state: Arc<crate::AppState> = workspace.read(cx).app_state().clone();

        cx.defer(move |cx| {
            let options = (app_state.build_window_options)(None, cx);

            let Ok(window) = cx.open_window(options, |window, cx| {
                cx.new(|cx| MultiWorkspace::new(workspace, window, cx))
            }) else {
                return;
            };

            let _ = window.update(cx, |_, window, _| {
                window.activate_window();
            });
        });
    }

    pub fn move_project_group_to_new_window(
        &mut self,
        key: &ProjectGroupKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspaces: Vec<_> = self
            .workspaces_for_project_group(key, cx)
            .cloned()
            .collect();
        if workspaces.is_empty() {
            return;
        }

        self.project_group_keys.retain(|k| k != key);

        let mut removed = Vec::new();
        for workspace in &workspaces {
            if self.remove(workspace, window, cx).is_some() {
                removed.push(workspace.clone());
            }
        }

        if removed.is_empty() {
            return;
        }

        let app_state = removed[0].read(cx).app_state().clone();

        cx.defer(move |cx| {
            let options = (app_state.build_window_options)(None, cx);

            let first = removed[0].clone();
            let rest = removed[1..].to_vec();

            let Ok(new_window) = cx.open_window(options, |window, cx| {
                cx.new(|cx| MultiWorkspace::new(first, window, cx))
            }) else {
                return;
            };

            new_window
                .update(cx, |mw, window, cx| {
                    for workspace in rest {
                        mw.activate(workspace, cx);
                    }
                    window.activate_window();
                })
                .log_err();
        });
    }

    pub fn open_project(
        &mut self,
        paths: Vec<PathBuf>,
        open_mode: OpenMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Result<Entity<Workspace>>> {
        if self.multi_workspace_enabled(cx) {
            self.find_or_create_local_workspace(PathList::new(&paths), window, cx)
        } else {
            let workspace = self.workspace().clone();
            cx.spawn_in(window, async move |_this, cx| {
                let should_continue = workspace
                    .update_in(cx, |workspace, window, cx| {
                        workspace.prepare_to_close(crate::CloseIntent::ReplaceWindow, window, cx)
                    })?
                    .await?;
                if should_continue {
                    workspace
                        .update_in(cx, |workspace, window, cx| {
                            workspace.open_workspace_for_paths(open_mode, paths, window, cx)
                        })?
                        .await
                } else {
                    Ok(workspace)
                }
            })
        }
    }
}

impl Render for MultiWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_workspace_sidebar_host(cx);

        let multi_workspace_enabled = self.multi_workspace_enabled(cx);

        let ui_font = theme_settings::setup_ui_font(window, cx);
        let text_color = cx.theme().colors().text;

        let workspace = self.workspace().clone();
        let workspace_key_context = workspace.update(cx, |workspace, cx| workspace.key_context(cx));
        let root = workspace.update(cx, |workspace, cx| workspace.actions(h_flex(), window, cx));

        client_side_decorations(
            root.key_context(workspace_key_context)
                .relative()
                .size_full()
                .font(ui_font)
                .text_color(text_color)
                .on_action(cx.listener(Self::close_window))
                .on_action(
                    cx.listener(|this: &mut Self, _: &NewWorkspaceInWindow, window, cx| {
                        this.create_workspace(window, cx);
                    }),
                )
                .when(self.multi_workspace_enabled(cx), |this| {
                    this.on_action(cx.listener(
                        |this: &mut Self, _: &ToggleProjectNavigation, window, cx| {
                            this.toggle_sidebar(window, cx);
                        },
                    ))
                    .on_action(cx.listener(
                        |this: &mut Self, _: &ToggleWorkspaceSidebar, window, cx| {
                            this.toggle_sidebar(window, cx);
                        },
                    ))
                    .on_action(cx.listener(
                        |this: &mut Self, _: &CloseProjectNavigation, window, cx| {
                            this.close_sidebar_action(window, cx);
                        },
                    ))
                    .on_action(cx.listener(
                        |this: &mut Self, _: &CloseWorkspaceSidebar, window, cx| {
                            this.close_sidebar_action(window, cx);
                        },
                    ))
                    .on_action(cx.listener(
                        |this: &mut Self, _: &FocusProjectNavigation, window, cx| {
                            this.focus_sidebar(window, cx);
                        },
                    ))
                    .on_action(cx.listener(
                        |this: &mut Self, _: &FocusWorkspaceSidebar, window, cx| {
                            this.focus_sidebar(window, cx);
                        },
                    ))
                    .on_action(cx.listener(Self::next_workspace))
                    .on_action(cx.listener(Self::previous_workspace))
                    .on_action(cx.listener(
                        |this: &mut Self, action: &ToggleThreadSwitcher, window, cx| {
                            if let Some(sidebar) = &this.sidebar {
                                sidebar.toggle_thread_switcher(action.select_last, window, cx);
                            }
                        },
                    ))
                })
                .when(
                    self.sidebar_open() && self.multi_workspace_enabled(cx),
                    |this| {
                        this.on_drag_move(cx.listener(
                            |this: &mut Self, e: &DragMoveEvent<DraggedSidebar>, _window, cx| {
                                let new_width = e.event.position.x;
                                this.workspace_sidebar_host.update(cx, |sidebar, cx| {
                                    sidebar.set_width(f64::from(new_width), cx);
                                });
                            },
                        ))
                    },
                )
                .on_action(
                    cx.listener(|this: &mut Self, _: &NextWorkspaceInWindow, window, cx| {
                        this.activate_next_workspace(window, cx);
                    }),
                )
                .on_action(cx.listener(
                    |this: &mut Self, _: &PreviousWorkspaceInWindow, window, cx| {
                        this.activate_previous_workspace(window, cx);
                    },
                ))
                .child({
                    let workspace_content = div()
                        .flex()
                        .flex_1()
                        .size_full()
                        .overflow_hidden()
                        .child(self.workspace().clone());

                    let workspace_content = {
                        let workspace = self.workspace().read(cx);
                        let sidebar_width = self.workspace_sidebar_host.read(cx).width();
                        let button_bar = workspace.button_bar(cx);

                        #[cfg(not(target_os = "macos"))]
                        let sidebar_collapsed = !self.sidebar_open();

                        #[cfg(target_os = "macos")]
                        let sidebar_collapsed =
                            workspace.workspace_sidebar_host_collapsed(window, cx);

                        #[cfg(not(target_os = "macos"))]
                        {
                            let weak = cx.weak_entity();
                            let resize_handle = deferred(
                                div()
                                    .id("workspace-sidebar-resize-handle")
                                    .absolute()
                                    .right(-SIDEBAR_RESIZE_HANDLE_SIZE / 2.)
                                    .top(px(0.))
                                    .h_full()
                                    .w(SIDEBAR_RESIZE_HANDLE_SIZE)
                                    .cursor_col_resize()
                                    .on_drag(DraggedSidebar, |dragged, _, _, cx| {
                                        cx.stop_propagation();
                                        cx.new(|_| dragged.clone())
                                    })
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                        cx.stop_propagation();
                                    })
                                    .on_mouse_up(MouseButton::Left, move |event, _, cx| {
                                        if event.click_count == 2 {
                                            weak.update(cx, |this, cx| {
                                                this.workspace_sidebar_host.update(
                                                    cx,
                                                    |sidebar, cx| {
                                                        sidebar.set_width(240.0, cx);
                                                    },
                                                );
                                                this.serialize(cx);
                                            })
                                            .ok();
                                            cx.stop_propagation();
                                        } else {
                                            weak.update(cx, |this, cx| {
                                                this.serialize(cx);
                                            })
                                            .ok();
                                        }
                                    })
                                    .occlude(),
                            );

                            div()
                                .size_full()
                                .flex()
                                .flex_row()
                                .when(!sidebar_collapsed, |this| {
                                    this.child(
                                        div()
                                            .id("workspace-sidebar-host-shell")
                                            .relative()
                                            .h_full()
                                            .w(px(sidebar_width as f32))
                                            .min_w(px(160.0))
                                            .max_w(px(480.0))
                                            .flex_shrink_0()
                                            .flex()
                                            .flex_col()
                                            .overflow_hidden()
                                            .bg(cx.theme().colors().panel_background)
                                            .when_some(button_bar, |this, dock_button_bar| {
                                                this.child(dock_button_bar)
                                            })
                                            .child(self.workspace_sidebar_host.clone())
                                            .child(resize_handle),
                                    )
                                })
                                .child(workspace_content)
                        }

                        #[cfg(target_os = "macos")]
                        {
                            let sidebar_titlebar_fill =
                                match cx.theme().window_background_appearance() {
                                    gpui::WindowBackgroundAppearance::Opaque => {
                                        Some(cx.theme().colors().panel_background)
                                    }
                                    _ => None,
                                };

                            if cfg!(any(test, feature = "test-support")) {
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_row()
                                    .child(
                                        div()
                                            .w(px(sidebar_width as f32))
                                            .min_w(px(160.0))
                                            .max_w(px(480.0))
                                            .when(sidebar_collapsed, |this| {
                                                this.w(px(0.0)).overflow_hidden()
                                            })
                                            .child(self.workspace_sidebar_host.clone()),
                                    )
                                    .child(workspace_content)
                            } else {
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_row()
                                    .child(
                                        native_sidebar("workspace-sidebar-host-shell", &[""; 0])
                                            .when_some(button_bar, |this, dock_button_bar| {
                                                this.header_view(
                                                    dock_button_bar,
                                                    crate::dock::DockButtonBar::NATIVE_SIDEBAR_HEIGHT,
                                                )
                                            })
                                            .sidebar_view(self.workspace_sidebar_host.clone())
                                            .sidebar_width(sidebar_width)
                                            .min_sidebar_width(160.0)
                                            .max_sidebar_width(480.0)
                                            .manage_window_chrome(false)
                                            .manage_toolbar(false)
                                            .collapsed(sidebar_collapsed)
                                            .sidebar_background_color(sidebar_titlebar_fill)
                                            .size_full(),
                                    )
                                    .child(workspace_content)
                            }
                        }
                    };

                    workspace_content
                })
                .child(self.workspace().read(cx).modal_layer.clone())
                .children(self.sidebar_overlay.as_ref().map(|view| {
                    deferred(div().absolute().size_full().inset_0().occlude().child(
                        v_flex().h(px(0.0)).top_20().items_center().child(
                            h_flex().occlude().child(view.clone()).on_mouse_down(
                                MouseButton::Left,
                                |_, _, cx| {
                                    cx.stop_propagation();
                                },
                            ),
                        ),
                    ))
                    .with_priority(2)
                })),
            window,
            cx,
            Tiling {
                left: cfg!(not(target_os = "macos"))
                    && multi_workspace_enabled
                    && self.sidebar_open(),
                ..Tiling::default()
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use feature_flags::FeatureFlagAppExt;
    use fs::FakeFs;
    use gpui::{Context, FocusHandle, Focusable, Render, TestAppContext, div};
    use project::DisableAiSettings;
    use settings::{Settings, SettingsStore};
    use std::sync::Arc;
    use workspace_modes::RegisteredModeView;

    struct TestBrowserModeView {
        focus_handle: FocusHandle,
    }

    impl TestBrowserModeView {
        fn new(cx: &mut Context<Self>) -> Self {
            Self {
                focus_handle: cx.focus_handle(),
            }
        }
    }

    impl Focusable for TestBrowserModeView {
        fn focus_handle(&self, _cx: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }

    impl Render for TestBrowserModeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    fn init_test(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            DisableAiSettings::register(cx);
            cx.update_flags(false, vec!["agent-v2".into()]);
            workspace_modes::init(cx);
        });
    }

    #[gpui::test]
    async fn test_browser_mode_view_is_shared_across_workspaces(cx: &mut TestAppContext) {
        init_test(cx);

        cx.update(|cx| {
            ModeViewRegistry::global_mut(cx).register_factory(
                ModeId::BROWSER,
                Arc::new(|cx| {
                    let browser_view: Entity<TestBrowserModeView> =
                        cx.new(|cx| TestBrowserModeView::new(cx));
                    let focus_handle = browser_view.focus_handle(cx);

                    RegisteredModeView {
                        view: browser_view.into(),
                        focus_handle,
                        titlebar_center_view: None,
                        sidebar_view: None,
                        navigation_host: None,
                        on_activate: None,
                        on_deactivate: None,
                    }
                }),
            );
        });

        let fs = FakeFs::new(cx.executor());
        let project_a = Project::test(fs.clone(), [], cx).await;
        let project_b = Project::test(fs, [], cx).await;

        let (multi_workspace, cx) =
            cx.add_window_view(|window, cx| MultiWorkspace::test_new(project_a, window, cx));

        multi_workspace.update_in(cx, |multi_workspace, window, cx| {
            multi_workspace.test_add_workspace(project_b, window, cx);
        });

        multi_workspace.read_with(cx, |multi_workspace, cx| {
            let first_browser_view = multi_workspace.workspaces()[0]
                .read(cx)
                .get_mode_view(ModeId::BROWSER)
                .and_then(|view| view.downcast::<TestBrowserModeView>().ok())
                .expect("first workspace should resolve the shared browser view");
            let second_browser_view = multi_workspace.workspaces()[1]
                .read(cx)
                .get_mode_view(ModeId::BROWSER)
                .and_then(|view| view.downcast::<TestBrowserModeView>().ok())
                .expect("second workspace should resolve the shared browser view");

            assert_eq!(
                first_browser_view.entity_id(),
                second_browser_view.entity_id(),
            );
        });
    }
}
