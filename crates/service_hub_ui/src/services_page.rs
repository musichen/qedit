use std::collections::BTreeMap;

use db::kvp::KeyValueStore;
#[cfg(target_os = "macos")]
use gpui::Subscription;
use gpui::{
    App, Context, Corner, CursorStyle, Entity, EventEmitter, FocusHandle, Focusable, Global,
    InteractiveElement, Render, SharedString, Stateful, WeakEntity, Window, div, point,
    prelude::FluentBuilder as _, px,
};
use service_hub::{ServiceHub, ServiceProviderDescriptor};
use ui::{
    AnyElement, ButtonLike, ButtonSize, ButtonStyle, Checkbox, Clickable, Color, ContextMenu, Icon,
    IconName, IconSize, Label, LabelSize, PopoverMenu, SpinnerLabel, Toggleable, prelude::*,
};
#[cfg(target_os = "macos")]
use ui::{
    Button, ContextMenuEntry, ContextMenuItem, IconButton, IconButtonShape, Indicator, Severity,
    TintColor, Tooltip,
};
use workspace::Workspace;
#[cfg(target_os = "macos")]
use workspace::WorkspaceSidebarSection;
use workspace::item::{Item, ItemBufferKind, ItemEvent};
#[cfg(target_os = "macos")]
use workspace_chrome::SidebarRow;

#[cfg(target_os = "macos")]
use crate::service_auth::{
    ServiceAuthFieldState, ServiceAuthStatusSummary, ServiceAuthUiAction, ServiceAuthUiModel,
};
use crate::service_hub_onboarding::ServiceHubOnboarding;
use crate::service_workflow::{
    ServiceWorkflowFieldState, ServiceWorkflowOption, ServiceWorkflowRunSummary,
    ServiceWorkflowUiAction, ServiceWorkflowUiModel,
};
use crate::services_provider::{
    ServiceWorkspacePane, ServicesPageState, build_service_workspace_panes,
    collect_provider_descriptors, normalize_services_page_state,
};

pub struct ServicesPage {
    focus_handle: FocusHandle,
    workspace: WeakEntity<Workspace>,
    workspace_paths: Vec<std::path::PathBuf>,
    providers: Vec<ServiceProviderDescriptor>,
    panes: BTreeMap<String, ServiceWorkspacePane>,
    state: ServicesPageState,
    onboarding_visible: bool,
}

#[derive(Default)]
struct ServiceHubOnboardingState {
    force_show: bool,
}

impl Global for ServiceHubOnboardingState {}

impl ServicesPage {
    const ONBOARDING_NAMESPACE: &'static str = "service_hub_onboarding";
    const ONBOARDING_SEEN_KEY: &'static str = "seen_v1";

    pub fn open(
        workspace: &mut Workspace,
        force_show_onboarding: bool,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        #[cfg(target_os = "macos")]
        Self::install_sidebar_section_view(workspace, cx);

        let should_show_onboarding = Self::should_show_onboarding(force_show_onboarding, cx);

        if let Some(existing) = workspace.item_of_type::<Self>(cx) {
            existing.update(cx, |page, cx| {
                if should_show_onboarding {
                    page.present_onboarding(cx);
                }
            });
            workspace.activate_item(&existing, true, true, window, cx);
            #[cfg(target_os = "macos")]
            workspace.select_sidebar_section(WorkspaceSidebarSection::Services, window, cx);
            return;
        }

        let page = Self::new(workspace, None, should_show_onboarding, window, cx);
        workspace.add_item_to_active_pane(Box::new(page), None, true, window, cx);
        #[cfg(target_os = "macos")]
        workspace.select_sidebar_section(WorkspaceSidebarSection::Services, window, cx);
    }

    pub fn reset_onboarding(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        Self::set_force_show_onboarding(true, cx);
        Self::clear_onboarding_seen(cx);

        if let Some(existing) = workspace.item_of_type::<Self>(cx) {
            existing.update(cx, |page, cx| {
                page.present_onboarding(cx);
            });
            workspace.activate_item(&existing, true, true, window, cx);
            #[cfg(target_os = "macos")]
            workspace.select_sidebar_section(WorkspaceSidebarSection::Services, window, cx);
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn install_sidebar_section_view(
        workspace: &mut Workspace,
        cx: &mut Context<Workspace>,
    ) {
        let workspace_handle = workspace.weak_handle();
        let sidebar_panel = cx.new(|cx| ServicesSidebarPanel::new(workspace_handle, cx));
        workspace.set_sidebar_section_view(
            WorkspaceSidebarSection::Services,
            Some(sidebar_panel.into()),
            cx,
        );
    }

    fn new(
        workspace: &mut Workspace,
        initial_state: Option<ServicesPageState>,
        onboarding_visible: bool,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        let panes = build_service_workspace_panes(ServiceHub::default().providers(), window, cx);
        let providers = collect_provider_descriptors(&panes);
        let state = normalize_services_page_state(&providers, initial_state);
        let workspace_handle = workspace.weak_handle();
        let workspace_paths = workspace
            .project()
            .read(cx)
            .visible_worktrees(cx)
            .map(|worktree| worktree.read(cx).abs_path().to_path_buf())
            .collect::<Vec<_>>();

        let page = cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            workspace: workspace_handle,
            workspace_paths,
            providers,
            panes,
            state,
            onboarding_visible,
        });

        page.update(cx, |page, cx| {
            page.normalize_active_provider_state();
            page.refresh_provider(window, cx);
        });
        page
    }

    pub(crate) fn workspace(&self) -> &WeakEntity<Workspace> {
        &self.workspace
    }

    pub(crate) fn workspace_paths(&self) -> &[std::path::PathBuf] {
        &self.workspace_paths
    }

    fn onboarding_seen(cx: &App) -> bool {
        KeyValueStore::global(cx)
            .scoped(Self::ONBOARDING_NAMESPACE)
            .read(Self::ONBOARDING_SEEN_KEY)
            .ok()
            .flatten()
            .is_some()
    }

    fn should_show_onboarding(force_show_onboarding: bool, cx: &App) -> bool {
        force_show_onboarding
            || cx
                .try_global::<ServiceHubOnboardingState>()
                .is_some_and(|state| state.force_show)
            || !Self::onboarding_seen(cx)
    }

    fn set_force_show_onboarding(force_show: bool, cx: &mut App) {
        cx.update_default_global::<ServiceHubOnboardingState, _>(|state, _| {
            state.force_show = force_show;
        });
    }

    fn persist_onboarding_seen(cx: &mut App) {
        let kvp = KeyValueStore::global(cx);
        db::write_and_log(cx, move || async move {
            kvp.scoped(Self::ONBOARDING_NAMESPACE)
                .write(Self::ONBOARDING_SEEN_KEY.to_string(), "1".to_string())
                .await
        });
    }

    fn clear_onboarding_seen(cx: &mut App) {
        let kvp = KeyValueStore::global(cx);
        db::write_and_log(cx, move || async move {
            kvp.scoped(Self::ONBOARDING_NAMESPACE)
                .delete(Self::ONBOARDING_SEEN_KEY.to_string())
                .await
        });
    }

    fn present_onboarding(&mut self, cx: &mut Context<Self>) {
        if self.onboarding_visible {
            return;
        }

        self.onboarding_visible = true;
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
    }

    pub(crate) fn complete_onboarding(
        &mut self,
        provider_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.onboarding_visible = false;
        Self::set_force_show_onboarding(false, cx);
        Self::persist_onboarding_seen(cx);
        cx.emit(ItemEvent::UpdateTab);

        match provider_id {
            Some(provider_id) if provider_id != self.state.provider_id => {
                self.select_provider(provider_id, window, cx);
            }
            _ => cx.notify(),
        }
    }

    pub(crate) fn with_provider_mut<R>(
        &mut self,
        provider_id: &str,
        callback: impl FnOnce(
            &mut dyn crate::services_provider::ServiceWorkspaceAdapter,
            &mut ServicesPageState,
        ) -> R,
    ) -> Option<R> {
        let pane = self.panes.get_mut(provider_id)?;
        Some(callback(pane.as_mut(), &mut self.state))
    }

    fn provider(&self) -> &ServiceProviderDescriptor {
        self.providers
            .iter()
            .find(|provider| provider.id == self.state.provider_id)
            .expect("selected provider should stay valid")
    }

    fn active_pane(&self) -> &ServiceWorkspacePane {
        self.panes
            .get(&self.state.provider_id)
            .expect("selected provider pane should stay valid")
    }

    fn normalize_active_provider_state(&mut self) {
        let provider_id = self.state.provider_id.clone();
        self.with_provider_mut(&provider_id, |pane, state| {
            pane.normalize_state(state);
        });
    }

    fn refresh_provider(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let provider_id = self.state.provider_id.clone();
        let workspace_paths = self.workspace_paths.clone();
        self.with_provider_mut(&provider_id, |pane, state| {
            pane.refresh(state, workspace_paths, window, cx);
        });
    }

    fn select_provider(
        &mut self,
        provider_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.state.provider_id == provider_id {
            return;
        }

        self.state.provider_id = provider_id;
        self.state.navigation_id = self.provider().shell.default_navigation_item_id.clone();
        self.state.selected_resource_id = None;
        self.state.selected_target_id = None;
        self.state.selected_workflow_id = None;
        self.normalize_active_provider_state();
        cx.emit(ItemEvent::UpdateTab);
        self.refresh_provider(window, cx);
    }

    #[cfg(target_os = "macos")]
    fn select_navigation(&mut self, navigation_id: String, cx: &mut Context<Self>) {
        if self.state.navigation_id == navigation_id {
            return;
        }

        self.state.navigation_id = navigation_id;
        self.normalize_active_provider_state();
        cx.notify();
    }

    #[cfg(target_os = "macos")]
    fn select_resource(
        &mut self,
        resource_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let provider_id = self.state.provider_id.clone();
        self.with_provider_mut(&provider_id, |pane, state| {
            pane.select_resource(state, resource_id, window, cx);
        });
    }

    #[cfg(target_os = "macos")]
    fn render_provider_menu(
        &self,
        page: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let menu = ContextMenu::build(window, cx, |mut menu, _, _| {
            for provider in &self.providers {
                let provider_id = provider.id.clone();
                let mut entry = ContextMenuEntry::new(provider.label.clone()).handler({
                    let page = page.clone();
                    move |window, cx| {
                        page.update(cx, |this, cx| {
                            this.select_provider(provider_id.clone(), window, cx);
                        })
                        .ok();
                    }
                });

                entry = entry.icon(IconName::Server);
                menu = menu.item(ContextMenuItem::Entry(entry));
            }

            menu
        });

        PopoverMenu::new("services-provider-menu-popover")
            .full_width(true)
            .window_overlay()
            .menu(move |_window, _cx| Some(menu.clone()))
            .trigger(
                ServiceSidebarMenuTrigger::new(
                    "services-provider-menu",
                    self.provider().label.clone(),
                )
                .start_image_path(self.provider().logo_asset_path.clone()),
            )
            .attach(Corner::BottomLeft)
            .anchor(Corner::TopLeft)
            .offset(point(px(0.), px(4.)))
    }

    #[cfg(target_os = "macos")]
    fn render_resource_menu(
        &self,
        page: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let resource_menu = self.active_pane().resource_menu(&self.state)?;
        let menu = ContextMenu::build(window, cx, |mut menu, _, _| {
            for entry in &resource_menu.entries {
                let resource_id = entry.id.clone();
                let label = match &entry.detail {
                    Some(detail) => format!("{} ({detail})", entry.label),
                    None => entry.label.clone(),
                };
                let page = page.clone();
                menu = menu.entry(label, None, move |window, cx| {
                    page.update(cx, |this, cx| {
                        this.select_resource(resource_id.clone(), window, cx);
                    })
                    .ok();
                });
            }

            menu
        });

        Some(
            v_flex()
                .gap_1()
                .child(
                    Label::new(resource_menu.singular_label.clone())
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .child(Self::render_sidebar_popover_menu(
                    "services-resource-menu",
                    resource_menu.current_label,
                    menu,
                ))
                .into_any_element(),
        )
    }

    pub(crate) fn render_sidebar_popover_menu(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        menu: Entity<ContextMenu>,
    ) -> impl IntoElement {
        let id = id.into();
        let label = label.into();
        PopoverMenu::new(format!("{id}-popover"))
            .full_width(true)
            .window_overlay()
            .menu(move |_window, _cx| Some(menu.clone()))
            .trigger(ServiceSidebarMenuTrigger::new(id, label))
            .attach(Corner::BottomLeft)
            .anchor(Corner::TopLeft)
            .offset(point(px(0.), px(4.)))
    }

    #[cfg(target_os = "macos")]
    fn render_auth_sidebar_footer(
        &self,
        page: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let auth_ui = self.active_pane().auth_ui_model()?;
        let authenticate_label = if auth_ui.status.authenticated {
            auth_ui.reauthenticate_label.clone()
        } else {
            auth_ui.authenticate_label.clone()
        };
        let provider_id = auth_ui.provider_id.clone();
        let (indicator_color, status_tooltip) = Self::render_auth_status_indicator(&auth_ui.status);

        Some(
            v_flex()
                .gap_3()
                .pt_3()
                .border_t_1()
                .border_color(cx.theme().colors().border_variant)
                .child(
                    h_flex()
                        .justify_between()
                        .items_start()
                        .gap_2()
                        .child(
                            v_flex().min_w_0().gap_1().child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        ButtonLike::new("services-auth-status-indicator")
                                            .style(ButtonStyle::Transparent)
                                            .size(ButtonSize::None)
                                            .cursor_style(CursorStyle::Arrow)
                                            .tooltip(Tooltip::text(status_tooltip))
                                            .child(Indicator::dot().color(indicator_color)),
                                    )
                                    .child(
                                        Label::new(auth_ui.status.detail.clone())
                                            .size(LabelSize::Small)
                                            .color(Color::Muted)
                                            .truncate(),
                                    ),
                            ),
                        )
                        .child(h_flex().items_center().gap_1().child(
                            if auth_ui.status.authenticated {
                                self.render_auth_overflow_menu(
                                    page.clone(),
                                    provider_id.clone(),
                                    window,
                                    cx,
                                    authenticate_label.clone(),
                                )
                                .into_any_element()
                            } else {
                                Button::new("services-auth-open", authenticate_label.clone())
                                    .style(ButtonStyle::Filled)
                                    .size(ButtonSize::Compact)
                                    .disabled(auth_ui.form.pending)
                                    .on_click({
                                        let page = page.clone();
                                        let provider_id = provider_id.clone();
                                        move |_, window, cx| {
                                            Self::dispatch_auth_action(
                                                &page,
                                                &provider_id,
                                                ServiceAuthUiAction::ShowAuthenticate,
                                                window,
                                                cx,
                                            );
                                        }
                                    })
                                    .into_any_element()
                            },
                        )),
                )
                .when_some(auth_ui.form.error_message.clone(), |this, error| {
                    this.child(Label::new(error).size(LabelSize::Small).color(Color::Error))
                })
                .when(
                    auth_ui.form.logout_available && auth_ui.status.authenticated,
                    |this| {
                        this.child(
                            SidebarRow::new(
                                "services-auth-logout-row",
                                auth_ui.logout_label.clone(),
                                IconName::Exit,
                            )
                            .on_click({
                                let page = page.clone();
                                let provider_id = provider_id.clone();
                                move |_, window, cx| {
                                    Self::dispatch_auth_action(
                                        &page,
                                        &provider_id,
                                        ServiceAuthUiAction::Logout,
                                        window,
                                        cx,
                                    );
                                }
                            }),
                        )
                    },
                )
                .when(auth_ui.form.expanded, |this| {
                    this.child(self.render_auth_form(
                        page.clone(),
                        auth_ui.clone(),
                        authenticate_label.clone(),
                        cx,
                    ))
                })
                .into_any_element(),
        )
    }

    #[cfg(target_os = "macos")]
    fn render_auth_overflow_menu(
        &self,
        page: WeakEntity<Self>,
        provider_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
        authenticate_label: SharedString,
    ) -> impl IntoElement {
        let menu = ContextMenu::build(window, cx, |menu, _, _| {
            menu.entry(authenticate_label.clone(), None, {
                let page = page.clone();
                let provider_id = provider_id.clone();
                move |window, cx| {
                    Self::dispatch_auth_action(
                        &page,
                        &provider_id,
                        ServiceAuthUiAction::ShowAuthenticate,
                        window,
                        cx,
                    );
                }
            })
        });

        PopoverMenu::new("services-auth-overflow-menu")
            .window_overlay()
            .menu(move |_window, _cx| Some(menu.clone()))
            .trigger(
                IconButton::new("services-auth-overflow", IconName::Ellipsis)
                    .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                    .shape(IconButtonShape::Square)
                    .style(ButtonStyle::Transparent)
                    .size(ButtonSize::Compact)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Authentication actions")),
            )
            .attach(Corner::BottomRight)
            .anchor(Corner::TopRight)
            .offset(point(px(0.), px(4.)))
    }

    #[cfg(target_os = "macos")]
    fn render_auth_form(
        &self,
        page: WeakEntity<Self>,
        auth_ui: ServiceAuthUiModel,
        authenticate_label: SharedString,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().background)
            .child(
                v_flex()
                    .gap_2()
                    .children(auth_ui.form.fields.iter().map(|field| {
                        match field {
                            ServiceAuthFieldState::Text { descriptor, input } => {
                                match descriptor.kind {
                                    service_hub::ServiceInputKind::FilePath => {
                                        h_flex()
                                            .items_end()
                                            .gap_2()
                                            .child(input.clone())
                                            .child(
                                                Button::new(
                                                    SharedString::from(format!(
                                                        "browse-auth-{}",
                                                        descriptor.key
                                                    )),
                                                    "Browse…",
                                                )
                                                .style(ButtonStyle::Outlined)
                                                .size(ButtonSize::Compact)
                                                .disabled(auth_ui.form.pending)
                                                .on_click({
                                                    let page = page.clone();
                                                    let provider_id = auth_ui.provider_id.clone();
                                                    let field_key = descriptor.key.clone();
                                                    move |_, window, cx| {
                                                        Self::dispatch_auth_action(
                                                            &page,
                                                            &provider_id,
                                                            ServiceAuthUiAction::PickFile {
                                                                field_key: field_key.clone(),
                                                            },
                                                            window,
                                                            cx,
                                                        );
                                                    }
                                                }),
                                            )
                                            .into_any_element()
                                    }
                                    service_hub::ServiceInputKind::Text
                                    | service_hub::ServiceInputKind::Toggle => {
                                        input.clone().into_any_element()
                                    }
                                }
                            }
                            ServiceAuthFieldState::Toggle { descriptor, value } => Checkbox::new(
                                SharedString::from(format!("auth-toggle-{}", descriptor.key)),
                                *value,
                            )
                            .label(descriptor.label.clone())
                            .disabled(auth_ui.form.pending)
                            .on_click(cx.listener({
                                let page = page.clone();
                                let provider_id = auth_ui.provider_id.clone();
                                let field_key = descriptor.key.clone();
                                move |_page, checked, window, cx| {
                                    Self::dispatch_auth_action(
                                        &page,
                                        &provider_id,
                                        ServiceAuthUiAction::SetToggle {
                                            field_key: field_key.clone(),
                                            value: *checked,
                                        },
                                        window,
                                        cx,
                                    );
                                }
                            }))
                            .into_any_element(),
                        }
                    }))
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("services-auth-cancel", "Cancel")
                                    .style(ButtonStyle::Outlined)
                                    .size(ButtonSize::Compact)
                                    .disabled(auth_ui.form.pending)
                                    .on_click({
                                        let page = page.clone();
                                        let provider_id = auth_ui.provider_id.clone();
                                        move |_, window, cx| {
                                            Self::dispatch_auth_action(
                                                &page,
                                                &provider_id,
                                                ServiceAuthUiAction::CancelAuthenticate,
                                                window,
                                                cx,
                                            );
                                        }
                                    }),
                            )
                            .child(
                                Button::new("services-auth-submit", authenticate_label)
                                    .style(ButtonStyle::Filled)
                                    .size(ButtonSize::Compact)
                                    .disabled(auth_ui.form.pending)
                                    .on_click({
                                        let page = page.clone();
                                        let provider_id = auth_ui.provider_id.clone();
                                        move |_, window, cx| {
                                            Self::dispatch_auth_action(
                                                &page,
                                                &provider_id,
                                                ServiceAuthUiAction::SubmitAuthenticate,
                                                window,
                                                cx,
                                            );
                                        }
                                    }),
                            ),
                    ),
            )
    }

    #[cfg(target_os = "macos")]
    fn render_auth_status_indicator(status: &ServiceAuthStatusSummary) -> (Color, String) {
        let indicator_color = match status.severity {
            Severity::Success => Color::Success,
            Severity::Warning => Color::Warning,
            Severity::Error => Color::Error,
            Severity::Info => Color::Info,
        };

        let mut tooltip_lines = vec![status.headline.clone(), status.detail.clone()];
        tooltip_lines.extend(status.warnings.clone());
        (indicator_color, tooltip_lines.join("\n"))
    }

    #[cfg(target_os = "macos")]
    fn dispatch_auth_action(
        page: &WeakEntity<Self>,
        provider_id: &str,
        action: ServiceAuthUiAction,
        window: &mut Window,
        cx: &mut App,
    ) {
        page.update(cx, |page, cx| {
            let workspace = page.workspace().clone();
            page.with_provider_mut(provider_id, |pane, state| {
                pane.handle_auth_ui_action(state, action, workspace, window, cx);
            });
        })
        .ok();
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn render_sidebar_controls(
        page: &Entity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let page_handle = page.downgrade();
        page.update(cx, |page, cx| {
            page.render_sidebar_controls_inner(page_handle, window, cx)
                .into_any_element()
        })
    }

    #[cfg(target_os = "macos")]
    fn render_sidebar_controls_inner(
        &self,
        page: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.onboarding_visible {
            return v_flex()
                .size_full()
                .p_3()
                .justify_center()
                .gap_2()
                .child(Label::new("Services").size(LabelSize::Large))
                .child(
                    Label::new("Choose a provider to get started.")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                );
        }

        let controls = v_flex()
            .flex_1()
            .min_h_0()
            .gap_1()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                Label::new("Provider")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                            .child(
                                IconButton::new("services-refresh", IconName::RotateCw)
                                    .shape(IconButtonShape::Square)
                                    .style(ButtonStyle::Transparent)
                                    .size(ButtonSize::Compact)
                                    .icon_size(IconSize::Small)
                                    .tooltip(Tooltip::text("Refresh"))
                                    .on_click({
                                        let page = page.clone();
                                        move |_, window, cx| {
                                            page.update(cx, |this, cx| {
                                                this.refresh_provider(window, cx);
                                            })
                                            .ok();
                                        }
                                    }),
                            ),
                    )
                    .child(self.render_provider_menu(page.clone(), window, cx)),
            )
            .when_some(
                self.render_resource_menu(page.clone(), window, cx),
                |this, resource_menu| this.child(resource_menu),
            )
            .child(
                v_flex()
                    .gap_1()
                    .children(self.provider().shell.navigation_items.iter().map(
                        |navigation_item| {
                            SidebarRow::new(
                                format!("services-nav-{}", navigation_item.id),
                                navigation_item.label.clone(),
                                Self::navigation_icon(&navigation_item.id),
                            )
                            .selected(self.state.navigation_id == navigation_item.id)
                            .on_click({
                                let navigation_id = navigation_item.id.clone();
                                let page = page.clone();
                                move |_, _window, cx| {
                                    page.update(cx, |this, cx| {
                                        this.select_navigation(navigation_id.clone(), cx);
                                    })
                                    .ok();
                                }
                            })
                        },
                    )),
            );

        v_flex()
            .size_full()
            .p_3()
            .gap_3()
            .child(controls)
            .when_some(
                self.render_auth_sidebar_footer(page.clone(), window, cx),
                |this, footer| this.child(footer),
            )
            .when_some(
                self.active_pane()
                    .render_sidebar_footer_extra(&self.state, window, cx),
                |this, footer| this.child(footer),
            )
    }

    fn render_provider_content(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.onboarding_visible {
            return ServiceHubOnboarding::new(cx.entity().downgrade(), self.providers.clone())
                .into_any_element();
        }

        let workflow_ui = self.active_pane().workflow_ui_model(&self.state);

        match workflow_ui {
            Some(workflow_ui) => v_flex()
                .size_full()
                .min_h_0()
                .gap_4()
                .child(self.render_workflow_surface(workflow_ui, window, cx))
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .child(self.active_pane().render_section(&self.state, window, cx)),
                )
                .into_any_element(),
            None => self.active_pane().render_section(&self.state, window, cx),
        }
    }

    #[cfg(target_os = "macos")]
    fn navigation_icon(navigation_id: &str) -> IconName {
        match navigation_id {
            "overview" => IconName::Info,
            "builds" => IconName::Box,
            "release" => IconName::ArrowCircle,
            _ => IconName::Globe,
        }
    }

    pub(crate) fn render_action_chip(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: IconName,
        disabled: bool,
        handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
        cx: &mut App,
    ) -> impl IntoElement {
        let id = id.into();
        let label: SharedString = label.into();
        let theme = cx.theme();
        let hover_background = theme.colors().text.opacity(0.09);
        let label_color = if disabled {
            theme.colors().text_muted.opacity(0.6)
        } else {
            theme.colors().text_muted
        };

        div()
            .id(id)
            .relative()
            .flex()
            .items_center()
            .h(px(28.))
            .px_2()
            .gap_1()
            .rounded(theme.component_radius().tab.unwrap_or(px(8.0)))
            .when(!disabled, |this| {
                this.hover(move |style| style.bg(hover_background))
                    .cursor_pointer()
                    .on_click(handler)
            })
            .when(disabled, |this| this.opacity(0.5))
            .child(Icon::new(icon).size(IconSize::Small).color(Color::Muted))
            .child(
                div()
                    .text_size(rems(0.75))
                    .text_color(label_color)
                    .child(label),
            )
    }

    fn render_workflow_submit_button(
        page: WeakEntity<Self>,
        workflow_ui: &ServiceWorkflowUiModel,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let disabled = workflow_ui.form.pending
            || workflow_ui.selected_workflow_id.is_none()
            || workflow_ui.disabled_reason.is_some();
        let provider_id = workflow_ui.provider_id.clone();
        let pending = workflow_ui.form.pending;
        let label = workflow_ui.execute_label.clone();

        ButtonLike::new("services-workflow-submit")
            .style(ButtonStyle::Subtle)
            .size(ButtonSize::Compact)
            .disabled(disabled)
            .on_click(move |_, window, cx| {
                Self::dispatch_workflow_action(
                    &page,
                    &provider_id,
                    ServiceWorkflowUiAction::Submit,
                    window,
                    cx,
                );
            })
            .child(
                h_flex()
                    .items_center()
                    .gap_1p5()
                    .when(pending, |this| {
                        this.child(
                            SpinnerLabel::dots()
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                    })
                    .when(!pending, |this| {
                        this.child(Icon::new(IconName::PlayFilled).size(IconSize::Small))
                    })
                    .child(
                        Label::new(if pending {
                            format!("{label}…")
                        } else {
                            label.to_string()
                        })
                        .size(LabelSize::Small),
                    ),
            )
    }

    fn render_workflow_surface(
        &self,
        workflow_ui: ServiceWorkflowUiModel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let radius = cx.theme().component_radius().panel.unwrap_or(px(10.0));
        let page = cx.entity().downgrade();
        let custom_form = self.active_pane().render_workflow_form(
            &self.state,
            &workflow_ui,
            page.clone(),
            window,
            cx,
        );
        let uses_custom_form = custom_form.is_some();
        let target_menu = if workflow_ui.targets.is_empty() {
            None
        } else {
            let menu = Self::build_workflow_option_menu(
                &workflow_ui.targets,
                page.clone(),
                workflow_ui.provider_id.clone(),
                window,
                cx,
                |option| ServiceWorkflowUiAction::SelectTarget {
                    target_id: option.id.clone(),
                },
            );
            Some(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new(workflow_ui.target_label.clone())
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(Self::render_sidebar_popover_menu(
                        "services-workflow-target-menu",
                        workflow_ui
                            .selected_target_id
                            .as_ref()
                            .and_then(|selected_id| {
                                workflow_ui
                                    .targets
                                    .iter()
                                    .find(|target| &target.id == selected_id)
                                    .map(|target| target.label.clone())
                            })
                            .unwrap_or_else(|| workflow_ui.target_label.to_string()),
                        menu,
                    ))
                    .into_any_element(),
            )
        };

        let workflow_menu = if workflow_ui.workflows.is_empty() {
            None
        } else {
            let menu = Self::build_workflow_option_menu(
                &workflow_ui.workflows,
                page.clone(),
                workflow_ui.provider_id.clone(),
                window,
                cx,
                |option| ServiceWorkflowUiAction::SelectWorkflow {
                    workflow_id: option.id.clone(),
                },
            );
            Some(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new(workflow_ui.workflow_label.clone())
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(Self::render_sidebar_popover_menu(
                        "services-workflow-menu",
                        workflow_ui
                            .selected_workflow_id
                            .as_ref()
                            .and_then(|selected_id| {
                                workflow_ui
                                    .workflows
                                    .iter()
                                    .find(|workflow| &workflow.id == selected_id)
                                    .map(|workflow| workflow.label.clone())
                            })
                            .unwrap_or_else(|| workflow_ui.workflow_label.to_string()),
                        menu,
                    ))
                    .into_any_element(),
            )
        };

        v_flex()
            .gap_3()
            .p_4()
            .rounded(radius)
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().background)
            .when_some(target_menu, |this, target_menu| this.child(target_menu))
            .when_some(workflow_menu, |this, workflow_menu| {
                this.child(workflow_menu)
            })
            .when_some(custom_form, |this, custom_form| this.child(custom_form))
            .when(
                !uses_custom_form && !workflow_ui.form.fields.is_empty(),
                |this| this.child(self.render_workflow_form(page.clone(), workflow_ui.clone(), cx)),
            )
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_3()
                    .child(
                        v_flex()
                            .gap_1()
                            .when_some(workflow_ui.run.clone(), |this, run| {
                                let (color, headline) = Self::render_workflow_run(run);
                                this.child(Label::new(headline).size(LabelSize::Small).color(color))
                            })
                            .when_some(workflow_ui.disabled_reason.clone(), |this, reason| {
                                this.child(
                                    Label::new(reason)
                                        .size(LabelSize::Small)
                                        .color(Color::Muted),
                                )
                            }),
                    )
                    .child(Self::render_workflow_submit_button(
                        page.clone(),
                        &workflow_ui,
                        cx,
                    )),
            )
            .when_some(workflow_ui.form.error_message.clone(), |this, error| {
                this.child(Label::new(error).size(LabelSize::Small).color(Color::Error))
            })
    }

    fn build_workflow_option_menu(
        options: &[ServiceWorkflowOption],
        page: WeakEntity<Self>,
        provider_id: String,
        window: &mut Window,
        cx: &mut App,
        build_action: impl Fn(&ServiceWorkflowOption) -> ServiceWorkflowUiAction + 'static + Copy,
    ) -> Entity<ContextMenu> {
        ContextMenu::build(window, cx, move |mut menu, _, _| {
            for option in options {
                let page = page.clone();
                let provider_id = provider_id.clone();
                let action = build_action(option);
                let label = match &option.detail {
                    Some(detail) => format!("{} ({detail})", option.label),
                    None => option.label.clone(),
                };
                menu = menu.entry(label, None, move |window, cx| {
                    Self::dispatch_workflow_action(&page, &provider_id, action.clone(), window, cx);
                });
            }

            menu
        })
    }

    fn render_workflow_form(
        &self,
        page: WeakEntity<Self>,
        workflow_ui: ServiceWorkflowUiModel,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_2()
            .children(workflow_ui.form.fields.iter().map(|field| {
                match field {
                    ServiceWorkflowFieldState::Text { descriptor, input } => {
                        match descriptor.kind {
                            service_hub::ServiceInputKind::FilePath => h_flex()
                                .items_end()
                                .gap_2()
                                .child(input.clone())
                                .child(Self::render_action_chip(
                                    SharedString::from(format!(
                                        "browse-workflow-{}",
                                        descriptor.key
                                    )),
                                    "Browse",
                                    IconName::FolderOpen,
                                    workflow_ui.form.pending,
                                    {
                                        let page = page.clone();
                                        let provider_id = workflow_ui.provider_id.clone();
                                        let field_key = descriptor.key.clone();
                                        move |_, window, cx| {
                                            Self::dispatch_workflow_action(
                                                &page,
                                                &provider_id,
                                                ServiceWorkflowUiAction::PickFile {
                                                    field_key: field_key.clone(),
                                                },
                                                window,
                                                cx,
                                            );
                                        }
                                    },
                                    cx,
                                ))
                                .into_any_element(),
                            service_hub::ServiceInputKind::Text
                            | service_hub::ServiceInputKind::Toggle => {
                                input.clone().into_any_element()
                            }
                        }
                    }
                    ServiceWorkflowFieldState::Toggle { descriptor, value } => Checkbox::new(
                        SharedString::from(format!("workflow-toggle-{}", descriptor.key)),
                        *value,
                    )
                    .label(descriptor.label.clone())
                    .disabled(workflow_ui.form.pending)
                    .on_click(cx.listener({
                        let provider_id = workflow_ui.provider_id.clone();
                        let field_key = descriptor.key.clone();
                        move |page, checked, window, cx| {
                            let workspace = page.workspace().clone();
                            page.with_provider_mut(&provider_id, |pane, state| {
                                pane.handle_workflow_ui_action(
                                    state,
                                    ServiceWorkflowUiAction::SetToggle {
                                        field_key: field_key.clone(),
                                        value: *checked,
                                    },
                                    workspace,
                                    window,
                                    cx,
                                );
                            });
                            cx.notify();
                        }
                    }))
                    .into_any_element(),
                }
            }))
    }

    fn render_workflow_run(run: ServiceWorkflowRunSummary) -> (Color, String) {
        let color = match run.state {
            service_hub::ServiceRunState::Pending | service_hub::ServiceRunState::Running => {
                Color::Muted
            }
            service_hub::ServiceRunState::Warning => Color::Warning,
            service_hub::ServiceRunState::Succeeded => Color::Success,
            service_hub::ServiceRunState::Failed => Color::Error,
        };

        let text = if run.detail.is_empty() {
            run.headline
        } else {
            format!("{}: {}", run.headline, run.detail)
        };
        (color, text)
    }

    fn dispatch_workflow_action(
        page: &WeakEntity<Self>,
        provider_id: &str,
        action: ServiceWorkflowUiAction,
        window: &mut Window,
        cx: &mut App,
    ) {
        page.update(cx, |page, cx| {
            let workspace = page.workspace().clone();
            page.with_provider_mut(provider_id, |pane, state| {
                pane.handle_workflow_ui_action(state, action, workspace, window, cx);
            });
        })
        .ok();
    }
}

#[derive(IntoElement)]
struct ServiceSidebarMenuTrigger {
    div: Stateful<gpui::Div>,
    label: SharedString,
    start_image_path: Option<SharedString>,
    selected: bool,
}

impl ServiceSidebarMenuTrigger {
    fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            div: div().id(id.into()),
            label: label.into(),
            start_image_path: None,
            selected: false,
        }
    }

    #[cfg(target_os = "macos")]
    fn start_image_path(mut self, start_image_path: Option<String>) -> Self {
        self.start_image_path = start_image_path.map(Into::into);
        self
    }
}

impl Clickable for ServiceSidebarMenuTrigger {
    fn on_click(
        mut self,
        handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.div = self.div.on_click(handler);
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.div = self.div.cursor(cursor_style);
        self
    }
}

impl Toggleable for ServiceSidebarMenuTrigger {
    fn toggle_state(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl RenderOnce for ServiceSidebarMenuTrigger {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (text_color, background, hover_background, active_background) = match self.selected {
            false => (
                cx.theme().colors().text_muted,
                cx.theme().colors().tab_inactive_background.opacity(0.0),
                cx.theme().colors().text.opacity(0.09),
                cx.theme().colors().text.opacity(0.14),
            ),
            true => (
                cx.theme().colors().text,
                cx.theme().colors().text.opacity(0.14),
                cx.theme().colors().text.opacity(0.14),
                cx.theme().colors().text.opacity(0.20),
            ),
        };

        self.div
            .w_full()
            .h(px(28.))
            .bg(background)
            .rounded(cx.theme().component_radius().tab.unwrap_or(px(6.0)))
            .when(!self.selected, |this| {
                this.hover(move |style| style.bg(hover_background))
            })
            .active(move |style| style.bg(active_background))
            .cursor_pointer()
            .child(
                h_flex()
                    .w_full()
                    .h_full()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .gap_2()
                    .text_color(text_color)
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .when_some(self.start_image_path, |this, start_image_path| {
                                let start_image_path = start_image_path.to_string();
                                this.child(
                                    gpui::img(start_image_path)
                                        .w(rems_from_px(16.0))
                                        .h(rems_from_px(16.0))
                                        .rounded(px(4.0)),
                                )
                            })
                            .child(Label::new(self.label).size(LabelSize::Small).truncate()),
                    )
                    .child(
                        Icon::new(IconName::ChevronUpDown)
                            .size(IconSize::XSmall)
                            .color(Color::Muted),
                    ),
            )
    }
}

impl EventEmitter<ItemEvent> for ServicesPage {}

impl Focusable for ServicesPage {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for ServicesPage {
    type Event = ItemEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        if self.onboarding_visible {
            "Services".into()
        } else {
            self.provider().label.clone().into()
        }
    }

    fn tab_tooltip_text(&self, _cx: &App) -> Option<SharedString> {
        Some("Services".into())
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Server))
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("Services Page Opened")
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn buffer_kind(&self, _cx: &App) -> ItemBufferKind {
        ItemBufferKind::None
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        f(*event)
    }
}

impl Render for ServicesPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .p_5()
            .child(self.render_provider_content(window, cx))
    }
}

#[cfg(target_os = "macos")]
struct ServicesSidebarPanel {
    workspace: WeakEntity<Workspace>,
    observed_page: Option<WeakEntity<ServicesPage>>,
    _workspace_subscription: Option<Subscription>,
    _page_subscription: Option<Subscription>,
}

#[cfg(target_os = "macos")]
impl ServicesSidebarPanel {
    fn new(workspace: WeakEntity<Workspace>, cx: &mut Context<Self>) -> Self {
        let mut panel = Self {
            workspace,
            observed_page: None,
            _workspace_subscription: None,
            _page_subscription: None,
        };

        if let Some(workspace) = panel.workspace.upgrade() {
            panel._workspace_subscription = Some(cx.observe(&workspace, |this, _, cx| {
                this.sync_page_subscription(cx);
                cx.notify();
            }));
        }

        let this = cx.entity().downgrade();
        cx.defer(move |cx| {
            let Some(this) = this.upgrade() else {
                return;
            };

            this.update(cx, |this, cx| {
                this.sync_page_subscription(cx);
                cx.notify();
            });
        });
        panel
    }

    fn sync_page_subscription(&mut self, cx: &mut Context<Self>) {
        let next_page = self
            .workspace
            .upgrade()
            .and_then(|workspace| workspace.read(cx).item_of_type::<ServicesPage>(cx));
        let next_page_id = next_page.as_ref().map(|page| page.entity_id());
        let current_page_id = self
            .observed_page
            .as_ref()
            .and_then(|page| page.upgrade())
            .map(|page| page.entity_id());

        if next_page_id == current_page_id {
            return;
        }

        self._page_subscription = None;
        self.observed_page = next_page.as_ref().map(|page| page.downgrade());

        if let Some(page) = next_page {
            self._page_subscription = Some(cx.observe(&page, |this, _, cx| {
                this.sync_page_subscription(cx);
                cx.notify();
            }));
        }
    }
}

#[cfg(target_os = "macos")]
impl Render for ServicesSidebarPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(page) = self.observed_page.as_ref().and_then(|page| page.upgrade()) {
            return ServicesPage::render_sidebar_controls(&page, window, cx);
        }

        v_flex()
            .size_full()
            .justify_center()
            .p_4()
            .gap_2()
            .child(Label::new("Services").size(LabelSize::Large))
            .child(
                Label::new("Open the Service Hub to manage providers, apps, and releases.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .into_any_element()
    }
}
