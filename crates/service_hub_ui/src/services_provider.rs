use std::{any::Any, collections::BTreeMap, path::PathBuf};

use gpui::{App, Context, WeakEntity, Window};
use service_hub::ServiceProviderDescriptor;
use ui::{ActiveTheme, AnyElement, Color, Label, LabelCommon, LabelSize, prelude::*};
use workspace::Workspace;

#[cfg(target_os = "macos")]
use crate::service_auth::{ServiceAuthUiAction, ServiceAuthUiModel};
use crate::{
    app_store_connect_provider::build_app_store_connect_workspace_adapter,
    service_workflow::{ServiceWorkflowUiAction, ServiceWorkflowUiModel},
    services_page::ServicesPage,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServicesPageState {
    pub provider_id: String,
    pub navigation_id: String,
    pub selected_resource_id: Option<String>,
    pub selected_target_id: Option<String>,
    pub selected_workflow_id: Option<String>,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceResourceMenuEntry {
    pub id: String,
    pub label: String,
    pub detail: Option<String>,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceResourceMenuModel {
    pub singular_label: String,
    pub current_label: String,
    pub entries: Vec<ServiceResourceMenuEntry>,
    pub disabled: bool,
}

pub(crate) trait ServiceWorkspaceAdapter {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn descriptor(&self) -> &ServiceProviderDescriptor;
    fn normalize_state(&self, state: &mut ServicesPageState);
    fn refresh(
        &mut self,
        state: &mut ServicesPageState,
        workspace_paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    );
    #[cfg(target_os = "macos")]
    fn resource_menu(&self, state: &ServicesPageState) -> Option<ServiceResourceMenuModel>;
    #[cfg(target_os = "macos")]
    fn select_resource(
        &mut self,
        state: &mut ServicesPageState,
        resource_id: String,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    );
    fn render_section(
        &self,
        state: &ServicesPageState,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> AnyElement;
    fn workflow_ui_model(&self, _state: &ServicesPageState) -> Option<ServiceWorkflowUiModel> {
        None
    }
    fn render_workflow_form(
        &self,
        _state: &ServicesPageState,
        _workflow_ui: &ServiceWorkflowUiModel,
        _page: WeakEntity<ServicesPage>,
        _window: &mut Window,
        _cx: &mut Context<ServicesPage>,
    ) -> Option<AnyElement> {
        None
    }
    fn handle_workflow_ui_action(
        &mut self,
        _state: &mut ServicesPageState,
        _action: ServiceWorkflowUiAction,
        _workspace: WeakEntity<Workspace>,
        _window: &mut Window,
        _cx: &mut Context<ServicesPage>,
    ) {
    }
    #[cfg(target_os = "macos")]
    fn auth_ui_model(&self) -> Option<ServiceAuthUiModel> {
        None
    }
    #[cfg(target_os = "macos")]
    fn handle_auth_ui_action(
        &mut self,
        _state: &mut ServicesPageState,
        _action: ServiceAuthUiAction,
        _workspace: WeakEntity<Workspace>,
        _window: &mut Window,
        _cx: &mut Context<ServicesPage>,
    ) {
    }
    #[cfg(target_os = "macos")]
    fn render_sidebar_footer_extra(
        &self,
        _state: &ServicesPageState,
        _window: &mut Window,
        _cx: &mut Context<ServicesPage>,
    ) -> Option<AnyElement> {
        None
    }
}

pub(crate) type ServiceWorkspacePane = Box<dyn ServiceWorkspaceAdapter>;

pub(crate) fn build_service_workspace_panes(
    descriptors: Vec<ServiceProviderDescriptor>,
    window: &mut Window,
    cx: &mut App,
) -> BTreeMap<String, ServiceWorkspacePane> {
    descriptors
        .into_iter()
        .map(|descriptor| {
            let provider_id = descriptor.id.clone();
            (
                provider_id,
                build_service_workspace_adapter(descriptor, window, cx),
            )
        })
        .collect()
}

pub(crate) fn collect_provider_descriptors(
    panes: &BTreeMap<String, ServiceWorkspacePane>,
) -> Vec<ServiceProviderDescriptor> {
    panes
        .values()
        .map(|pane| pane.descriptor().clone())
        .collect()
}

pub(crate) fn normalize_services_page_state(
    providers: &[ServiceProviderDescriptor],
    initial_state: Option<ServicesPageState>,
) -> ServicesPageState {
    let initial_state_ref = initial_state.as_ref();
    let provider = initial_state
        .as_ref()
        .and_then(|state| {
            providers
                .iter()
                .find(|provider| provider.id == state.provider_id)
        })
        .or_else(|| providers.first())
        .expect("service hub should register at least one provider");

    let navigation_id = initial_state
        .as_ref()
        .map(|state| state.navigation_id.clone())
        .filter(|navigation_id| {
            provider
                .shell
                .navigation_items
                .iter()
                .any(|item| &item.id == navigation_id)
        })
        .unwrap_or_else(|| provider.shell.default_navigation_item_id.clone());

    ServicesPageState {
        provider_id: provider.id.clone(),
        navigation_id,
        selected_resource_id: initial_state_ref
            .and_then(|state| state.selected_resource_id.clone()),
        selected_target_id: initial_state_ref.and_then(|state| state.selected_target_id.clone()),
        selected_workflow_id: initial_state_ref
            .and_then(|state| state.selected_workflow_id.clone()),
    }
}

fn build_service_workspace_adapter(
    descriptor: ServiceProviderDescriptor,
    window: &mut Window,
    cx: &mut App,
) -> ServiceWorkspacePane {
    let adapter_builders: [fn(
        ServiceProviderDescriptor,
        &mut Window,
        &mut App,
    ) -> Option<ServiceWorkspacePane>; 1] = [build_app_store_connect_workspace_adapter];

    for build_adapter in adapter_builders {
        if let Some(adapter) = build_adapter(descriptor.clone(), window, cx) {
            return adapter;
        }
    }

    Box::new(UnavailableServiceWorkspacePane::new(descriptor))
}

pub(crate) struct UnavailableServiceWorkspacePane {
    descriptor: ServiceProviderDescriptor,
}

impl UnavailableServiceWorkspacePane {
    fn new(descriptor: ServiceProviderDescriptor) -> Self {
        Self { descriptor }
    }
}

impl ServiceWorkspaceAdapter for UnavailableServiceWorkspacePane {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn descriptor(&self) -> &ServiceProviderDescriptor {
        &self.descriptor
    }

    fn normalize_state(&self, state: &mut ServicesPageState) {
        if !self
            .descriptor
            .shell
            .navigation_items
            .iter()
            .any(|item| item.id == state.navigation_id)
        {
            state.navigation_id = self.descriptor.shell.default_navigation_item_id.clone();
        }
        state.selected_resource_id = None;
        state.selected_target_id = None;
        state.selected_workflow_id = None;
    }

    fn refresh(
        &mut self,
        _state: &mut ServicesPageState,
        _workspace_paths: Vec<PathBuf>,
        _window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        cx.notify();
    }

    #[cfg(target_os = "macos")]
    fn resource_menu(&self, _state: &ServicesPageState) -> Option<ServiceResourceMenuModel> {
        None
    }

    #[cfg(target_os = "macos")]
    fn select_resource(
        &mut self,
        _state: &mut ServicesPageState,
        _resource_id: String,
        _window: &mut Window,
        _cx: &mut Context<ServicesPage>,
    ) {
    }

    fn render_section(
        &self,
        state: &ServicesPageState,
        _window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> AnyElement {
        let section_label = self
            .descriptor
            .shell
            .navigation_items
            .iter()
            .find(|item| item.id == state.navigation_id)
            .map(|item| item.label.clone())
            .unwrap_or_else(|| "Overview".to_string());

        v_flex()
            .w_full()
            .gap_2()
            .p_5()
            .rounded_xl()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().background)
            .child(Label::new(self.descriptor.label.clone()))
            .child(
                Label::new(format!(
                    "{} is defined in service metadata, but there is no UI adapter for it yet.",
                    section_label
                ))
                .size(LabelSize::Small)
                .color(Color::Muted),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use service_hub::{ServiceAuthKind, ServiceProviderDescriptor, ServiceShellDescriptor};

    use super::{ServicesPageState, normalize_services_page_state};

    fn test_provider(id: &str, navigation_ids: &[&str]) -> ServiceProviderDescriptor {
        ServiceProviderDescriptor {
            id: id.to_string(),
            label: id.to_string(),
            logo_asset_path: None,
            shell: ServiceShellDescriptor {
                resource_kind: None,
                navigation_items: navigation_ids
                    .iter()
                    .map(
                        |navigation_id| service_hub::ServiceNavigationItemDescriptor {
                            id: (*navigation_id).to_string(),
                            label: (*navigation_id).to_string(),
                        },
                    )
                    .collect(),
                default_navigation_item_id: navigation_ids[0].to_string(),
            },
            auth_kind: ServiceAuthKind::None,
            auth: None,
            targets: Vec::new(),
            workflows: Vec::new(),
        }
    }

    #[test]
    fn normalizes_invalid_provider_to_first_registered_provider() {
        let providers = vec![
            test_provider("app-store-connect", &["overview", "builds"]),
            test_provider("vercel", &["overview"]),
        ];

        let state = normalize_services_page_state(
            &providers,
            Some(ServicesPageState {
                provider_id: "missing".to_string(),
                navigation_id: "missing".to_string(),
                selected_resource_id: Some("resource-1".to_string()),
                selected_target_id: Some("testflight".to_string()),
                selected_workflow_id: Some("status".to_string()),
            }),
        );

        assert_eq!(state.provider_id, "app-store-connect");
        assert_eq!(state.navigation_id, "overview");
        assert_eq!(state.selected_resource_id.as_deref(), Some("resource-1"));
        assert_eq!(state.selected_target_id.as_deref(), Some("testflight"));
        assert_eq!(state.selected_workflow_id.as_deref(), Some("status"));
    }

    #[test]
    fn normalizes_invalid_navigation_to_provider_default() {
        let providers = vec![test_provider("app-store-connect", &["overview", "builds"])];

        let state = normalize_services_page_state(
            &providers,
            Some(ServicesPageState {
                provider_id: "app-store-connect".to_string(),
                navigation_id: "releases".to_string(),
                selected_resource_id: None,
                selected_target_id: None,
                selected_workflow_id: None,
            }),
        );

        assert_eq!(state.provider_id, "app-store-connect");
        assert_eq!(state.navigation_id, "overview");
    }
}
