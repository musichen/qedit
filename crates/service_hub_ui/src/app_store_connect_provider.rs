use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result};
use app_runtime::{ProjectKind, RuntimeCatalog, SystemCommandRunner};
use gpui::{
    App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, Render, ScrollHandle,
    SharedString, WeakEntity, Window, px,
};
use project::DirectoryLister;
use serde::Deserialize;
use service_hub::{
    ServiceOperationRequest, ServiceProviderDescriptor, ServiceResourceRef, ServiceRunDescriptor,
    ServiceRunState, ServiceWorkflowDescriptor, ServiceWorkflowKind, ServiceWorkflowRequest,
};
#[cfg(target_os = "macos")]
use ui::Severity;
use ui::{
    AnyElement, Button, ButtonSize, ButtonStyle, Checkbox, Color, ContextMenu, IconButton,
    IconName, Indicator, Label, LabelSize, Modal, ModalFooter, ModalHeader, ToggleState,
    WithScrollbar, h_flex, prelude::*, v_flex,
};
use ui_input::InputField;
use util::command::new_command;
use workspace::{DismissDecision, ModalView, Workspace};

#[cfg(target_os = "macos")]
use crate::{
    app_store_connect_auth::{AscAuthSummary, load_auth_status},
    command_runner::run_auth_action,
    service_auth::{
        ServiceAuthFormState, ServiceAuthStatusSummary, ServiceAuthUiAction, ServiceAuthUiModel,
    },
    services_provider::{ServiceResourceMenuEntry, ServiceResourceMenuModel},
};
use crate::{
    command_runner::{run_json_operation, run_workflow},
    service_workflow::{
        ServiceWorkflowFormState, ServiceWorkflowOption, ServiceWorkflowRunSummary,
        ServiceWorkflowUiAction, ServiceWorkflowUiModel,
    },
    services_page::ServicesPage,
    services_provider::{ServiceWorkspaceAdapter, ServicesPageState},
};

pub(crate) const APP_STORE_CONNECT_PROVIDER_ID: &str = "app-store-connect";
const ASC_BUILDS_PAGE_SIZE: usize = 50;
const ASC_CLI_INSTALL_URL: &str = "https://github.com/rudrankriyam/App-Store-Connect-CLI#1-install";
const ASC_CLI_GITHUB_URL: &str = "https://github.com/rudrankriyam/App-Store-Connect-CLI";

pub(crate) fn build_app_store_connect_workspace_adapter(
    descriptor: ServiceProviderDescriptor,
    window: &mut Window,
    cx: &mut App,
) -> Option<Box<dyn ServiceWorkspaceAdapter>> {
    (descriptor.id == APP_STORE_CONNECT_PROVIDER_ID).then(|| {
        Box::new(AppStoreConnectWorkspaceProvider::new(
            descriptor, window, cx,
        )) as Box<dyn ServiceWorkspaceAdapter>
    })
}

fn with_app_store_connect_provider_mut<R>(
    page: &mut ServicesPage,
    callback: impl FnOnce(&mut AppStoreConnectWorkspaceProvider, &mut ServicesPageState) -> R,
) -> Option<R> {
    page.with_provider_mut(APP_STORE_CONNECT_PROVIDER_ID, |pane, state| {
        pane.as_any_mut()
            .downcast_mut::<AppStoreConnectWorkspaceProvider>()
            .map(|provider| callback(provider, state))
    })
    .flatten()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AscAppSummary {
    id: String,
    name: String,
    bundle_id: String,
    sku: String,
    primary_locale: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AscBuildSummary {
    id: String,
    build_number: String,
    marketing_version: Option<String>,
    platform: Option<String>,
    processing_state: String,
    uploaded_date: String,
    expiration_date: Option<String>,
    testflight_internal_state: Option<String>,
    testflight_external_state: Option<String>,
    app_store_version_id: Option<String>,
    app_store_state: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AscBuildListState {
    builds: Vec<AscBuildSummary>,
    next_page_url: Option<String>,
    is_loading_more: bool,
    load_more_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AscBuildPage {
    builds: Vec<AscBuildSummary>,
    next_page_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AscCliSummary {
    path: String,
    version: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AscCliState {
    Checking,
    Ready(AscCliSummary),
    Missing(String),
    Installing,
    InstallFailed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AppleWorkspaceSchemeSummary {
    id: String,
    label: String,
    bundle_id: Option<String>,
    marketing_version: Option<String>,
    build_number: Option<String>,
    platform: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AppleWorkspaceProjectSummary {
    id: String,
    label: String,
    project_path: PathBuf,
    project_kind: ProjectKind,
    schemes: Vec<AppleWorkspaceSchemeSummary>,
}

impl AppleWorkspaceProjectSummary {
    fn display_path(&self) -> String {
        self.project_path.to_string_lossy().into_owned()
    }

    fn root_path(&self) -> &Path {
        self.project_path
            .parent()
            .unwrap_or(self.project_path.as_path())
    }

    fn default_export_options_path(&self) -> Option<PathBuf> {
        let path = self.root_path().join(".asc/export-options-app-store.plist");
        path.exists().then_some(path)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AscWebAuthSummary {
    authenticated: bool,
}

#[derive(Clone, Debug)]
enum LoadState<T> {
    Loading,
    Ready(T),
    Error(String),
}

#[derive(Clone)]
struct AscCreateAppFormState {
    selected_project_id: Option<String>,
    selected_scheme_id: Option<String>,
    selected_platform: String,
    app_name_input: Entity<InputField>,
    bundle_id_input: Entity<InputField>,
    sku_input: Entity<InputField>,
    primary_locale_input: Entity<InputField>,
    initial_version_input: Entity<InputField>,
    company_name_input: Entity<InputField>,
    apple_id_input: Entity<InputField>,
    password_input: Entity<InputField>,
    two_factor_command_input: Entity<InputField>,
    create_internal_group: ToggleState,
    internal_group_name_input: Entity<InputField>,
    pending: bool,
    error_message: Option<SharedString>,
    success_message: Option<SharedString>,
}

impl AscCreateAppFormState {
    fn new(window: &mut Window, cx: &mut App) -> Self {
        Self {
            selected_project_id: None,
            selected_scheme_id: None,
            selected_platform: "IOS".to_string(),
            app_name_input: new_text_input(window, cx, "App Name", "IOSSample", false),
            bundle_id_input: new_text_input(window, cx, "Bundle ID", "com.example.app", false),
            sku_input: new_text_input(window, cx, "SKU", "com.example.app", false),
            primary_locale_input: new_text_input(window, cx, "Primary Locale", "en-US", false),
            initial_version_input: new_text_input(window, cx, "Initial Version", "1.0", false),
            company_name_input: new_text_input(window, cx, "Company Name", "Qedit", false),
            apple_id_input: new_text_input(
                window,
                cx,
                "Apple Account Email",
                "name@example.com",
                false,
            ),
            password_input: new_text_input(window, cx, "Apple Account Password", "", true),
            two_factor_command_input: new_text_input(
                window,
                cx,
                "2FA Command",
                "osascript /path/to/get-apple-2fa-code.scpt",
                false,
            ),
            create_internal_group: ToggleState::Unselected,
            internal_group_name_input: new_text_input(
                window,
                cx,
                "Internal TestFlight Group",
                "Internal Testers",
                false,
            ),
            pending: false,
            error_message: None,
            success_message: None,
        }
    }

    fn clear_status(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }

    fn set_pending(&mut self, pending: bool) {
        self.pending = pending;
        if pending {
            self.clear_status();
        }
    }

    fn set_error(&mut self, error: impl Into<SharedString>) {
        self.pending = false;
        self.error_message = Some(error.into());
        self.success_message = None;
    }

    fn finish_success(&mut self, message: impl Into<SharedString>) {
        self.pending = false;
        self.error_message = None;
        self.success_message = Some(message.into());
    }

    fn set_defaults_for_scheme(
        &mut self,
        project: &AppleWorkspaceProjectSummary,
        scheme: &AppleWorkspaceSchemeSummary,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.selected_project_id = Some(project.id.clone());
        self.selected_scheme_id = Some(scheme.id.clone());
        self.selected_platform = scheme.platform.clone().unwrap_or_else(|| "IOS".to_string());
        set_input_text(&self.app_name_input, &scheme.label, window, cx);
        set_input_text(
            &self.bundle_id_input,
            scheme.bundle_id.as_deref().unwrap_or_default(),
            window,
            cx,
        );
        set_input_text(
            &self.sku_input,
            scheme
                .bundle_id
                .as_deref()
                .unwrap_or_else(|| scheme.label.as_str()),
            window,
            cx,
        );
        set_input_text(&self.primary_locale_input, "en-US", window, cx);
        set_input_text(
            &self.initial_version_input,
            scheme.marketing_version.as_deref().unwrap_or("1.0"),
            window,
            cx,
        );
        self.clear_status();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AscTestFlightSourceMode {
    LocalProject,
    IpaFile,
    ExistingBuild,
}

impl AscTestFlightSourceMode {
    fn label(self) -> &'static str {
        match self {
            Self::LocalProject => "Local Project",
            Self::IpaFile => "IPA File",
            Self::ExistingBuild => "Existing Build",
        }
    }
}

#[derive(Clone, Debug)]
struct AscTestFlightWorkflowValues {
    source_mode: AscTestFlightSourceMode,
    project_kind: Option<ProjectKind>,
    project_path: Option<PathBuf>,
    scheme: Option<AppleWorkspaceSchemeSummary>,
    ipa_path: String,
    version: String,
    build_id: String,
    build_number: String,
    group: String,
    configuration: String,
    export_options: String,
    wait_for_processing: bool,
    notify_testers: bool,
    clean_build: bool,
}

#[derive(Clone)]
struct AscTestFlightReleaseFormState {
    source_mode: AscTestFlightSourceMode,
    selected_project_id: Option<String>,
    selected_scheme_id: Option<String>,
    ipa_path_input: Entity<InputField>,
    version_input: Entity<InputField>,
    build_id_input: Entity<InputField>,
    build_number_input: Entity<InputField>,
    group_input: Entity<InputField>,
    configuration_input: Entity<InputField>,
    export_options_input: Entity<InputField>,
    wait_for_processing: ToggleState,
    notify_testers: ToggleState,
    clean_build: ToggleState,
}

impl AscTestFlightReleaseFormState {
    fn new(window: &mut Window, cx: &mut App) -> Self {
        Self {
            source_mode: AscTestFlightSourceMode::LocalProject,
            selected_project_id: None,
            selected_scheme_id: None,
            ipa_path_input: new_text_input(window, cx, "IPA Path", "./build/Qedit.ipa", false),
            version_input: new_text_input(window, cx, "Version", "1.2.3", false),
            build_id_input: new_text_input(window, cx, "Build ID", "BUILD_ID", false),
            build_number_input: new_text_input(window, cx, "Build Number", "42", false),
            group_input: new_text_input(window, cx, "Beta Groups", "Internal Testers", false),
            configuration_input: new_text_input(window, cx, "Configuration", "Release", false),
            export_options_input: new_text_input(
                window,
                cx,
                "Export Options",
                ".asc/export-options-app-store.plist",
                false,
            ),
            wait_for_processing: ToggleState::Unselected,
            notify_testers: ToggleState::Unselected,
            clean_build: ToggleState::Unselected,
        }
    }

    fn set_defaults_for_scheme(
        &mut self,
        project: &AppleWorkspaceProjectSummary,
        scheme: &AppleWorkspaceSchemeSummary,
        window: &mut Window,
        cx: &mut App,
    ) {
        let previous_project_id = self.selected_project_id.clone();
        self.selected_project_id = Some(project.id.clone());
        self.selected_scheme_id = Some(scheme.id.clone());
        set_input_text(
            &self.version_input,
            scheme.marketing_version.as_deref().unwrap_or("1.0"),
            window,
            cx,
        );
        if trimmed_input_text(&self.configuration_input, cx).is_empty() {
            set_input_text(&self.configuration_input, "Release", window, cx);
        }
        if let Some(path) = project.default_export_options_path() {
            set_input_text(
                &self.export_options_input,
                &path.to_string_lossy(),
                window,
                cx,
            );
        } else if previous_project_id.as_deref() != Some(project.id.as_str()) {
            set_input_text(&self.export_options_input, "", window, cx);
        }
    }

    fn build_workflow_input(
        &self,
        projects: &[AppleWorkspaceProjectSummary],
        cx: &App,
    ) -> Result<BTreeMap<String, String>, SharedString> {
        let selected_project = self
            .selected_project_id
            .as_ref()
            .and_then(|selected_id| projects.iter().find(|project| &project.id == selected_id));
        let selected_scheme = selected_project.and_then(|project| {
            self.selected_scheme_id.as_ref().and_then(|selected_id| {
                project
                    .schemes
                    .iter()
                    .find(|scheme| &scheme.id == selected_id)
            })
        });

        build_testflight_workflow_input(AscTestFlightWorkflowValues {
            source_mode: self.source_mode,
            project_kind: selected_project.map(|project| project.project_kind.clone()),
            project_path: selected_project.map(|project| project.project_path.clone()),
            scheme: selected_scheme.cloned(),
            ipa_path: trimmed_input_text(&self.ipa_path_input, cx),
            version: trimmed_input_text(&self.version_input, cx),
            build_id: trimmed_input_text(&self.build_id_input, cx),
            build_number: trimmed_input_text(&self.build_number_input, cx),
            group: trimmed_input_text(&self.group_input, cx),
            configuration: trimmed_input_text(&self.configuration_input, cx),
            export_options: trimmed_input_text(&self.export_options_input, cx),
            wait_for_processing: self.wait_for_processing.selected(),
            notify_testers: self.notify_testers.selected(),
            clean_build: self.clean_build.selected(),
        })
    }
}

fn build_testflight_workflow_input(
    values: AscTestFlightWorkflowValues,
) -> Result<BTreeMap<String, String>, SharedString> {
    let mut input = BTreeMap::new();

    if values.group.is_empty() {
        return Err("Beta Groups is required".into());
    }
    input.insert("group".to_string(), values.group);

    match values.source_mode {
        AscTestFlightSourceMode::LocalProject => {
            let project_kind = values
                .project_kind
                .ok_or_else(|| SharedString::from("Workspace Project is required"))?;
            let project_path = values
                .project_path
                .ok_or_else(|| SharedString::from("Workspace Project is required"))?;
            let scheme = values
                .scheme
                .ok_or_else(|| SharedString::from("Scheme is required"))?;

            match project_kind {
                ProjectKind::AppleWorkspace => {
                    input.insert(
                        "workspace_path".to_string(),
                        project_path.to_string_lossy().into_owned(),
                    );
                }
                ProjectKind::AppleProject => {
                    input.insert(
                        "project_path".to_string(),
                        project_path.to_string_lossy().into_owned(),
                    );
                }
                ProjectKind::GpuiApplication => {
                    return Err(
                        "TestFlight local build requires an Apple project or workspace.".into(),
                    );
                }
            }

            input.insert("scheme".to_string(), scheme.label);

            if !values.version.is_empty() {
                input.insert("version".to_string(), values.version);
            }
            if !values.configuration.is_empty() {
                input.insert("configuration".to_string(), values.configuration);
            }
            if values.export_options.is_empty() {
                return Err("Export Options is required for local project publishing.".into());
            }
            input.insert("export_options".to_string(), values.export_options);
            if let Some(platform) = scheme.platform {
                input.insert("platform".to_string(), platform);
            }
            if values.clean_build {
                input.insert("clean".to_string(), "true".to_string());
            }
        }
        AscTestFlightSourceMode::IpaFile => {
            if values.ipa_path.is_empty() {
                return Err("IPA Path is required".into());
            }
            input.insert("ipa_path".to_string(), values.ipa_path);
            if !values.version.is_empty() {
                input.insert("version".to_string(), values.version);
            }
        }
        AscTestFlightSourceMode::ExistingBuild => {
            if values.build_id.is_empty() && values.build_number.is_empty() {
                return Err("Build ID or Build Number is required".into());
            }
            if !values.build_id.is_empty() {
                input.insert("build_id".to_string(), values.build_id);
            }
            if !values.build_number.is_empty() {
                input.insert("build_number".to_string(), values.build_number);
            }
        }
    }

    if values.wait_for_processing {
        input.insert("wait".to_string(), "true".to_string());
    }
    if values.notify_testers {
        input.insert("notify".to_string(), "true".to_string());
    }

    Ok(input)
}

fn validate_local_project_bundle_id_match(
    selected_app_bundle_id: &str,
    scheme: &AppleWorkspaceSchemeSummary,
) -> Result<(), SharedString> {
    let scheme_bundle_id = scheme
        .bundle_id
        .as_deref()
        .map(str::trim)
        .filter(|bundle_id| !bundle_id.is_empty())
        .ok_or_else(|| {
            SharedString::from(
                "Qedit could not read the selected scheme bundle ID. Check PRODUCT_BUNDLE_IDENTIFIER for the Release configuration before publishing.",
            )
        })?;

    if scheme_bundle_id == selected_app_bundle_id {
        return Ok(());
    }

    Err(format!(
        "The selected App Store Connect app uses bundle ID `{selected_app_bundle_id}`, but the local scheme `{}` builds `{scheme_bundle_id}`. Choose the matching app or switch schemes before publishing.",
        scheme.label
    )
    .into())
}

fn new_text_input(
    window: &mut Window,
    cx: &mut App,
    label: &str,
    placeholder: &str,
    masked: bool,
) -> Entity<InputField> {
    cx.new(|cx| {
        InputField::new(window, cx, placeholder)
            .label(label.to_string())
            .tab_stop(true)
            .masked(masked)
    })
}

fn set_input_text(input: &Entity<InputField>, text: &str, window: &mut Window, cx: &mut App) {
    input.update(cx, |input, cx| {
        input.set_text(text, window, cx);
    });
}

fn new_two_factor_code_file_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    std::env::temp_dir().join(format!("qedit-asc-2fa-{timestamp}.txt"))
}

fn two_factor_wait_command(path: &Path) -> String {
    let path = path.to_string_lossy().replace('"', "\\\"");
    format!("sh -lc 'while [ ! -s \"{path}\" ]; do sleep 1; done; cat \"{path}\"'")
}

fn persist_two_factor_code(path: &Path, code: &str) -> std::io::Result<()> {
    std::fs::write(path, code.trim())
}

fn two_factor_code_file_has_contents(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|contents| !contents.trim().is_empty())
        .unwrap_or(false)
}

pub(crate) struct AppStoreConnectWorkspaceProvider {
    descriptor: ServiceProviderDescriptor,
    #[cfg(target_os = "macos")]
    auth_form: ServiceAuthFormState,
    workflow_forms: BTreeMap<String, ServiceWorkflowFormState>,
    create_app_form: AscCreateAppFormState,
    testflight_release_form: AscTestFlightReleaseFormState,
    latest_run: Option<ServiceRunDescriptor>,
    cli_state: AscCliState,
    #[cfg(target_os = "macos")]
    auth_state: LoadState<AscAuthSummary>,
    web_auth_state: LoadState<AscWebAuthSummary>,
    workspace_projects_state: LoadState<Vec<AppleWorkspaceProjectSummary>>,
    apps_state: LoadState<Vec<AscAppSummary>>,
    builds_state: LoadState<AscBuildListState>,
    content_scroll_handle: ScrollHandle,
    builds_scroll_handle: ScrollHandle,
}

impl AppStoreConnectWorkspaceProvider {
    fn panel_radius(cx: &App) -> gpui::Pixels {
        cx.theme().component_radius().panel.unwrap_or(px(10.0))
    }

    fn cli_ready(&self) -> bool {
        matches!(self.cli_state, AscCliState::Ready(_))
    }

    // Failure modes:
    // - Authentication checks fail or return partial data.
    // - App listing fails or returns no apps, leaving the shell without a resource selection.
    // - A selected app disappears between refreshes and the shell must recover cleanly.
    // - Build loading fails independently from auth or app loading.
    pub fn new(descriptor: ServiceProviderDescriptor, window: &mut Window, cx: &mut App) -> Self {
        Self {
            #[cfg(target_os = "macos")]
            auth_form: ServiceAuthFormState::new(&descriptor, window, cx),
            workflow_forms: descriptor
                .workflows
                .iter()
                .map(|workflow| {
                    (
                        workflow.id.clone(),
                        ServiceWorkflowFormState::new(workflow, window, cx),
                    )
                })
                .collect(),
            create_app_form: AscCreateAppFormState::new(window, cx),
            testflight_release_form: AscTestFlightReleaseFormState::new(window, cx),
            latest_run: None,
            descriptor,
            cli_state: AscCliState::Checking,
            #[cfg(target_os = "macos")]
            auth_state: LoadState::Loading,
            web_auth_state: LoadState::Loading,
            workspace_projects_state: LoadState::Loading,
            apps_state: LoadState::Loading,
            builds_state: LoadState::Ready(AscBuildListState::default()),
            content_scroll_handle: ScrollHandle::new(),
            builds_scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn descriptor(&self) -> &ServiceProviderDescriptor {
        &self.descriptor
    }

    pub fn normalize_state(&self, state: &mut ServicesPageState) {
        if !self
            .descriptor
            .shell
            .navigation_items
            .iter()
            .any(|item| item.id == state.navigation_id)
        {
            state.navigation_id = self.descriptor.shell.default_navigation_item_id.clone();
        }

        if let LoadState::Ready(apps) = &self.apps_state {
            if !apps
                .iter()
                .any(|app| Some(app.id.as_str()) == state.selected_resource_id.as_deref())
            {
                state.selected_resource_id = apps.first().map(|app| app.id.clone());
            }
        }

        self.normalize_workflow_state(state);
    }

    pub fn refresh(
        &mut self,
        _state: &mut ServicesPageState,
        workspace_paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        self.cli_state = AscCliState::Checking;
        #[cfg(target_os = "macos")]
        {
            self.auth_state = LoadState::Loading;
        }
        self.web_auth_state = LoadState::Loading;
        self.workspace_projects_state = LoadState::Loading;
        self.apps_state = LoadState::Loading;
        self.builds_state = LoadState::Ready(AscBuildListState::default());
        self.latest_run = None;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            #[cfg(target_os = "macos")]
            let (cli_state, auth_result, web_auth_result, projects_result, apps_result) = cx
                .background_spawn(async move {
                    let cli_state = load_asc_cli_state();
                    if !matches!(cli_state, AscCliState::Ready(_)) {
                        let projects = load_workspace_projects(workspace_paths).await;
                        return (cli_state, None, None, Some(projects), None);
                    }

                    let auth = load_auth_status().await;
                    let web_auth = load_web_auth_status().await;
                    let projects = load_workspace_projects(workspace_paths).await;
                    let apps = load_apps().await;
                    (
                        cli_state,
                        Some(auth),
                        Some(web_auth),
                        Some(projects),
                        Some(apps),
                    )
                })
                .await;

            #[cfg(not(target_os = "macos"))]
            let (cli_state, web_auth_result, projects_result, apps_result) = cx
                .background_spawn(async move {
                    let cli_state = load_asc_cli_state();
                    if !matches!(cli_state, AscCliState::Ready(_)) {
                        let projects = load_workspace_projects(workspace_paths).await;
                        return (cli_state, None, Some(projects), None);
                    }

                    let web_auth = load_web_auth_status().await;
                    let projects = load_workspace_projects(workspace_paths).await;
                    let apps = load_apps().await;
                    (cli_state, Some(web_auth), Some(projects), Some(apps))
                })
                .await;

            let selected_app_id = this
                .update_in(cx, |page, window, cx| {
                    with_app_store_connect_provider_mut(page, |pane, state| {
                        pane.cli_state = cli_state.clone();

                        if !pane.cli_ready() {
                            #[cfg(target_os = "macos")]
                            {
                                pane.auth_form.cancel();
                                pane.auth_state = LoadState::Error(asc_cli_missing_message());
                            }
                            pane.web_auth_state = LoadState::Error(asc_cli_missing_message());
                            pane.workspace_projects_state = match projects_result
                                .expect("workspace projects should always be loaded")
                            {
                                Ok(projects) => LoadState::Ready(projects),
                                Err(error) => LoadState::Error(error.to_string()),
                            };
                            pane.apps_state = LoadState::Ready(Vec::new());
                            pane.builds_state = LoadState::Ready(AscBuildListState::default());
                            state.selected_resource_id = None;
                            cx.notify();
                            return None;
                        }

                        #[cfg(target_os = "macos")]
                        {
                            pane.auth_state = match auth_result
                                .expect("auth should load when ASC CLI is ready")
                            {
                                Ok(summary) => LoadState::Ready(summary),
                                Err(error) => LoadState::Error(error.to_string()),
                            };
                        }
                        pane.web_auth_state = match web_auth_result
                            .expect("web auth should load when ASC CLI is ready")
                        {
                            Ok(summary) => LoadState::Ready(summary),
                            Err(error) => LoadState::Error(error.to_string()),
                        };
                        pane.workspace_projects_state = match projects_result
                            .expect("workspace projects should load when ASC CLI is ready")
                        {
                            Ok(projects) => {
                                pane.normalize_project_forms(&projects, window, cx);
                                LoadState::Ready(projects)
                            }
                            Err(error) => LoadState::Error(error.to_string()),
                        };

                        match apps_result.expect("apps should load when ASC CLI is ready") {
                            Ok(apps) => {
                                let next_selected_app_id = state
                                    .selected_resource_id
                                    .as_ref()
                                    .and_then(|selected_id| {
                                        apps.iter()
                                            .find(|app| &app.id == selected_id)
                                            .map(|app| app.id.clone())
                                    })
                                    .or_else(|| apps.first().map(|app| app.id.clone()));

                                pane.apps_state = LoadState::Ready(apps);
                                state.selected_resource_id = next_selected_app_id.clone();
                                pane.builds_state = if next_selected_app_id.is_some() {
                                    LoadState::Loading
                                } else {
                                    LoadState::Ready(AscBuildListState::default())
                                };
                                cx.notify();
                                next_selected_app_id
                            }
                            Err(error) => {
                                pane.apps_state = LoadState::Error(error.to_string());
                                state.selected_resource_id = None;
                                pane.builds_state = LoadState::Ready(AscBuildListState::default());
                                cx.notify();
                                None
                            }
                        }
                    })
                })
                .ok()
                .flatten()
                .flatten();

            if let Some(app_id) = selected_app_id {
                this.update_in(cx, |page, window, cx| {
                    with_app_store_connect_provider_mut(page, |pane, state| {
                        pane.load_builds_for_app(state, app_id, window, cx);
                    });
                })
                .ok();
            }
        })
        .detach();
    }

    #[cfg(target_os = "macos")]
    pub fn resource_menu(&self, state: &ServicesPageState) -> Option<ServiceResourceMenuModel> {
        if !self.cli_ready() {
            return None;
        }

        let resource_kind = self.descriptor.shell.resource_kind.as_ref()?;
        let current_label = match &self.apps_state {
            LoadState::Loading => format!("Loading {}…", resource_kind.plural_label),
            LoadState::Error(_) => format!("Select {}", resource_kind.singular_label),
            LoadState::Ready(apps) if apps.is_empty() => {
                format!("No {}", resource_kind.plural_label)
            }
            LoadState::Ready(apps) => state
                .selected_resource_id
                .as_ref()
                .and_then(|selected_id| apps.iter().find(|app| &app.id == selected_id))
                .map(|app| app.name.clone())
                .unwrap_or_else(|| format!("Select {}", resource_kind.singular_label)),
        };

        Some(ServiceResourceMenuModel {
            singular_label: resource_kind.singular_label.clone(),
            current_label,
            entries: self
                .apps()
                .iter()
                .map(|app| ServiceResourceMenuEntry {
                    id: app.id.clone(),
                    label: app.name.clone(),
                    detail: Some(app.bundle_id.clone()),
                })
                .collect(),
            disabled: matches!(self.apps_state, LoadState::Loading) || self.apps().is_empty(),
        })
    }

    #[cfg(target_os = "macos")]
    pub fn select_resource(
        &mut self,
        state: &mut ServicesPageState,
        resource_id: String,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        if !self.cli_ready() {
            return;
        }

        if state.selected_resource_id.as_ref() == Some(&resource_id) {
            return;
        }

        self.load_builds_for_app(state, resource_id, window, cx);
    }

    pub fn render_section(
        &self,
        state: &ServicesPageState,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> AnyElement {
        if !self.cli_ready() {
            return self
                .render_cli_requirement_card(window, cx)
                .into_any_element();
        }

        match state.navigation_id.as_str() {
            "builds" => self
                .render_builds_content(state, window, cx)
                .into_any_element(),
            "release" => self
                .render_release_content(state, window, cx)
                .into_any_element(),
            _ => self
                .render_overview_content(state, window, cx)
                .into_any_element(),
        }
    }

    fn install_cli(&mut self, window: &mut Window, cx: &mut Context<ServicesPage>) {
        if matches!(self.cli_state, AscCliState::Installing) {
            return;
        }

        self.cli_state = AscCliState::Installing;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let install_result = install_asc_cli().await;

            this.update_in(cx, |page, window, cx| {
                let workspace_paths = page.workspace_paths().to_vec();
                with_app_store_connect_provider_mut(page, |pane, state| match install_result {
                    Ok(()) => pane.refresh(state, workspace_paths, window, cx),
                    Err(error) => {
                        pane.cli_state = AscCliState::InstallFailed(error.to_string());
                        cx.notify();
                    }
                });
            })
            .ok();
        })
        .detach();
    }

    fn navigation_workflows(&self, state: &ServicesPageState) -> Vec<&ServiceWorkflowDescriptor> {
        self.descriptor
            .workflows
            .iter()
            .filter(|workflow| {
                state.navigation_id == "release" && workflow.kind == ServiceWorkflowKind::Release
            })
            .collect()
    }

    fn available_targets<'a>(
        &'a self,
        workflows: &[&'a ServiceWorkflowDescriptor],
    ) -> Vec<ServiceWorkflowOption> {
        let supported_target_ids = workflows
            .iter()
            .flat_map(|workflow| workflow.target_ids.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>();

        self.descriptor
            .targets
            .iter()
            .filter(|target| supported_target_ids.contains(&target.id))
            .map(|target| ServiceWorkflowOption {
                id: target.id.clone(),
                label: target.label.clone(),
                detail: target.detail.clone(),
            })
            .collect()
    }

    fn available_workflows(&self, state: &ServicesPageState) -> Vec<ServiceWorkflowOption> {
        self.navigation_workflows(state)
            .into_iter()
            .filter(|workflow| workflow.supports_target(state.selected_target_id.as_deref()))
            .map(|workflow| ServiceWorkflowOption {
                id: workflow.id.clone(),
                label: workflow.label.clone(),
                detail: Some(workflow.detail.clone()),
            })
            .collect()
    }

    fn selected_workflow_descriptor(
        &self,
        state: &ServicesPageState,
    ) -> Option<&ServiceWorkflowDescriptor> {
        let selected_workflow_id = state.selected_workflow_id.as_deref()?;
        self.navigation_workflows(state)
            .into_iter()
            .find(|workflow| workflow.id == selected_workflow_id)
    }

    fn selected_workflow_form(
        &self,
        state: &ServicesPageState,
    ) -> Option<&ServiceWorkflowFormState> {
        let workflow_id = state.selected_workflow_id.as_ref()?;
        self.workflow_form_by_id(workflow_id)
    }

    fn selected_workflow_form_mut(
        &mut self,
        state: &ServicesPageState,
    ) -> Option<&mut ServiceWorkflowFormState> {
        let workflow_id = state.selected_workflow_id.as_ref()?;
        self.workflow_form_by_id_mut(workflow_id)
    }

    fn workflow_form_by_id(&self, workflow_id: &str) -> Option<&ServiceWorkflowFormState> {
        self.workflow_forms.get(workflow_id)
    }

    fn workflow_form_by_id_mut(
        &mut self,
        workflow_id: &str,
    ) -> Option<&mut ServiceWorkflowFormState> {
        self.workflow_forms.get_mut(workflow_id)
    }

    fn normalize_workflow_state(&self, state: &mut ServicesPageState) {
        let workflows = self.navigation_workflows(state);
        if workflows.is_empty() {
            state.selected_target_id = None;
            state.selected_workflow_id = None;
            return;
        }

        let available_targets = self.available_targets(&workflows);
        if available_targets.is_empty() {
            state.selected_target_id = None;
        } else if !available_targets
            .iter()
            .any(|target| Some(target.id.as_str()) == state.selected_target_id.as_deref())
        {
            state.selected_target_id = available_targets.first().map(|target| target.id.clone());
        }

        let available_workflows = self.available_workflows(state);
        if !available_workflows
            .iter()
            .any(|workflow| Some(workflow.id.as_str()) == state.selected_workflow_id.as_deref())
        {
            state.selected_workflow_id = available_workflows
                .first()
                .map(|workflow| workflow.id.clone());
        }
    }

    fn workspace_projects(&self) -> &[AppleWorkspaceProjectSummary] {
        match &self.workspace_projects_state {
            LoadState::Ready(projects) => projects,
            LoadState::Loading | LoadState::Error(_) => &[],
        }
    }

    fn workspace_project(&self, project_id: Option<&str>) -> Option<&AppleWorkspaceProjectSummary> {
        let project_id = project_id?;
        self.workspace_projects()
            .iter()
            .find(|project| project.id == project_id)
    }

    fn workspace_scheme<'a>(
        &'a self,
        project: &'a AppleWorkspaceProjectSummary,
        scheme_id: Option<&str>,
    ) -> Option<&'a AppleWorkspaceSchemeSummary> {
        let scheme_id = scheme_id?;
        project.schemes.iter().find(|scheme| scheme.id == scheme_id)
    }

    fn normalize_project_forms(
        &mut self,
        projects: &[AppleWorkspaceProjectSummary],
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(project) = projects.first() else {
            self.create_app_form.selected_project_id = None;
            self.create_app_form.selected_scheme_id = None;
            self.testflight_release_form.selected_project_id = None;
            self.testflight_release_form.selected_scheme_id = None;
            return;
        };

        let create_project = self
            .create_app_form
            .selected_project_id
            .as_deref()
            .and_then(|selected_id| projects.iter().find(|project| project.id == selected_id))
            .unwrap_or(project);
        let create_scheme = self
            .create_app_form
            .selected_scheme_id
            .as_deref()
            .and_then(|selected_id| {
                create_project
                    .schemes
                    .iter()
                    .find(|scheme| scheme.id == selected_id)
            })
            .or_else(|| create_project.schemes.first());
        if let Some(create_scheme) = create_scheme {
            self.create_app_form
                .set_defaults_for_scheme(create_project, create_scheme, window, cx);
        }

        let release_project = self
            .testflight_release_form
            .selected_project_id
            .as_deref()
            .and_then(|selected_id| projects.iter().find(|project| project.id == selected_id))
            .unwrap_or(project);
        let release_scheme = self
            .testflight_release_form
            .selected_scheme_id
            .as_deref()
            .and_then(|selected_id| {
                release_project
                    .schemes
                    .iter()
                    .find(|scheme| scheme.id == selected_id)
            })
            .or_else(|| release_project.schemes.first());
        if let Some(release_scheme) = release_scheme {
            self.testflight_release_form.set_defaults_for_scheme(
                release_project,
                release_scheme,
                window,
                cx,
            );
        }
    }

    fn select_create_project(
        &mut self,
        project_id: String,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(project) = self.workspace_project(Some(project_id.as_str())).cloned() else {
            return;
        };
        let Some(scheme) = project.schemes.first() else {
            return;
        };

        self.create_app_form
            .set_defaults_for_scheme(&project, scheme, window, cx);
        cx.notify();
    }

    fn select_create_scheme(
        &mut self,
        scheme_id: String,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(project) = self
            .workspace_project(self.create_app_form.selected_project_id.as_deref())
            .cloned()
        else {
            return;
        };
        let Some(scheme) = project.schemes.iter().find(|scheme| scheme.id == scheme_id) else {
            return;
        };

        self.create_app_form
            .set_defaults_for_scheme(&project, scheme, window, cx);
        cx.notify();
    }

    fn select_create_platform(&mut self, platform: String, cx: &mut Context<ServicesPage>) {
        self.create_app_form.selected_platform = platform;
        self.create_app_form.clear_status();
        cx.notify();
    }

    fn select_testflight_source_mode(
        &mut self,
        source_mode: AscTestFlightSourceMode,
        cx: &mut Context<ServicesPage>,
    ) {
        self.testflight_release_form.source_mode = source_mode;
        if let Some(form) = self.workflow_form_by_id_mut("publish_testflight") {
            form.clear_error();
        }
        self.latest_run = None;
        cx.notify();
    }

    fn select_testflight_project(
        &mut self,
        project_id: String,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(project) = self.workspace_project(Some(project_id.as_str())).cloned() else {
            return;
        };
        let Some(scheme) = project.schemes.first() else {
            return;
        };

        self.testflight_release_form
            .set_defaults_for_scheme(&project, scheme, window, cx);
        if let Some(form) = self.workflow_form_by_id_mut("publish_testflight") {
            form.clear_error();
        }
        self.latest_run = None;
        cx.notify();
    }

    fn select_testflight_scheme(
        &mut self,
        scheme_id: String,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(project) = self
            .workspace_project(self.testflight_release_form.selected_project_id.as_deref())
            .cloned()
        else {
            return;
        };
        let Some(scheme) = project.schemes.iter().find(|scheme| scheme.id == scheme_id) else {
            return;
        };

        self.testflight_release_form
            .set_defaults_for_scheme(&project, scheme, window, cx);
        if let Some(form) = self.workflow_form_by_id_mut("publish_testflight") {
            form.clear_error();
        }
        self.latest_run = None;
        cx.notify();
    }

    fn validate_testflight_release_selection(
        &self,
        selected_app: &AscAppSummary,
    ) -> Result<(), SharedString> {
        if self.testflight_release_form.source_mode != AscTestFlightSourceMode::LocalProject {
            return Ok(());
        }

        let project = self
            .workspace_project(self.testflight_release_form.selected_project_id.as_deref())
            .ok_or_else(|| SharedString::from("Workspace Project is required"))?;
        let scheme = self
            .workspace_scheme(
                project,
                self.testflight_release_form.selected_scheme_id.as_deref(),
            )
            .or_else(|| project.schemes.first())
            .ok_or_else(|| SharedString::from("Scheme is required"))?;

        validate_local_project_bundle_id_match(&selected_app.bundle_id, scheme)
    }

    fn submit_create_app(
        &mut self,
        _state: &mut ServicesPageState,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(project) = self
            .workspace_project(self.create_app_form.selected_project_id.as_deref())
            .cloned()
        else {
            self.create_app_form
                .set_error("Choose a workspace project before creating an app.");
            cx.notify();
            return;
        };
        let Some(_scheme) = self
            .workspace_scheme(&project, self.create_app_form.selected_scheme_id.as_deref())
            .cloned()
        else {
            self.create_app_form
                .set_error("Choose a scheme before creating an app.");
            cx.notify();
            return;
        };

        let app_name = trimmed_input_text(&self.create_app_form.app_name_input, cx);
        let bundle_id = trimmed_input_text(&self.create_app_form.bundle_id_input, cx);
        let sku = trimmed_input_text(&self.create_app_form.sku_input, cx);
        let primary_locale = trimmed_input_text(&self.create_app_form.primary_locale_input, cx);
        let version = trimmed_input_text(&self.create_app_form.initial_version_input, cx);
        let company_name = trimmed_input_text(&self.create_app_form.company_name_input, cx);
        let apple_id = trimmed_input_text(&self.create_app_form.apple_id_input, cx);
        let password = trimmed_input_text(&self.create_app_form.password_input, cx);
        let two_factor_code_command =
            trimmed_input_text(&self.create_app_form.two_factor_command_input, cx);
        let internal_group_name =
            trimmed_input_text(&self.create_app_form.internal_group_name_input, cx);

        if app_name.is_empty() || bundle_id.is_empty() || sku.is_empty() {
            self.create_app_form
                .set_error("App name, bundle ID, and SKU are required.");
            cx.notify();
            return;
        }
        if !password.is_empty() && apple_id.is_empty() {
            self.create_app_form
                .set_error("Apple Account Email is required when a password is provided.");
            cx.notify();
            return;
        }
        if self.create_app_form.create_internal_group.selected() && internal_group_name.is_empty() {
            self.create_app_form
                .set_error("Internal TestFlight Group is required when group creation is enabled.");
            cx.notify();
            return;
        }

        self.create_app_form.set_pending(true);
        cx.notify();

        let selected_platform = self.create_app_form.selected_platform.clone();
        let should_create_group = self.create_app_form.create_internal_group.selected();
        let interactive_two_factor_path = if !apple_id.is_empty()
            && !password.is_empty()
            && two_factor_code_command.is_empty()
            && !matches!(
                self.web_auth_state,
                LoadState::Ready(AscWebAuthSummary {
                    authenticated: true
                })
            ) {
            Some(new_two_factor_code_file_path())
        } else {
            None
        };

        if let Some(code_file_path) = interactive_two_factor_path.clone() {
            let _ = persist_two_factor_code(&code_file_path, "");
            workspace
                .update(cx, |workspace, cx| {
                    let code_file_path = code_file_path.clone();
                    workspace.toggle_modal(window, cx, move |window, cx| {
                        AscTwoFactorModal::new(code_file_path.clone(), window, cx)
                    });
                })
                .ok();
        }

        cx.spawn_in(window, async move |this, cx| {
            let generated_two_factor_code_command = interactive_two_factor_path
                .as_ref()
                .map(|path| two_factor_wait_command(path));
            let result = cx
                .background_spawn(async move {
                    if !apple_id.is_empty()
                        || !password.is_empty()
                        || generated_two_factor_code_command.is_some()
                        || !two_factor_code_command.is_empty()
                    {
                        let _: serde_json::Value = run_json_operation(ServiceOperationRequest {
                            provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
                            operation: "web_auth_login".to_string(),
                            resource: None,
                            artifact: None,
                            input: [
                                ("apple_id".to_string(), apple_id.clone()),
                                ("password".to_string(), password.clone()),
                                (
                                    "two_factor_code_command".to_string(),
                                    generated_two_factor_code_command
                                        .clone()
                                        .unwrap_or_else(|| two_factor_code_command.clone()),
                                ),
                            ]
                            .into_iter()
                            .filter(|(_, value)| !value.is_empty())
                            .collect(),
                        })
                        .await?;
                    }

                    let _: serde_json::Value = run_json_operation(ServiceOperationRequest {
                        provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
                        operation: "create_app".to_string(),
                        resource: None,
                        artifact: None,
                        input: [
                            ("name".to_string(), app_name.clone()),
                            ("bundle_id".to_string(), bundle_id.clone()),
                            ("sku".to_string(), sku.clone()),
                            ("platform".to_string(), selected_platform.clone()),
                            ("primary_locale".to_string(), primary_locale.clone()),
                            ("version".to_string(), version.clone()),
                            ("company_name".to_string(), company_name.clone()),
                            ("apple_id".to_string(), apple_id.clone()),
                            ("password".to_string(), password.clone()),
                            (
                                "two_factor_code_command".to_string(),
                                generated_two_factor_code_command
                                    .clone()
                                    .unwrap_or_else(|| two_factor_code_command.clone()),
                            ),
                        ]
                        .into_iter()
                        .filter(|(_, value)| !value.is_empty())
                        .collect(),
                    })
                    .await?;

                    let app =
                        wait_for_created_app(bundle_id.clone(), app_name.clone(), sku.clone())
                            .await?
                            .ok_or_else(|| {
                                anyhow::anyhow!("Created app was not found after creation.")
                            })?;

                    let group_warning = if should_create_group {
                        create_internal_group_with_retry(&app, internal_group_name.clone())
                            .await
                            .err()
                    } else {
                        None
                    };

                    Ok::<_, anyhow::Error>(CreateAppResult {
                        app,
                        apps: load_apps().await?,
                        project,
                        group_warning,
                    })
                })
                .await;

            this.update_in(cx, |page, window, cx| {
                if interactive_two_factor_path.is_some() {
                    workspace
                        .update(cx, |workspace, cx| {
                            workspace.hide_modal(window, cx);
                        })
                        .ok();
                }
                with_app_store_connect_provider_mut(page, |pane, state| match result {
                    Ok(result) => {
                        let success_message = match result.group_warning.as_ref() {
                            Some(warning) => format!(
                                "Created {} for {}. Internal group creation failed: {}",
                                result.app.name, result.project.label, warning
                            ),
                            None => {
                                format!("Created {} for {}.", result.app.name, result.project.label)
                            }
                        };
                        pane.create_app_form.finish_success(success_message);
                        pane.apps_state = LoadState::Ready(result.apps);
                        pane.web_auth_state = LoadState::Ready(AscWebAuthSummary {
                            authenticated: true,
                        });
                        state.selected_resource_id = Some(result.app.id.clone());
                        pane.load_builds_for_app(state, result.app.id, window, cx);
                    }
                    Err(error) => {
                        pane.create_app_form.set_error(error.to_string());
                        cx.notify();
                    }
                });
            })
            .ok();

            if let Some(code_file_path) = interactive_two_factor_path.as_ref() {
                let _ = std::fs::remove_file(code_file_path);
            }
        })
        .detach();
    }

    fn load_builds_for_app(
        &mut self,
        state: &mut ServicesPageState,
        app_id: String,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(app) = self.apps().iter().find(|app| app.id == app_id).cloned() else {
            self.builds_state = LoadState::Ready(AscBuildListState::default());
            cx.notify();
            return;
        };

        state.selected_resource_id = Some(app.id.clone());
        self.builds_state = LoadState::Loading;
        self.latest_run = None;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let builds_result = cx
                .background_spawn(async move { load_builds_page(&app, None).await })
                .await;
            this.update_in(cx, |page, _window, cx| {
                with_app_store_connect_provider_mut(page, |pane, state| {
                    if state.selected_resource_id.as_deref() != Some(app_id.as_str()) {
                        return;
                    }

                    pane.builds_state = match builds_result {
                        Ok(page) => LoadState::Ready(AscBuildListState {
                            builds: page.builds,
                            next_page_url: page.next_page_url,
                            is_loading_more: false,
                            load_more_error: None,
                        }),
                        Err(error) => LoadState::Error(error.to_string()),
                    };
                    cx.notify();
                });
            })
            .ok();
        })
        .detach();
    }

    fn refresh_builds(
        &mut self,
        state: &mut ServicesPageState,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        if let Some(app_id) = state.selected_resource_id.clone() {
            self.load_builds_for_app(state, app_id, window, cx);
        }
    }

    fn load_more_builds(
        &mut self,
        state: &ServicesPageState,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(app_id) = state.selected_resource_id.clone() else {
            return;
        };
        let Some(app) = self.apps().iter().find(|app| app.id == app_id).cloned() else {
            return;
        };

        let next_page_url = match &mut self.builds_state {
            LoadState::Ready(builds_state) => {
                if builds_state.is_loading_more {
                    return;
                }

                let Some(next_page_url) = builds_state.next_page_url.clone() else {
                    return;
                };
                builds_state.is_loading_more = true;
                builds_state.load_more_error = None;
                next_page_url
            }
            LoadState::Loading | LoadState::Error(_) => return,
        };

        cx.notify();

        let request_next_page_url = next_page_url.clone();
        cx.spawn_in(window, async move |this, cx| {
            let builds_result = cx
                .background_spawn(async move {
                    load_builds_page(&app, Some(request_next_page_url)).await
                })
                .await;
            this.update_in(cx, |page, _window, cx| {
                with_app_store_connect_provider_mut(page, |pane, state| {
                    if state.selected_resource_id.as_deref() != Some(app_id.as_str()) {
                        return;
                    }

                    let LoadState::Ready(builds_state) = &mut pane.builds_state else {
                        return;
                    };
                    if builds_state.next_page_url.as_deref() != Some(next_page_url.as_str()) {
                        return;
                    }

                    builds_state.is_loading_more = false;
                    match builds_result {
                        Ok(page) => {
                            builds_state.builds.extend(page.builds);
                            builds_state.next_page_url = page.next_page_url;
                            builds_state.load_more_error = None;
                        }
                        Err(error) => {
                            builds_state.load_more_error = Some(error.to_string());
                        }
                    }
                    cx.notify();
                });
            })
            .ok();
        })
        .detach();
    }

    #[cfg(target_os = "macos")]
    fn show_authenticate_form(&mut self) {
        self.auth_form.show();
    }

    #[cfg(target_os = "macos")]
    fn cancel_authenticate_form(&mut self) {
        self.auth_form.cancel();
    }

    #[cfg(target_os = "macos")]
    fn submit_authenticate(
        &mut self,
        _workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let request = match self
            .auth_form
            .build_authenticate_request(APP_STORE_CONNECT_PROVIDER_ID, cx)
        {
            Ok(request) => request,
            Err(error) => {
                self.auth_form.set_error(error);
                cx.notify();
                return;
            }
        };

        self.auth_form.set_pending(true);
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(async move { run_auth_action(request).await })
                .await;
            this.update_in(cx, |page, window, cx| {
                let workspace_paths = page.workspace_paths().to_vec();
                with_app_store_connect_provider_mut(page, |pane, state| match result {
                    Ok(()) => {
                        pane.auth_form.finish_success();
                        pane.refresh(state, workspace_paths, window, cx);
                    }
                    Err(error) => {
                        pane.auth_form.set_pending(false);
                        pane.auth_form.set_error(error.to_string());
                        cx.notify();
                    }
                });
            })
            .ok();
        })
        .detach();
    }

    #[cfg(target_os = "macos")]
    fn logout(
        &mut self,
        _workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(request) = self
            .auth_form
            .build_logout_request(APP_STORE_CONNECT_PROVIDER_ID)
        else {
            return;
        };

        self.auth_form.set_pending(true);
        self.auth_form.error_message = None;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(async move { run_auth_action(request).await })
                .await;
            this.update_in(cx, |page, window, cx| {
                let workspace_paths = page.workspace_paths().to_vec();
                with_app_store_connect_provider_mut(page, |pane, state| match result {
                    Ok(()) => {
                        pane.auth_form.finish_success();
                        pane.refresh(state, workspace_paths, window, cx);
                    }
                    Err(error) => {
                        pane.auth_form.set_pending(false);
                        pane.auth_form.set_error(error.to_string());
                        cx.notify();
                    }
                });
            })
            .ok();
        })
        .detach();
    }

    #[cfg(target_os = "macos")]
    fn pick_auth_file(
        &mut self,
        field_key: String,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let prompt = workspace
            .update(cx, |workspace, cx| {
                workspace.prompt_for_open_path(
                    gpui::PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: false,
                        prompt: Some(SharedString::from("Select an App Store Connect key file")),
                    },
                    DirectoryLister::Local(
                        workspace.project().clone(),
                        workspace.app_state().fs.clone(),
                    ),
                    window,
                    cx,
                )
            })
            .ok();

        let Some(prompt) = prompt else {
            return;
        };

        cx.spawn_in(window, async move |this, cx| {
            let path = match prompt.await {
                Ok(Some(mut paths)) => paths.pop(),
                Ok(None) => None,
                Err(error) => {
                    this.update(cx, |page, cx| {
                        with_app_store_connect_provider_mut(page, |pane, _state| {
                            pane.auth_form.set_error(error.to_string());
                            cx.notify();
                        });
                    })
                    .ok();
                    None
                }
            };

            let Some(path) = path else {
                return;
            };

            this.update_in(cx, |page, window, cx| {
                with_app_store_connect_provider_mut(page, |pane, _state| {
                    pane.auth_form
                        .set_text(&field_key, &path.to_string_lossy(), window, cx);
                    cx.notify();
                });
            })
            .ok();
        })
        .detach();
    }

    fn pick_workflow_file(
        &mut self,
        state: &ServicesPageState,
        field_key: String,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(workflow_id) = state.selected_workflow_id.clone() else {
            return;
        };
        let Some(form) = self.workflow_form_by_id_mut(&workflow_id) else {
            return;
        };
        form.clear_error();

        let prompt = workspace
            .update(cx, |workspace, cx| {
                workspace.prompt_for_open_path(
                    gpui::PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: false,
                        prompt: Some(SharedString::from("Select an App Store Connect artifact")),
                    },
                    DirectoryLister::Local(
                        workspace.project().clone(),
                        workspace.app_state().fs.clone(),
                    ),
                    window,
                    cx,
                )
            })
            .ok();

        let Some(prompt) = prompt else {
            return;
        };

        cx.spawn_in(window, async move |this, cx| {
            let path = match prompt.await {
                Ok(Some(mut paths)) => paths.pop(),
                Ok(None) => None,
                Err(error) => {
                    this.update(cx, |page, cx| {
                        with_app_store_connect_provider_mut(page, |pane, _state| {
                            if let Some(form) = pane.workflow_form_by_id_mut(&workflow_id) {
                                form.set_error(error.to_string());
                            }
                            cx.notify();
                        });
                    })
                    .ok();
                    None
                }
            };

            let Some(path) = path else {
                return;
            };

            this.update_in(cx, |page, window, cx| {
                with_app_store_connect_provider_mut(page, |pane, _state| {
                    if let Some(form) = pane.workflow_form_by_id(&workflow_id) {
                        form.set_text(&field_key, &path.to_string_lossy(), window, cx);
                    }
                    cx.notify();
                });
            })
            .ok();
        })
        .detach();
    }

    fn pick_testflight_ipa_file(
        &mut self,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        if let Some(form) = self.workflow_form_by_id_mut("publish_testflight") {
            form.clear_error();
        }

        let prompt = workspace
            .update(cx, |workspace, cx| {
                workspace.prompt_for_open_path(
                    gpui::PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: false,
                        prompt: Some(SharedString::from("Select an App Store Connect artifact")),
                    },
                    DirectoryLister::Local(
                        workspace.project().clone(),
                        workspace.app_state().fs.clone(),
                    ),
                    window,
                    cx,
                )
            })
            .ok();

        let Some(prompt) = prompt else {
            return;
        };

        cx.spawn_in(window, async move |this, cx| {
            let path = match prompt.await {
                Ok(Some(mut paths)) => paths.pop(),
                Ok(None) => None,
                Err(error) => {
                    this.update(cx, |page, cx| {
                        with_app_store_connect_provider_mut(page, |pane, _state| {
                            if let Some(form) = pane.workflow_form_by_id_mut("publish_testflight") {
                                form.set_error(error.to_string());
                            }
                            cx.notify();
                        });
                    })
                    .ok();
                    None
                }
            };

            let Some(path) = path else {
                return;
            };

            this.update_in(cx, |page, window, cx| {
                with_app_store_connect_provider_mut(page, |pane, _state| {
                    set_input_text(
                        &pane.testflight_release_form.ipa_path_input,
                        &path.to_string_lossy(),
                        window,
                        cx,
                    );
                    cx.notify();
                });
            })
            .ok();
        })
        .detach();
    }

    fn pick_testflight_export_options_file(
        &mut self,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        if let Some(form) = self.workflow_form_by_id_mut("publish_testflight") {
            form.clear_error();
        }

        let prompt = workspace
            .update(cx, |workspace, cx| {
                workspace.prompt_for_open_path(
                    gpui::PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: false,
                        prompt: Some(SharedString::from("Select ExportOptions.plist")),
                    },
                    DirectoryLister::Local(
                        workspace.project().clone(),
                        workspace.app_state().fs.clone(),
                    ),
                    window,
                    cx,
                )
            })
            .ok();

        let Some(prompt) = prompt else {
            return;
        };

        cx.spawn_in(window, async move |this, cx| {
            let path = match prompt.await {
                Ok(Some(mut paths)) => paths.pop(),
                Ok(None) => None,
                Err(error) => {
                    this.update(cx, |page, cx| {
                        with_app_store_connect_provider_mut(page, |pane, _state| {
                            if let Some(form) = pane.workflow_form_by_id_mut("publish_testflight") {
                                form.set_error(error.to_string());
                            }
                            cx.notify();
                        });
                    })
                    .ok();
                    None
                }
            };

            let Some(path) = path else {
                return;
            };

            this.update_in(cx, |page, window, cx| {
                with_app_store_connect_provider_mut(page, |pane, _state| {
                    set_input_text(
                        &pane.testflight_release_form.export_options_input,
                        &path.to_string_lossy(),
                        window,
                        cx,
                    );
                    cx.notify();
                });
            })
            .ok();
        })
        .detach();
    }

    fn select_target(&mut self, state: &mut ServicesPageState, target_id: String) {
        if state.selected_target_id.as_ref() == Some(&target_id) {
            return;
        }

        state.selected_target_id = Some(target_id);
        self.normalize_workflow_state(state);
        self.latest_run = None;
    }

    fn select_workflow(&mut self, state: &mut ServicesPageState, workflow_id: String) {
        if state.selected_workflow_id.as_ref() == Some(&workflow_id) {
            return;
        }

        state.selected_workflow_id = Some(workflow_id);
        self.latest_run = None;
    }

    fn workflow_ui_model(&self, state: &ServicesPageState) -> Option<ServiceWorkflowUiModel> {
        if !self.cli_ready() {
            return None;
        }

        let workflows = self.available_workflows(state);
        if workflows.is_empty() {
            return None;
        }

        let form = self.selected_workflow_form(state)?.clone();
        let descriptor = self.selected_workflow_descriptor(state)?;
        let selected_app = self.selected_app(state);
        let disabled_reason = if selected_app.is_none() && descriptor.resource_kind.is_some() {
            Some("Select an app first.".into())
        } else {
            None
        };

        Some(ServiceWorkflowUiModel {
            provider_id: self.descriptor.id.clone(),
            target_label: "Target".into(),
            selected_target_id: state.selected_target_id.clone(),
            targets: self.available_targets(&self.navigation_workflows(state)),
            workflow_label: "Workflow".into(),
            selected_workflow_id: state.selected_workflow_id.clone(),
            workflows,
            execute_label: descriptor.label.clone().into(),
            form,
            run: self
                .latest_run
                .as_ref()
                .filter(|run| {
                    Some(run.workflow.as_str()) == state.selected_workflow_id.as_deref()
                        && run.target_id == state.selected_target_id
                })
                .map(|run| ServiceWorkflowRunSummary {
                    state: run.state.clone(),
                    headline: run.headline.clone(),
                    detail: run.detail.clone(),
                }),
            disabled_reason,
        })
    }

    fn submit_workflow(
        &mut self,
        state: &mut ServicesPageState,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        let Some(selected_app) = self.selected_app(state) else {
            return;
        };
        let Some(descriptor) = self.selected_workflow_descriptor(state).cloned() else {
            return;
        };
        let workflow_id = descriptor.id.clone();
        let target_id = state.selected_target_id.clone();
        let input = if workflow_id == "publish_testflight" {
            if let Err(error) = self.validate_testflight_release_selection(&selected_app) {
                if let Some(form) = self.workflow_form_by_id_mut(&workflow_id) {
                    form.set_error(error);
                }
                cx.notify();
                return;
            }

            match self
                .testflight_release_form
                .build_workflow_input(self.workspace_projects(), cx)
            {
                Ok(input) => input,
                Err(error) => {
                    if let Some(form) = self.workflow_form_by_id_mut(&workflow_id) {
                        form.set_error(error);
                    }
                    cx.notify();
                    return;
                }
            }
        } else {
            let Some(form) = self.selected_workflow_form_mut(state) else {
                return;
            };

            match form.build_input(cx) {
                Ok(input) => input,
                Err(error) => {
                    form.set_error(error);
                    cx.notify();
                    return;
                }
            }
        };
        if let Some(form) = self.workflow_form_by_id_mut(&workflow_id) {
            form.set_pending(true);
            form.clear_error();
        }

        let request = ServiceWorkflowRequest {
            provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
            workflow: workflow_id.clone(),
            target_id: target_id.clone(),
            resource: Some(ServiceResourceRef {
                provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
                kind: "app".to_string(),
                external_id: selected_app.id.clone(),
                label: selected_app.name.clone(),
            }),
            artifact: None,
            input,
        };
        self.latest_run = Some(ServiceRunDescriptor {
            workflow: workflow_id.clone(),
            target_id: target_id.clone(),
            state: ServiceRunState::Running,
            headline: descriptor.label.clone(),
            detail: if workflow_id == "publish_testflight" {
                format!(
                    "Building, exporting, uploading, and preparing TestFlight distribution for {}.",
                    selected_app.name
                )
            } else {
                format!("Running {} for {}.", descriptor.label, selected_app.name)
            },
            output: None,
        });
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let workflow_label = descriptor.label.clone();
            let run_result = {
                let result = cx
                    .background_spawn(async move { run_workflow(request).await })
                    .await;
                match result {
                    Ok(execution) => WorkflowExecutionResult {
                        output: execution.combined_output(),
                        error: None,
                    },
                    Err(error) => WorkflowExecutionResult {
                        output: String::new(),
                        error: Some(error.to_string()),
                    },
                }
            };

            this.update_in(cx, |page, _window, cx| {
                with_app_store_connect_provider_mut(page, |pane, _state| {
                    match run_result.error {
                        Some(error) => {
                            let error_detail = summarize_workflow_error(&error);
                            let is_distribution_pending = workflow_id == "publish_testflight"
                                && is_testflight_distribution_pending_error(&error);

                            if let Some(form) = pane.workflow_form_by_id_mut(&workflow_id) {
                                if is_distribution_pending {
                                    form.finish_success();
                                } else {
                                    form.set_error(error_detail.clone());
                                }
                            }

                            pane.latest_run = Some(ServiceRunDescriptor {
                                workflow: workflow_id.clone(),
                                target_id: target_id.clone(),
                                state: if is_distribution_pending {
                                    ServiceRunState::Warning
                                } else {
                                    ServiceRunState::Failed
                                },
                                headline: if is_distribution_pending {
                                    format!("{workflow_label} uploaded")
                                } else {
                                    format!("{workflow_label} failed")
                                },
                                detail: if is_distribution_pending {
                                    "Build uploaded to App Store Connect, but Apple has not finished making it assignable to TestFlight groups yet. Re-run when processing finishes or enable Wait for processing.".to_string()
                                } else {
                                    error_detail
                                },
                                output: None,
                            });
                        }
                        None => {
                            let detail = summarize_workflow_output(&run_result.output);
                            if let Some(form) = pane.workflow_form_by_id_mut(&workflow_id) {
                                form.finish_success();
                            }
                            pane.latest_run = Some(ServiceRunDescriptor {
                                workflow: workflow_id.clone(),
                                target_id: target_id.clone(),
                                state: ServiceRunState::Succeeded,
                                headline: format!("{workflow_label} finished"),
                                detail,
                                output: None,
                            });
                        }
                    }

                    cx.notify();
                });
            })
            .ok();
        })
        .detach();
    }

    fn apps(&self) -> &[AscAppSummary] {
        match &self.apps_state {
            LoadState::Ready(apps) => apps,
            LoadState::Loading | LoadState::Error(_) => &[],
        }
    }

    fn selected_app(&self, state: &ServicesPageState) -> Option<AscAppSummary> {
        state
            .selected_resource_id
            .as_ref()
            .and_then(|selected_id| self.apps().iter().find(|app| &app.id == selected_id))
            .cloned()
    }

    fn selected_build(&self) -> Option<&AscBuildSummary> {
        match &self.builds_state {
            LoadState::Ready(builds_state) => builds_state.builds.first(),
            LoadState::Loading | LoadState::Error(_) => None,
        }
    }

    #[cfg(target_os = "macos")]
    fn auth_status_summary(&self) -> ServiceAuthStatusSummary {
        match &self.auth_state {
            LoadState::Loading => ServiceAuthStatusSummary {
                severity: Severity::Success,
                headline: "Checking authentication…".to_string(),
                detail: "Validating the current App Store Connect profile.".to_string(),
                warnings: Vec::new(),
                authenticated: false,
            },
            LoadState::Error(error) => ServiceAuthStatusSummary {
                severity: Severity::Warning,
                headline: "Authentication check failed".to_string(),
                detail: error.clone(),
                warnings: Vec::new(),
                authenticated: false,
            },
            LoadState::Ready(summary) => ServiceAuthStatusSummary {
                severity: if summary.healthy {
                    Severity::Success
                } else {
                    Severity::Warning
                },
                headline: summary.headline.clone(),
                detail: summary.detail.clone(),
                warnings: summary.warnings.clone(),
                authenticated: summary.authenticated,
            },
        }
    }

    #[cfg(target_os = "macos")]
    fn render_cli_sidebar_status(&self, cx: &App) -> Option<AnyElement> {
        let content = match &self.cli_state {
            AscCliState::Ready(summary) => h_flex()
                .items_center()
                .gap_2()
                .child(Indicator::dot().color(Color::Success))
                .child(
                    Label::new(format!("ASC CLI ready: {}", summary.version))
                        .size(LabelSize::Small)
                        .color(Color::Muted)
                        .truncate(),
                )
                .into_any_element(),
            AscCliState::Checking => Label::new("Checking ASC CLI…")
                .size(LabelSize::Small)
                .color(Color::Muted)
                .into_any_element(),
            AscCliState::Installing => Label::new("Installing ASC CLI…")
                .size(LabelSize::Small)
                .color(Color::Muted)
                .into_any_element(),
            AscCliState::Missing(_) | AscCliState::InstallFailed(_) => return None,
        };

        Some(
            v_flex()
                .gap_2()
                .pt_3()
                .border_t_1()
                .border_color(cx.theme().colors().border_variant)
                .child(content)
                .into_any_element(),
        )
    }

    fn render_detail_row(
        &self,
        title: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> impl IntoElement {
        h_flex()
            .justify_between()
            .gap_3()
            .child(Label::new(title).size(LabelSize::Small).color(Color::Muted))
            .child(
                Label::new(value)
                    .size(LabelSize::Small)
                    .single_line()
                    .truncate(),
            )
    }

    fn render_empty_panel(
        &self,
        title: impl Into<SharedString>,
        detail: impl Into<SharedString>,
        cx: &App,
    ) -> impl IntoElement {
        let radius = Self::panel_radius(cx);
        v_flex()
            .w_full()
            .gap_2()
            .p_5()
            .rounded(radius)
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().background)
            .child(Label::new(title))
            .child(
                Label::new(detail)
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
    }

    fn render_panel_header(
        &self,
        title: impl Into<SharedString>,
        detail: impl Into<SharedString>,
    ) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(Label::new(title).size(LabelSize::Large))
            .child(
                Label::new(detail)
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
    }

    fn render_cli_requirement_card(
        &self,
        _window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> impl IntoElement {
        let radius = Self::panel_radius(cx);
        let page = cx.entity().downgrade();
        let (status_label, status_color, detail) = match &self.cli_state {
            AscCliState::Checking => (
                "Checking ASC CLI…",
                Color::Muted,
                Some(
                    "Qedit is validating the local ASC CLI installation before loading App Store Connect data."
                        .to_string(),
                ),
            ),
            AscCliState::Installing => (
                "Installing ASC CLI…",
                Color::Accent,
                Some(
                    "Qedit is running `brew install asc`. Leave this view open until installation finishes."
                        .to_string(),
                ),
            ),
            AscCliState::Missing(_detail) => (
                "ASC CLI is required",
                Color::Warning,
                None,
            ),
            AscCliState::InstallFailed(detail) => (
                "ASC CLI installation failed",
                Color::Error,
                Some(detail.clone()),
            ),
            AscCliState::Ready(_) => (
                "ASC CLI ready",
                Color::Success,
                None,
            ),
        };

        v_flex().size_full().justify_center().items_center().child(
            v_flex()
                .w_full()
                .max_w(rems(42.))
                .gap_4()
                .p_5()
                .rounded(radius)
                .border_1()
                .border_color(cx.theme().colors().border_variant)
                .bg(cx.theme().colors().background)
                .child(self.render_panel_header(
                    "App Store Connect",
                    "This provider depends on the local ASC CLI.",
                ))
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(Indicator::dot().color(status_color))
                        .child(
                            Label::new(status_label)
                                .size(LabelSize::Small)
                                .color(status_color),
                        ),
                )
                .when_some(detail, |this, detail| {
                    this.child(
                        Label::new(detail)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                })
                .child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child(
                            Button::new("asc-cli-install", "Install")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .disabled(matches!(
                                    self.cli_state,
                                    AscCliState::Checking | AscCliState::Installing
                                ))
                                .on_click({
                                    let page = page.clone();
                                    move |_, window, cx| {
                                        page.update(cx, |page, cx| {
                                            with_app_store_connect_provider_mut(
                                                page,
                                                |pane, _state| {
                                                    pane.install_cli(window, cx);
                                                },
                                            );
                                        })
                                        .ok();
                                    }
                                }),
                        )
                        .child(
                            IconButton::new("asc-cli-github", IconName::Github)
                                .shape(ui::IconButtonShape::Square)
                                .style(ButtonStyle::Transparent)
                                .size(ButtonSize::Compact)
                                .icon_size(ui::IconSize::Small)
                                .tooltip(ui::Tooltip::text("View ASC CLI on GitHub"))
                                .on_click(|_, _, cx| {
                                    cx.open_url(ASC_CLI_GITHUB_URL);
                                }),
                        ),
                ),
        )
    }

    fn render_popover_button(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        menu: Entity<ContextMenu>,
    ) -> impl IntoElement {
        ServicesPage::render_sidebar_popover_menu(id, label, menu)
    }

    fn render_testflight_workflow_form(
        &self,
        page: WeakEntity<ServicesPage>,
        workflow_ui: &ServiceWorkflowUiModel,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> AnyElement {
        let selected_project =
            self.workspace_project(self.testflight_release_form.selected_project_id.as_deref());
        let selected_scheme = selected_project.and_then(|project| {
            self.workspace_scheme(
                project,
                self.testflight_release_form.selected_scheme_id.as_deref(),
            )
        });

        let source_menu = ContextMenu::build(window, cx, |mut menu, _, _| {
            for source_mode in [
                AscTestFlightSourceMode::LocalProject,
                AscTestFlightSourceMode::IpaFile,
                AscTestFlightSourceMode::ExistingBuild,
            ] {
                let page = page.clone();
                menu = menu.entry(source_mode.label().to_string(), None, move |_window, cx| {
                    page.update(cx, |page, cx| {
                        with_app_store_connect_provider_mut(page, |pane, _state| {
                            pane.select_testflight_source_mode(source_mode, cx);
                        });
                    })
                    .ok();
                });
            }
            menu
        });

        let project_menu = ContextMenu::build(window, cx, |mut menu, _, _| {
            for project in self.workspace_projects() {
                let label = project.label.clone();
                let detail = project.display_path();
                let page = page.clone();
                let project_id = project.id.clone();
                menu = menu.entry(format!("{label} ({detail})"), None, move |window, cx| {
                    page.update(cx, |page, cx| {
                        with_app_store_connect_provider_mut(page, |pane, _state| {
                            pane.select_testflight_project(project_id.clone(), window, cx);
                        });
                    })
                    .ok();
                });
            }
            menu
        });

        let scheme_menu = ContextMenu::build(window, cx, |mut menu, _, _| {
            if let Some(project) = selected_project {
                for scheme in &project.schemes {
                    let page = page.clone();
                    let scheme_id = scheme.id.clone();
                    menu = menu.entry(scheme.label.clone(), None, move |window, cx| {
                        page.update(cx, |page, cx| {
                            with_app_store_connect_provider_mut(page, |pane, _state| {
                                pane.select_testflight_scheme(scheme_id.clone(), window, cx);
                            });
                        })
                        .ok();
                    });
                }
            }
            menu
        });

        let source_mode = self.testflight_release_form.source_mode;
        let scheme_summary = selected_scheme.map(|scheme| {
            [
                scheme
                    .bundle_id
                    .clone()
                    .unwrap_or_else(|| "Unknown bundle ID".to_string()),
                format_platform(scheme.platform.as_deref()),
                format!(
                    "Version {}",
                    scheme
                        .marketing_version
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string())
                ),
                format!(
                    "Build {}",
                    scheme
                        .build_number
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string())
                ),
            ]
            .join(" · ")
        });

        v_flex()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new("Source")
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(Self::render_popover_button(
                        "asc-testflight-source",
                        source_mode.label(),
                        source_menu,
                    )),
            )
            .when(
                matches!(source_mode, AscTestFlightSourceMode::LocalProject),
                |this| {
                    this.child(
                        h_flex()
                            .gap_3()
                            .flex_wrap()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .min_w(rems(18.))
                                    .child(
                                        Label::new("Workspace Project")
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
                                    )
                                    .child(Self::render_popover_button(
                                        "asc-testflight-project",
                                        selected_project
                                            .map(|project| project.label.clone())
                                            .unwrap_or_else(|| "Select project".to_string()),
                                        project_menu,
                                    )),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .min_w(rems(16.))
                                    .child(
                                        Label::new("Scheme")
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
                                    )
                                    .child(Self::render_popover_button(
                                        "asc-testflight-scheme",
                                        selected_scheme
                                            .map(|scheme| scheme.label.clone())
                                            .unwrap_or_else(|| "Select scheme".to_string()),
                                        scheme_menu,
                                    )),
                            ),
                    )
                    .when_some(scheme_summary, |this, scheme_summary| {
                        this.child(
                            Label::new(scheme_summary)
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                    })
                    .child(
                        h_flex()
                            .gap_3()
                            .flex_wrap()
                            .child(
                                div()
                                    .min_w(rems(12.))
                                    .flex_1()
                                    .child(self.testflight_release_form.version_input.clone()),
                            )
                            .child(
                                div().min_w(rems(12.)).flex_1().child(
                                    self.testflight_release_form.configuration_input.clone(),
                                ),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_end()
                            .gap_2()
                            .child(self.testflight_release_form.export_options_input.clone())
                            .child(ServicesPage::render_action_chip(
                                "asc-testflight-export-options-browse",
                                "Browse",
                                IconName::FolderOpen,
                                workflow_ui.form.pending,
                                {
                                    let page = page.clone();
                                    move |_, window, cx| {
                                        page.update(cx, |page, cx| {
                                            let workspace = page.workspace().clone();
                                            with_app_store_connect_provider_mut(
                                                page,
                                                |pane, _state| {
                                                    pane.pick_testflight_export_options_file(
                                                        workspace, window, cx,
                                                    );
                                                },
                                            );
                                        })
                                        .ok();
                                    }
                                },
                                cx,
                            )),
                    )
                    .child(
                        Label::new(
                            "Required for local project publishing unless the workspace already has .asc/export-options-app-store.plist.",
                        )
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                    )
                    .child(self.testflight_release_form.group_input.clone())
                },
            )
            .when(
                matches!(source_mode, AscTestFlightSourceMode::IpaFile),
                |this| {
                    this.child(
                        h_flex()
                            .items_end()
                            .gap_2()
                            .child(self.testflight_release_form.ipa_path_input.clone())
                            .child(ServicesPage::render_action_chip(
                                "asc-testflight-ipa-browse",
                                "Browse",
                                IconName::FolderOpen,
                                workflow_ui.form.pending,
                                {
                                    let page = page.clone();
                                    move |_, window, cx| {
                                        page.update(cx, |page, cx| {
                                            let workspace = page.workspace().clone();
                                            with_app_store_connect_provider_mut(
                                                page,
                                                |pane, _state| {
                                                    pane.pick_testflight_ipa_file(
                                                        workspace, window, cx,
                                                    );
                                                },
                                            );
                                        })
                                        .ok();
                                    }
                                },
                                cx,
                            )),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .flex_wrap()
                            .child(
                                div()
                                    .min_w(rems(12.))
                                    .flex_1()
                                    .child(self.testflight_release_form.version_input.clone()),
                            )
                            .child(
                                div()
                                    .min_w(rems(12.))
                                    .flex_1()
                                    .child(self.testflight_release_form.group_input.clone()),
                            ),
                    )
                },
            )
            .when(
                matches!(source_mode, AscTestFlightSourceMode::ExistingBuild),
                |this| {
                    this.child(
                        h_flex()
                            .gap_3()
                            .flex_wrap()
                            .child(
                                div()
                                    .min_w(rems(12.))
                                    .flex_1()
                                    .child(self.testflight_release_form.build_number_input.clone()),
                            )
                            .child(
                                div()
                                    .min_w(rems(12.))
                                    .flex_1()
                                    .child(self.testflight_release_form.build_id_input.clone()),
                            ),
                    )
                    .child(self.testflight_release_form.group_input.clone())
                },
            )
            .child(
                h_flex()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        Checkbox::new(
                            "asc-testflight-wait",
                            self.testflight_release_form.wait_for_processing,
                        )
                        .label("Wait for processing")
                        .disabled(workflow_ui.form.pending)
                        .on_click(cx.listener(
                            |page, checked, _window, cx| {
                                with_app_store_connect_provider_mut(page, |pane, _state| {
                                    pane.testflight_release_form.wait_for_processing = *checked;
                                    if let Some(form) =
                                        pane.workflow_form_by_id_mut("publish_testflight")
                                    {
                                        form.clear_error();
                                    }
                                    pane.latest_run = None;
                                });
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        Checkbox::new(
                            "asc-testflight-notify",
                            self.testflight_release_form.notify_testers,
                        )
                        .label("Notify testers")
                        .disabled(workflow_ui.form.pending)
                        .on_click(cx.listener(
                            |page, checked, _window, cx| {
                                with_app_store_connect_provider_mut(page, |pane, _state| {
                                    pane.testflight_release_form.notify_testers = *checked;
                                    if let Some(form) =
                                        pane.workflow_form_by_id_mut("publish_testflight")
                                    {
                                        form.clear_error();
                                    }
                                    pane.latest_run = None;
                                });
                                cx.notify();
                            },
                        )),
                    )
                    .when(
                        matches!(source_mode, AscTestFlightSourceMode::LocalProject),
                        |this| {
                            this.child(
                                Checkbox::new(
                                    "asc-testflight-clean",
                                    self.testflight_release_form.clean_build,
                                )
                                .label("Clean build")
                                .disabled(workflow_ui.form.pending)
                                .on_click(cx.listener(
                                    |page, checked, _window, cx| {
                                        with_app_store_connect_provider_mut(
                                            page,
                                            |pane, _state| {
                                                pane.testflight_release_form.clean_build = *checked;
                                                if let Some(form) = pane
                                                    .workflow_form_by_id_mut("publish_testflight")
                                                {
                                                    form.clear_error();
                                                }
                                                pane.latest_run = None;
                                            },
                                        );
                                        cx.notify();
                                    },
                                )),
                            )
                        },
                    ),
            )
            .into_any_element()
    }

    fn render_create_app_card(
        &self,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> impl IntoElement {
        let radius = Self::panel_radius(cx);
        let page = cx.entity().downgrade();
        let selected_project =
            self.workspace_project(self.create_app_form.selected_project_id.as_deref());
        let selected_scheme = selected_project.and_then(|project| {
            self.workspace_scheme(project, self.create_app_form.selected_scheme_id.as_deref())
        });

        let project_menu = ContextMenu::build(window, cx, |mut menu, _, _| {
            for project in self.workspace_projects() {
                let label = project.label.clone();
                let detail = project.display_path();
                let page = page.clone();
                let project_id = project.id.clone();
                menu = menu.entry(format!("{label} ({detail})"), None, move |window, cx| {
                    page.update(cx, |page, cx| {
                        with_app_store_connect_provider_mut(page, |pane, _state| {
                            pane.select_create_project(project_id.clone(), window, cx);
                        });
                    })
                    .ok();
                });
            }
            menu
        });

        let scheme_menu = ContextMenu::build(window, cx, |mut menu, _, _| {
            if let Some(project) = selected_project {
                for scheme in &project.schemes {
                    let page = page.clone();
                    let scheme_id = scheme.id.clone();
                    menu = menu.entry(scheme.label.clone(), None, move |window, cx| {
                        page.update(cx, |page, cx| {
                            with_app_store_connect_provider_mut(page, |pane, _state| {
                                pane.select_create_scheme(scheme_id.clone(), window, cx);
                            });
                        })
                        .ok();
                    });
                }
            }
            menu
        });

        let platform_menu = ContextMenu::build(window, cx, |mut menu, _, _| {
            for platform in ["IOS", "MAC_OS", "TV_OS", "UNIVERSAL"] {
                let page = page.clone();
                let platform_id = platform.to_string();
                menu = menu.entry(platform.to_string(), None, move |_window, cx| {
                    page.update(cx, |page, cx| {
                        with_app_store_connect_provider_mut(page, |pane, _state| {
                            pane.select_create_platform(platform_id.clone(), cx);
                        });
                    })
                    .ok();
                });
            }
            menu
        });

        v_flex()
            .gap_4()
            .p_5()
            .rounded(radius)
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().background)
            .child(self.render_panel_header(
                "Create App From Workspace Project",
                "Create a new App Store Connect app for a local Apple project, then use it for release work inside Qedit.",
            ))
            .child(
                h_flex()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        v_flex()
                            .gap_1()
                            .min_w(rems(18.))
                            .child(
                                Label::new("Workspace Project")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                            .child(Self::render_popover_button(
                                "asc-create-project",
                                selected_project
                                    .map(|project| project.label.clone())
                                    .unwrap_or_else(|| "Select project".to_string()),
                                project_menu,
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .min_w(rems(16.))
                            .child(
                                Label::new("Scheme")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                            .child(Self::render_popover_button(
                                "asc-create-scheme",
                                selected_scheme
                                    .map(|scheme| scheme.label.clone())
                                    .unwrap_or_else(|| "Select scheme".to_string()),
                                scheme_menu,
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .min_w(rems(12.))
                            .child(
                                Label::new("Platform")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                            .child(Self::render_popover_button(
                                "asc-create-platform",
                                self.create_app_form.selected_platform.clone(),
                                platform_menu,
                            )),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(self.create_app_form.app_name_input.clone())
                    .child(self.create_app_form.bundle_id_input.clone())
                    .child(self.create_app_form.sku_input.clone())
                    .child(
                        h_flex()
                            .gap_3()
                            .flex_wrap()
                            .child(
                                div()
                                    .min_w(rems(12.))
                                    .flex_1()
                                    .child(self.create_app_form.primary_locale_input.clone()),
                            )
                            .child(
                                div()
                                    .min_w(rems(12.))
                                    .flex_1()
                                    .child(self.create_app_form.initial_version_input.clone()),
                            ),
                    )
                    .child(self.create_app_form.company_name_input.clone())
                    .child(self.create_app_form.apple_id_input.clone())
                    .child(self.create_app_form.password_input.clone())
                    .child(self.create_app_form.two_factor_command_input.clone())
                    .child(
                        Checkbox::new(
                            "asc-create-internal-group",
                            self.create_app_form.create_internal_group,
                        )
                        .label("Create an internal TestFlight group")
                        .disabled(self.create_app_form.pending)
                        .on_click(cx.listener(|page, checked, _window, cx| {
                            with_app_store_connect_provider_mut(page, |pane, _state| {
                                pane.create_app_form.create_internal_group = *checked;
                            });
                            cx.notify();
                        })),
                    )
                    .when(self.create_app_form.create_internal_group.selected(), |this| {
                        this.child(self.create_app_form.internal_group_name_input.clone())
                    }),
            )
            .child(
                match &self.web_auth_state {
                    LoadState::Loading => {
                        Label::new("Checking ASC web-session status…")
                            .size(LabelSize::Small)
                            .color(Color::Muted)
                            .into_any_element()
                    }
                    LoadState::Error(error) => Label::new(error.clone())
                        .size(LabelSize::Small)
                        .color(Color::Muted)
                        .into_any_element(),
                    LoadState::Ready(summary) => Label::new(if summary.authenticated {
                        "ASC web session is ready."
                    } else {
                        "ASC web session is not authenticated. Provide credentials above or rely on a cached session."
                    })
                    .size(LabelSize::Small)
                    .color(Color::Muted)
                    .into_any_element(),
                },
            )
            .when_some(self.create_app_form.success_message.clone(), |this, message| {
                this.child(Label::new(message).size(LabelSize::Small).color(Color::Success))
            })
            .when_some(self.create_app_form.error_message.clone(), |this, error| {
                this.child(Label::new(error).size(LabelSize::Small).color(Color::Error))
            })
            .child(
                h_flex()
                    .justify_end()
                    .items_center()
                    .gap_2()
                    .child(ServicesPage::render_action_chip(
                            "asc-create-app-submit",
                            if self.create_app_form.pending {
                                "Creating…"
                            } else {
                                "Create App"
                            },
                            IconName::PlayFilled,
                            self.create_app_form.pending || self.workspace_projects().is_empty(),
                            cx.listener(|page, _, window, cx| {
                                let workspace = page.workspace().clone();
                                with_app_store_connect_provider_mut(page, |pane, state| {
                                    pane.submit_create_app(state, workspace, window, cx);
                                });
                            }),
                            cx,
                        )),
            )
    }

    fn render_build_status(&self, build: &AscBuildSummary) -> impl IntoElement {
        v_flex()
            .gap_0p5()
            .child(render_state_line(
                &build.processing_state,
                format_processing_state,
            ))
            .when_some(build.testflight_external_state.as_ref(), |cell, state| {
                cell.child(render_state_line(
                    &format!("TestFlight {state}"),
                    format_embedded_state,
                ))
            })
            .when_some(build.app_store_state.as_ref(), |cell, state| {
                cell.child(render_state_line(
                    &format!("App Store {state}"),
                    format_embedded_state,
                ))
            })
    }

    fn render_builds_table_header(&self, cx: &App) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .gap_4()
            .px_3()
            .py_2()
            .bg(cx.theme().colors().background)
            .border_b_1()
            .border_color(cx.theme().colors().border_variant)
            .child(
                div().min_w(rems(8.)).w(rems(8.)).child(
                    Label::new("Platform")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
            )
            .child(
                div().min_w(rems(10.)).w(rems(10.)).child(
                    Label::new("Release Type")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
            )
            .child(
                div().min_w(rems(12.)).w(rems(12.)).child(
                    Label::new("Date")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
            )
            .child(
                div().min_w(rems(12.)).w(rems(12.)).child(
                    Label::new("Status")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
            )
            .child(
                div().min_w(rems(12.)).w(rems(12.)).child(
                    Label::new("Build")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
            )
            .child(
                div().min_w(rems(14.)).flex_grow().child(
                    Label::new("Version")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
            )
    }

    fn render_builds_table_row(
        &self,
        build: &AscBuildSummary,
        row_index: usize,
        cx: &App,
    ) -> impl IntoElement {
        let row_background = if row_index.is_multiple_of(2) {
            cx.theme().colors().editor_background
        } else {
            cx.theme().colors().background
        };

        h_flex()
            .w_full()
            .flex_none()
            .items_start()
            .gap_4()
            .px_3()
            .py_3()
            .bg(row_background)
            .when(row_index > 0, |row| {
                row.border_t_1()
                    .border_color(cx.theme().colors().border_variant)
            })
            .child(div().min_w(rems(8.)).w(rems(8.)).child(
                Label::new(format_platform(build.platform.as_deref())).size(LabelSize::Small),
            ))
            .child(
                div()
                    .min_w(rems(10.))
                    .w(rems(10.))
                    .child(Label::new(build_release_type(build)).size(LabelSize::Small)),
            )
            .child(
                v_flex()
                    .min_w(rems(12.))
                    .w(rems(12.))
                    .gap_0p5()
                    .child(
                        Label::new(format_build_date(&build.uploaded_date)).size(LabelSize::Small),
                    )
                    .when_some(build.expiration_date.as_ref(), |cell, expiration_date| {
                        cell.child(
                            Label::new(format!("Expires {}", format_build_date(expiration_date)))
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                    }),
            )
            .child(
                div()
                    .min_w(rems(16.))
                    .w(rems(16.))
                    .child(self.render_build_status(build)),
            )
            .child(
                div()
                    .min_w(rems(12.))
                    .w(rems(12.))
                    .child(Label::new(build.build_number.clone()).size(LabelSize::Small)),
            )
            .child(
                v_flex().min_w(rems(14.)).flex_grow().child(
                    Label::new(
                        build
                            .marketing_version
                            .clone()
                            .unwrap_or_else(|| "Unknown Version".to_string()),
                    )
                    .size(LabelSize::Small),
                ),
            )
    }

    fn render_builds_table(
        &self,
        builds: &[AscBuildSummary],
        cx: &App,
    ) -> impl IntoElement + use<> {
        let radius = Self::panel_radius(cx);
        v_flex()
            .w_full()
            .min_w_0()
            .flex_none()
            .gap_0()
            .rounded(radius)
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .overflow_hidden()
            .child(self.render_builds_table_header(cx))
            .children(
                builds
                    .iter()
                    .enumerate()
                    .map(|(row_index, build)| self.render_builds_table_row(build, row_index, cx)),
            )
    }

    fn render_release_content(
        &self,
        state: &ServicesPageState,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .min_h_0()
            .child(
                v_flex()
                    .id("asc-release-scroll-content")
                    .track_scroll(&self.content_scroll_handle)
                    .size_full()
                    .min_w_0()
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .when(self.selected_app(state).is_none(), |this| {
                                this.child(self.render_empty_panel(
                                    "No app selected",
                                    "Choose an app to publish a build from the shared workflow controls above.",
                                    cx,
                                ))
                            }),
                    ),
            )
            .vertical_scrollbar_for(&self.content_scroll_handle, window, cx)
    }

    fn render_overview_content(
        &self,
        state: &ServicesPageState,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> impl IntoElement {
        let radius = Self::panel_radius(cx);
        let selected_app = self.selected_app(state);
        let selected_build = self.selected_build();

        div()
            .size_full()
            .min_h_0()
            .child(
                v_flex()
                    .id("asc-overview-scroll-content")
                    .track_scroll(&self.content_scroll_handle)
                    .size_full()
                    .min_w_0()
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .when_some(selected_app.clone(), |this, app| {
                                this.child(
                                    h_flex()
                                        .gap_3()
                                        .flex_wrap()
                                        .child(
                                            v_flex()
                                                .h(rems(18.))
                                                .min_w(rems(24.))
                                                .flex_1()
                                                .gap_4()
                                                .justify_between()
                                                .p_5()
                                                .rounded(radius)
                                                .border_1()
                                                .border_color(cx.theme().colors().border_variant)
                                                .bg(cx.theme().colors().background)
                                                .child(
                                                    v_flex()
                                                        .gap_1()
                                                        .child(Label::new("App Details").size(LabelSize::Small))
                                                        .child(
                                                            Label::new(app.name.clone())
                                                                .size(LabelSize::Large)
                                                                .single_line()
                                                                .truncate(),
                                                        )
                                                        .child(
                                                            Label::new(app.bundle_id.clone())
                                                                .size(LabelSize::Small)
                                                                .color(Color::Muted)
                                                                .single_line()
                                                                .truncate(),
                                                        ),
                                                )
                                                .child(
                                                    v_flex()
                                                        .gap_3()
                                                        .child(self.render_detail_row("SKU", app.sku.clone()))
                                                        .child(
                                                            self.render_detail_row(
                                                                "Primary Locale",
                                                                app.primary_locale
                                                                    .clone()
                                                                    .unwrap_or_else(|| "Not Set".to_string()),
                                                            ),
                                                        )
                                                        .child(self.render_detail_row("App ID", app.id.clone())),
                                                ),
                                        )
                                        .child(
                                            v_flex()
                                                .h(rems(18.))
                                                .min_w(rems(24.))
                                                .flex_1()
                                                .gap_4()
                                                .justify_between()
                                                .p_5()
                                                .rounded(radius)
                                                .border_1()
                                                .border_color(cx.theme().colors().border_variant)
                                                .bg(cx.theme().colors().background)
                                                .child(Label::new("Latest Build").size(LabelSize::Small))
                                                .when_some(selected_build, |panel, build| {
                                                    panel
                                                        .child(
                                                            v_flex()
                                                                .gap_1()
                                                                .child(
                                                                    Label::new(
                                                                        build
                                                                            .marketing_version
                                                                            .clone()
                                                                            .unwrap_or_else(|| {
                                                                                "Unknown Version".to_string()
                                                                            }),
                                                                    )
                                                                    .size(LabelSize::Large),
                                                                )
                                                                .child(
                                                                    Label::new(format!(
                                                                        "Build {}",
                                                                        build.build_number
                                                                    ))
                                                                    .size(LabelSize::Small)
                                                                    .color(Color::Muted),
                                                                ),
                                                        )
                                                        .child(
                                                            v_flex()
                                                                .gap_3()
                                                                .child(self.render_detail_row(
                                                                    "Platform",
                                                                    format_platform(build.platform.as_deref()),
                                                                ))
                                                                .child(self.render_detail_row(
                                                                    "Status",
                                                                    build_status_summary(build),
                                                                ))
                                                                .child(self.render_detail_row(
                                                                    "Uploaded",
                                                                    format_build_date(&build.uploaded_date),
                                                                ))
                                                                .child(
                                                                    self.render_detail_row(
                                                                        "Expires",
                                                                        build
                                                                            .expiration_date
                                                                            .as_ref()
                                                                            .map(|date| format_build_date(date))
                                                                            .unwrap_or_else(|| {
                                                                                "Not Set".to_string()
                                                                            }),
                                                                    ),
                                                                ),
                                                        )
                                                })
                                                .when(selected_build.is_none(), |panel| {
                                                    panel.child(
                                                        Label::new("No builds are available for the selected app.")
                                                            .size(LabelSize::Small)
                                                            .color(Color::Muted),
                                                    )
                                                }),
                                        ),
                                )
                            })
                            .when(selected_app.is_none(), |this| {
                                this.child(self.render_empty_panel(
                                    "No app selected",
                                    "Choose an app from the top bar to inspect its release data.",
                                    cx,
                                ))
                            })
                            .child(self.render_create_app_card(window, cx)),
                    ),
            )
            .vertical_scrollbar_for(&self.content_scroll_handle, window, cx)
    }

    fn render_builds_content(
        &self,
        state: &ServicesPageState,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> impl IntoElement {
        let selected_app = self.selected_app(state);
        let content = match &self.builds_state {
            LoadState::Loading => Label::new("Loading builds…")
                .color(Color::Muted)
                .into_any_element(),
            LoadState::Error(error) => v_flex()
                .gap_1()
                .child(Label::new("Could not load builds").color(Color::Error))
                .child(
                    Label::new(error.clone())
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
            LoadState::Ready(_) if selected_app.is_none() => {
                Label::new("Select an app to load its builds.")
                    .color(Color::Muted)
                    .into_any_element()
            }
            LoadState::Ready(builds_state) if builds_state.builds.is_empty() => {
                Label::new("No builds were returned for the selected app.")
                    .color(Color::Muted)
                    .into_any_element()
            }
            LoadState::Ready(builds_state) if selected_app.is_some() => {
                let is_loading_more = builds_state.is_loading_more;
                let load_more_error = builds_state.load_more_error.clone();
                let has_next_page = builds_state.next_page_url.is_some();
                v_flex()
                    .gap_4()
                    .child(self.render_builds_table(&builds_state.builds, cx))
                    .when(has_next_page || load_more_error.is_some(), |this| {
                        this.child(
                            v_flex()
                                .w_full()
                                .items_center()
                                .gap_2()
                                .when_some(load_more_error, |this, error| {
                                    this.child(
                                        Label::new(error)
                                            .size(LabelSize::Small)
                                            .color(Color::Error),
                                    )
                                })
                                .when(has_next_page, |this| {
                                    this.child(
                                        Button::new("services-load-more-builds", "Load more")
                                            .style(ButtonStyle::Subtle)
                                            .size(ButtonSize::Compact)
                                            .disabled(is_loading_more)
                                            .on_click(cx.listener(|page, _, window, cx| {
                                                with_app_store_connect_provider_mut(
                                                    page,
                                                    |pane, state| {
                                                        pane.load_more_builds(state, window, cx);
                                                    },
                                                );
                                            })),
                                    )
                                }),
                        )
                    })
                    .into_any_element()
            }
            LoadState::Ready(_) => Label::new("Select an app to load its builds.")
                .color(Color::Muted)
                .into_any_element(),
        };

        v_flex().size_full().min_h_0().child(
            v_flex()
                .flex_1()
                .min_h_0()
                .gap_3()
                .child(
                    h_flex().justify_end().child(
                        IconButton::new("services-refresh-builds", IconName::RotateCw)
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .disabled(selected_app.is_none())
                            .on_click(cx.listener(|page, _, window, cx| {
                                with_app_store_connect_provider_mut(page, |pane, state| {
                                    pane.refresh_builds(state, window, cx);
                                });
                            })),
                    ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .child(
                            v_flex()
                                .id("services-builds-scroll-content")
                                .track_scroll(&self.builds_scroll_handle)
                                .size_full()
                                .min_w_0()
                                .overflow_y_scroll()
                                .child(content),
                        )
                        .vertical_scrollbar_for(&self.builds_scroll_handle, window, cx),
                ),
        )
    }
}

impl ServiceWorkspaceAdapter for AppStoreConnectWorkspaceProvider {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn descriptor(&self) -> &ServiceProviderDescriptor {
        self.descriptor()
    }

    fn normalize_state(&self, state: &mut ServicesPageState) {
        self.normalize_state(state);
    }

    fn refresh(
        &mut self,
        state: &mut ServicesPageState,
        workspace_paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        self.refresh(state, workspace_paths, window, cx);
    }

    #[cfg(target_os = "macos")]
    fn resource_menu(&self, state: &ServicesPageState) -> Option<ServiceResourceMenuModel> {
        self.resource_menu(state)
    }

    #[cfg(target_os = "macos")]
    fn select_resource(
        &mut self,
        state: &mut ServicesPageState,
        resource_id: String,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        self.select_resource(state, resource_id, window, cx);
    }

    fn render_section(
        &self,
        state: &ServicesPageState,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> AnyElement {
        self.render_section(state, window, cx)
    }

    fn workflow_ui_model(&self, state: &ServicesPageState) -> Option<ServiceWorkflowUiModel> {
        self.workflow_ui_model(state)
    }

    fn render_workflow_form(
        &self,
        state: &ServicesPageState,
        workflow_ui: &ServiceWorkflowUiModel,
        page: WeakEntity<ServicesPage>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> Option<AnyElement> {
        (state.selected_workflow_id.as_deref() == Some("publish_testflight"))
            .then(|| self.render_testflight_workflow_form(page, workflow_ui, window, cx))
    }

    fn handle_workflow_ui_action(
        &mut self,
        state: &mut ServicesPageState,
        action: ServiceWorkflowUiAction,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        if !self.cli_ready() {
            return;
        }

        match action {
            ServiceWorkflowUiAction::SelectTarget { target_id } => {
                self.select_target(state, target_id);
                cx.notify();
            }
            ServiceWorkflowUiAction::SelectWorkflow { workflow_id } => {
                self.select_workflow(state, workflow_id);
                cx.notify();
            }
            ServiceWorkflowUiAction::Submit => self.submit_workflow(state, window, cx),
            ServiceWorkflowUiAction::PickFile { field_key } => {
                self.pick_workflow_file(state, field_key, workspace, window, cx);
            }
            ServiceWorkflowUiAction::SetToggle { field_key, value } => {
                if let Some(form) = self.selected_workflow_form_mut(state) {
                    form.set_toggle(&field_key, value);
                }
                cx.notify();
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn auth_ui_model(&self) -> Option<ServiceAuthUiModel> {
        if !self.cli_ready() {
            return None;
        }

        Some(ServiceAuthUiModel {
            provider_id: self.descriptor.id.clone(),
            authenticate_label: "Authenticate".into(),
            reauthenticate_label: "Re-authenticate".into(),
            logout_label: "Log Out".into(),
            status: self.auth_status_summary(),
            form: self.auth_form.clone(),
        })
    }

    #[cfg(target_os = "macos")]
    fn handle_auth_ui_action(
        &mut self,
        _state: &mut ServicesPageState,
        action: ServiceAuthUiAction,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) {
        if !self.cli_ready() {
            return;
        }

        match action {
            ServiceAuthUiAction::ShowAuthenticate => {
                self.show_authenticate_form();
                cx.notify();
            }
            ServiceAuthUiAction::CancelAuthenticate => {
                self.cancel_authenticate_form();
                cx.notify();
            }
            ServiceAuthUiAction::SubmitAuthenticate => {
                self.submit_authenticate(workspace, window, cx)
            }
            ServiceAuthUiAction::Logout => self.logout(workspace, window, cx),
            ServiceAuthUiAction::PickFile { field_key } => {
                self.pick_auth_file(field_key, workspace, window, cx);
            }
            ServiceAuthUiAction::SetToggle { field_key, value } => {
                self.auth_form.set_toggle(&field_key, value);
                cx.notify();
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn render_sidebar_footer_extra(
        &self,
        _state: &ServicesPageState,
        _window: &mut Window,
        cx: &mut Context<ServicesPage>,
    ) -> Option<AnyElement> {
        self.render_cli_sidebar_status(cx)
    }
}

#[derive(Clone, Debug)]
struct WorkflowExecutionResult {
    output: String,
    error: Option<String>,
}

#[derive(Clone, Debug)]
struct CreateAppResult {
    app: AscAppSummary,
    apps: Vec<AscAppSummary>,
    project: AppleWorkspaceProjectSummary,
    group_warning: Option<String>,
}

struct AscTwoFactorModal {
    focus_handle: FocusHandle,
    code_input: Entity<InputField>,
    code_file_path: PathBuf,
}

impl AscTwoFactorModal {
    fn new(code_file_path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            code_input: new_text_input(window, cx, "Verification Code", "123456", false),
            code_file_path,
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let code = self.code_input.read(cx).text(cx).trim().to_string();
        if code.is_empty() {
            return;
        }

        let _ = persist_two_factor_code(&self.code_file_path, &code);
        cx.emit(DismissEvent);
    }
}

impl EventEmitter<DismissEvent> for AscTwoFactorModal {}

impl Focusable for AscTwoFactorModal {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ModalView for AscTwoFactorModal {
    fn fade_out_background(&self) -> bool {
        true
    }

    fn on_before_dismiss(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> DismissDecision {
        if !two_factor_code_file_has_contents(&self.code_file_path) {
            let _ = persist_two_factor_code(&self.code_file_path, "cancel");
        }
        DismissDecision::Dismiss(true)
    }
}

impl Render for AscTwoFactorModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("AscTwoFactorModal")
            .occlude()
            .elevation_3(cx)
            .w(rems(28.))
            .track_focus(&self.focus_handle)
            .child(
                Modal::new("asc-two-factor-modal", None)
                    .header(
                        ModalHeader::new()
                            .headline("Apple Verification Code")
                            .description(
                                "After you click Create App, Apple may show a native verification prompt. Paste that 2FA code here and confirm to continue.",
                            )
                            .show_dismiss_button(true),
                    )
                    .child(
                        v_flex()
                            .gap_3()
                            .p_4()
                            .child(self.code_input.clone())
                            .child(
                                Label::new(
                                    "If you close this modal without confirming, the current create-app attempt will be cancelled.",
                                )
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                            ),
                    )
                    .footer(
                        ModalFooter::new().end_slot(
                            h_flex()
                                .gap_2()
                                .child(ServicesPage::render_action_chip(
                                    "asc-two-factor-cancel",
                                    "Cancel",
                                    IconName::Close,
                                    false,
                                    cx.listener(|_, _, _, cx| cx.emit(DismissEvent)),
                                    cx,
                                ))
                                .child(ServicesPage::render_action_chip(
                                    "asc-two-factor-confirm",
                                    "Confirm",
                                    IconName::Check,
                                    false,
                                    cx.listener(|this, _, _, cx| this.submit(cx)),
                                    cx,
                                )),
                        ),
                    ),
            )
    }
}

fn summarize_workflow_error(error: &str) -> String {
    error
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.trim_start_matches("Error: ").to_string())
        .unwrap_or_else(|| "Workflow failed.".to_string())
}

fn is_testflight_distribution_pending_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("failed to add groups")
        && (error.contains("not in an internally testable state")
            || error.contains("not in an externally testable state")
            || error.contains("build is not assignable"))
}

fn summarize_workflow_output(output: &str) -> String {
    output
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| "Workflow completed successfully.".to_string())
}

#[derive(Deserialize)]
struct AscAppsResponse {
    data: Vec<AscAppRecord>,
}

#[derive(Deserialize)]
struct AscAppRecord {
    id: String,
    attributes: AscAppAttributes,
}

#[derive(Deserialize)]
struct AscAppAttributes {
    name: String,
    #[serde(rename = "bundleId")]
    bundle_id: String,
    sku: String,
    #[serde(rename = "primaryLocale")]
    primary_locale: Option<String>,
}

#[derive(Deserialize)]
struct AscWebAuthStatusResponse {
    authenticated: bool,
}

#[derive(Deserialize)]
struct AscBuildsResponse {
    data: Vec<AscBuildRecord>,
    #[serde(default)]
    included: Vec<AscPreReleaseVersionRecord>,
    #[serde(default)]
    links: AscPaginationLinks,
}

#[derive(Deserialize)]
struct AscBuildRecord {
    id: String,
    attributes: AscBuildAttributes,
    #[serde(default)]
    relationships: AscBuildRelationships,
}

#[derive(Deserialize)]
struct AscBuildAttributes {
    version: String,
    #[serde(rename = "uploadedDate")]
    uploaded_date: String,
    #[serde(rename = "expirationDate")]
    expiration_date: Option<String>,
    #[serde(rename = "processingState")]
    processing_state: String,
}

#[derive(Default, Deserialize)]
struct AscBuildRelationships {
    #[serde(rename = "preReleaseVersion")]
    pre_release_version: Option<AscBuildRelationship>,
}

#[derive(Deserialize)]
struct AscBuildRelationship {
    #[serde(default)]
    data: Option<AscRelatedResourceIdentifier>,
}

#[derive(Deserialize)]
struct AscRelatedResourceIdentifier {
    id: String,
}

#[derive(Deserialize)]
struct AscPreReleaseVersionRecord {
    #[serde(rename = "type")]
    resource_type: String,
    id: String,
    attributes: AscPreReleaseVersionAttributes,
}

#[derive(Clone, Deserialize)]
struct AscPreReleaseVersionAttributes {
    version: String,
    platform: String,
}

#[derive(Default, Deserialize)]
struct AscPaginationLinks {
    #[serde(default)]
    next: String,
}

fn trimmed_input_text(input: &Entity<InputField>, cx: &App) -> String {
    input.read(cx).text(cx).trim().to_string()
}

fn asc_cli_missing_message() -> String {
    "This provider requires a local ASC CLI installation.".to_string()
}

fn parse_asc_cli_probe(path_output: &str, version_output: &str) -> Result<AscCliSummary, String> {
    let path = path_output.trim();
    if path.is_empty() {
        return Err(asc_cli_missing_message());
    }

    let version = version_output.trim();
    if version.is_empty() {
        return Err(format!(
            "ASC CLI was found at {path}, but Qedit could not read its version."
        ));
    }

    Ok(AscCliSummary {
        path: path.to_string(),
        version: version.to_string(),
    })
}

fn command_output_detail(stdout: &[u8], stderr: &[u8], fallback: &str) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }

    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    if !stdout.is_empty() {
        return stdout;
    }

    fallback.to_string()
}

// Failure modes:
// - `asc` is missing from PATH.
// - `asc version` fails because the installation is incomplete or broken.
// - Homebrew is unavailable, so one-click installation cannot proceed.
// - Homebrew installation fails and Qedit must surface actionable command output.
fn load_asc_cli_state() -> AscCliState {
    let path_output = match std::process::Command::new("zsh")
        .args(["-lc", "command -v asc"])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            return AscCliState::Missing(format!("{} {}", asc_cli_missing_message(), error));
        }
    };

    if !path_output.status.success() {
        return AscCliState::Missing(asc_cli_missing_message());
    }

    let version_output = match std::process::Command::new("asc").arg("version").output() {
        Ok(output) => output,
        Err(error) => {
            return AscCliState::Missing(format!("{} {}", asc_cli_missing_message(), error));
        }
    };

    if !version_output.status.success() {
        let path = String::from_utf8_lossy(&path_output.stdout)
            .trim()
            .to_string();
        return AscCliState::InstallFailed(format!(
            "ASC CLI was found at {path}, but `asc version` failed: {}",
            command_output_detail(
                &version_output.stdout,
                &version_output.stderr,
                "`asc version` exited unsuccessfully",
            )
        ));
    }

    match parse_asc_cli_probe(
        &String::from_utf8_lossy(&path_output.stdout),
        &String::from_utf8_lossy(&version_output.stdout),
    ) {
        Ok(summary) => AscCliState::Ready(summary),
        Err(error) => AscCliState::InstallFailed(error),
    }
}

async fn install_asc_cli() -> Result<()> {
    new_command("brew")
        .arg("--version")
        .output()
        .await
        .with_context(|| {
            format!(
                "Automatic ASC CLI installation currently requires Homebrew. Install it manually from {}.",
                ASC_CLI_INSTALL_URL
            )
        })?;

    let output = new_command("brew")
        .args(["install", "asc"])
        .output()
        .await
        .with_context(|| "Failed to start Homebrew while installing ASC CLI")?;

    if !output.status.success() {
        anyhow::bail!(
            "Homebrew failed to install ASC CLI: {}",
            command_output_detail(
                &output.stdout,
                &output.stderr,
                "`brew install asc` exited unsuccessfully",
            )
        );
    }

    match load_asc_cli_state() {
        AscCliState::Ready(_) => Ok(()),
        AscCliState::Missing(detail) | AscCliState::InstallFailed(detail) => anyhow::bail!(detail),
        AscCliState::Checking | AscCliState::Installing => {
            anyhow::bail!("ASC CLI installation did not finish cleanly.")
        }
    }
}

async fn load_apps() -> Result<Vec<AscAppSummary>> {
    let response: AscAppsResponse = run_json_operation(ServiceOperationRequest {
        provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
        operation: "list_apps".to_string(),
        resource: None,
        artifact: None,
        input: [("paginate".to_string(), "true".to_string())]
            .into_iter()
            .collect(),
    })
    .await?;

    let mut apps = response
        .data
        .into_iter()
        .map(|app| AscAppSummary {
            id: app.id,
            name: app.attributes.name,
            bundle_id: app.attributes.bundle_id,
            sku: app.attributes.sku,
            primary_locale: app.attributes.primary_locale,
        })
        .collect::<Vec<_>>();
    apps.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.bundle_id.cmp(&right.bundle_id))
    });
    Ok(apps)
}

async fn load_app_by_bundle_id(bundle_id: String) -> Result<Option<AscAppSummary>> {
    let response: AscAppsResponse = run_json_operation(ServiceOperationRequest {
        provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
        operation: "list_apps".to_string(),
        resource: None,
        artifact: None,
        input: [("bundle_id".to_string(), bundle_id)].into_iter().collect(),
    })
    .await?;

    Ok(response.data.into_iter().next().map(|app| AscAppSummary {
        id: app.id,
        name: app.attributes.name,
        bundle_id: app.attributes.bundle_id,
        sku: app.attributes.sku,
        primary_locale: app.attributes.primary_locale,
    }))
}

async fn wait_for_created_app(
    bundle_id: String,
    app_name: String,
    sku: String,
) -> Result<Option<AscAppSummary>> {
    for _ in 0..10 {
        if let Some(app) = load_app_by_bundle_id(bundle_id.clone()).await? {
            return Ok(Some(app));
        }

        let apps = load_apps().await?;
        if let Some(app) = apps.into_iter().find(|app| {
            app.bundle_id == bundle_id
                || app.sku == sku
                || app.name == app_name
                || app.name.starts_with(&format!("{app_name} - "))
        }) {
            return Ok(Some(app));
        }

        thread::sleep(Duration::from_secs(1));
    }

    Ok(None)
}

async fn create_internal_group_with_retry(
    app: &AscAppSummary,
    group_name: String,
) -> Result<(), String> {
    let mut last_error = None;

    for _ in 0..10 {
        let result = run_json_operation::<serde_json::Value>(ServiceOperationRequest {
            provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
            operation: "create_testflight_group".to_string(),
            resource: Some(ServiceResourceRef {
                provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
                kind: "app".to_string(),
                external_id: app.id.clone(),
                label: app.name.clone(),
            }),
            artifact: None,
            input: [
                ("name".to_string(), group_name.clone()),
                ("internal".to_string(), "true".to_string()),
            ]
            .into_iter()
            .collect(),
        })
        .await;

        match result {
            Ok(_) => return Ok(()),
            Err(error) => {
                last_error = Some(error.to_string());
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "Failed to create internal TestFlight group.".to_string()))
}

async fn load_web_auth_status() -> Result<AscWebAuthSummary> {
    let response: AscWebAuthStatusResponse = run_json_operation(ServiceOperationRequest {
        provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
        operation: "web_auth_status".to_string(),
        resource: None,
        artifact: None,
        input: BTreeMap::new(),
    })
    .await?;

    Ok(AscWebAuthSummary {
        authenticated: response.authenticated,
    })
}

async fn load_workspace_projects(
    workspace_paths: Vec<PathBuf>,
) -> Result<Vec<AppleWorkspaceProjectSummary>> {
    let runner = SystemCommandRunner;
    let catalog = RuntimeCatalog::discover(&workspace_paths, &runner);
    let mut projects = Vec::new();

    for project in catalog.projects {
        if !matches!(
            project.kind,
            ProjectKind::AppleProject | ProjectKind::AppleWorkspace
        ) {
            continue;
        }

        let mut schemes = Vec::new();
        for target in project.targets {
            schemes.push(load_workspace_scheme_metadata(
                &project.project_path,
                &project.kind,
                target.label,
            ));
        }

        if schemes.is_empty() {
            continue;
        }

        projects.push(AppleWorkspaceProjectSummary {
            id: project.id,
            label: project.label,
            project_path: project.project_path,
            project_kind: project.kind,
            schemes,
        });
    }

    projects.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(projects)
}

fn load_workspace_scheme_metadata(
    project_path: &std::path::Path,
    project_kind: &ProjectKind,
    scheme: String,
) -> AppleWorkspaceSchemeSummary {
    let mut command = std::process::Command::new("xcodebuild");
    match project_kind {
        ProjectKind::AppleWorkspace => {
            command.arg("-workspace").arg(project_path);
        }
        ProjectKind::AppleProject => {
            command.arg("-project").arg(project_path);
        }
        ProjectKind::GpuiApplication => {}
    }
    command
        .arg("-scheme")
        .arg(&scheme)
        .arg("-showBuildSettings")
        .arg("-configuration")
        .arg("Release");

    let output = command.output().ok();
    let stdout = output
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default();

    let bundle_id = build_setting(&stdout, "PRODUCT_BUNDLE_IDENTIFIER");
    let marketing_version = build_setting(&stdout, "MARKETING_VERSION");
    let build_number = build_setting(&stdout, "CURRENT_PROJECT_VERSION");
    let platform = infer_platform(&stdout);

    AppleWorkspaceSchemeSummary {
        id: scheme.clone(),
        label: scheme,
        bundle_id,
        marketing_version,
        build_number,
        platform,
    }
}

fn build_setting(output: &str, key: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (candidate_key, value) = line.split_once(" = ")?;
        (candidate_key.trim() == key).then(|| value.trim().to_string())
    })
}

fn infer_platform(output: &str) -> Option<String> {
    let sdk_root = build_setting(output, "SDKROOT")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let supported_platforms = build_setting(output, "SUPPORTED_PLATFORMS")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let combined = format!("{sdk_root} {supported_platforms}");

    if combined.contains("iphone") {
        Some("IOS".to_string())
    } else if combined.contains("macos") {
        Some("MAC_OS".to_string())
    } else if combined.contains("appletv") {
        Some("TV_OS".to_string())
    } else if combined.contains("xros") || combined.contains("vision") {
        Some("VISION_OS".to_string())
    } else {
        None
    }
}

async fn load_builds_page(
    app: &AscAppSummary,
    next_page_url: Option<String>,
) -> Result<AscBuildPage> {
    let input = if let Some(next_page_url) = next_page_url {
        [("next".to_string(), next_page_url)].into_iter().collect()
    } else {
        [
            ("limit".to_string(), ASC_BUILDS_PAGE_SIZE.to_string()),
            ("sort".to_string(), "-uploadedDate".to_string()),
        ]
        .into_iter()
        .collect()
    };
    let response: AscBuildsResponse = run_json_operation(ServiceOperationRequest {
        provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
        operation: "list_builds".to_string(),
        resource: Some(ServiceResourceRef {
            provider_id: APP_STORE_CONNECT_PROVIDER_ID.to_string(),
            kind: "app".to_string(),
            external_id: app.id.clone(),
            label: app.name.clone(),
        }),
        artifact: None,
        input,
    })
    .await?;

    Ok(build_page_from_response(response))
}

fn build_page_from_response(response: AscBuildsResponse) -> AscBuildPage {
    let pre_release_versions = response
        .included
        .into_iter()
        .filter(|record| record.resource_type == "preReleaseVersions")
        .map(|record| (record.id, record.attributes))
        .collect::<BTreeMap<_, _>>();

    let builds = response
        .data
        .into_iter()
        .map(|build| {
            let pre_release_version = build
                .relationships
                .pre_release_version
                .and_then(|relationship| relationship.data)
                .and_then(|identifier| pre_release_versions.get(&identifier.id).cloned());

            AscBuildSummary {
                id: build.id,
                build_number: build.attributes.version,
                marketing_version: pre_release_version
                    .as_ref()
                    .map(|version| version.version.clone()),
                platform: pre_release_version
                    .as_ref()
                    .map(|version| version.platform.clone()),
                processing_state: build.attributes.processing_state,
                uploaded_date: build.attributes.uploaded_date,
                expiration_date: build.attributes.expiration_date,
                testflight_internal_state: None,
                testflight_external_state: None,
                app_store_version_id: None,
                app_store_state: None,
            }
        })
        .collect::<Vec<_>>();

    AscBuildPage {
        builds,
        next_page_url: non_empty_string(response.links.next),
    }
}

fn format_platform(platform: Option<&str>) -> String {
    match platform {
        Some("IOS") => "iOS".to_string(),
        Some("MAC_OS") => "macOS".to_string(),
        Some("TV_OS") => "tvOS".to_string(),
        Some("VISION_OS") => "visionOS".to_string(),
        Some(platform) => platform.replace('_', " "),
        None => "Unknown".to_string(),
    }
}

fn format_processing_state(processing_state: &str) -> String {
    processing_state
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut characters = segment.chars();
            let Some(first) = characters.next() else {
                return String::new();
            };

            format!(
                "{}{}",
                first.to_ascii_uppercase(),
                characters.as_str().to_ascii_lowercase()
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_build_date(timestamp: &str) -> String {
    let Some((date, time_with_offset)) = timestamp.split_once('T') else {
        return timestamp.to_string();
    };

    let time = time_with_offset.chars().take(5).collect::<String>();
    format!("{date} {time}")
}

fn build_release_type(build: &AscBuildSummary) -> String {
    let in_testflight =
        build.testflight_internal_state.is_some() || build.testflight_external_state.is_some();
    let in_app_store = build.app_store_version_id.is_some();

    match (in_testflight, in_app_store) {
        (true, true) => "TestFlight + App Store".to_string(),
        (true, false) => "TestFlight".to_string(),
        (false, true) => "App Store".to_string(),
        (false, false) => "Unknown".to_string(),
    }
}

fn build_status_summary(build: &AscBuildSummary) -> String {
    let mut states = vec![format_processing_state(&build.processing_state)];
    if let Some(state) = &build.testflight_external_state {
        states.push(format!("TF {}", format_processing_state(state)));
    }
    if let Some(state) = &build.app_store_state {
        states.push(format!("AS {}", format_processing_state(state)));
    }
    states.join(" · ")
}

fn color_for_state(value: &str) -> Color {
    let value = value.to_ascii_uppercase();
    if value.contains("REJECTED")
        || value.contains("INVALID")
        || value.contains("FAILED")
        || value.contains("ERROR")
    {
        Color::Error
    } else if value.contains("WAITING")
        || value.contains("IN_REVIEW")
        || value.contains("FOR_REVIEW")
        || value.contains("PROCESSING")
        || value.contains("PENDING")
        || value.contains("PREPARE")
        || value.contains("SUBMITTED")
    {
        Color::Warning
    } else if value.contains("READY")
        || value.contains("VALID")
        || value.contains("ACTIVE")
        || value.contains("APPROVED")
        || value.contains("TESTING")
    {
        Color::Success
    } else {
        Color::Muted
    }
}

fn render_state_line(value: &str, formatter: impl Fn(&str) -> String) -> impl IntoElement {
    let color = color_for_state(value);

    h_flex()
        .items_center()
        .gap_1p5()
        .child(Indicator::dot().color(color))
        .child(
            Label::new(formatter(value))
                .size(LabelSize::Small)
                .color(color),
        )
}

fn format_embedded_state(value: &str) -> String {
    let Some((label, state)) = value.split_once(' ') else {
        return format_processing_state(value);
    };
    format!("{label} {}", format_processing_state(state))
}

fn non_empty_string(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use app_runtime::ProjectKind;
    use serde_json::json;

    use super::{
        AppleWorkspaceSchemeSummary, AscBuildsResponse, AscTestFlightSourceMode,
        AscTestFlightWorkflowValues, asc_cli_missing_message, build_page_from_response,
        build_testflight_workflow_input, command_output_detail,
        is_testflight_distribution_pending_error, parse_asc_cli_probe, summarize_workflow_error,
        validate_local_project_bundle_id_match,
    };

    #[test]
    fn parses_build_pages_without_eager_follow_up_requests() {
        let response: AscBuildsResponse = serde_json::from_value(json!({
            "data": [
                {
                    "id": "build-1",
                    "attributes": {
                        "version": "42",
                        "uploadedDate": "2026-04-05T12:34:56Z",
                        "expirationDate": "2026-05-05T12:34:56Z",
                        "processingState": "VALID"
                    },
                    "relationships": {
                        "preReleaseVersion": {
                            "data": {
                                "id": "prv-1"
                            }
                        }
                    }
                }
            ],
            "included": [
                {
                    "type": "preReleaseVersions",
                    "id": "prv-1",
                    "attributes": {
                        "version": "1.2.3",
                        "platform": "IOS"
                    }
                }
            ],
            "links": {
                "next": "https://api.appstoreconnect.apple.com/v1/builds?cursor=AQ"
            }
        }))
        .unwrap();

        let page = build_page_from_response(response);

        assert_eq!(page.builds.len(), 1);
        assert_eq!(page.builds[0].marketing_version.as_deref(), Some("1.2.3"));
        assert_eq!(page.builds[0].platform.as_deref(), Some("IOS"));
        assert_eq!(
            page.next_page_url.as_deref(),
            Some("https://api.appstoreconnect.apple.com/v1/builds?cursor=AQ")
        );
        assert!(page.builds[0].testflight_external_state.is_none());
        assert!(page.builds[0].app_store_state.is_none());
    }

    #[test]
    fn parses_asc_cli_probe_output() {
        let summary =
            parse_asc_cli_probe("/opt/homebrew/bin/asc\n", "1.0.1 (commit: abcdef)\n").unwrap();

        assert_eq!(summary.path, "/opt/homebrew/bin/asc");
        assert_eq!(summary.version, "1.0.1 (commit: abcdef)");
    }

    #[test]
    fn rejects_empty_asc_cli_probe_path() {
        let error = parse_asc_cli_probe("", "1.0.1").unwrap_err();

        assert_eq!(error, asc_cli_missing_message());
    }

    #[test]
    fn command_output_detail_prefers_stderr() {
        let detail = command_output_detail(b"stdout text", b"stderr text", "fallback");

        assert_eq!(detail, "stderr text");
    }

    #[test]
    fn builds_testflight_local_project_input() {
        let input = build_testflight_workflow_input(AscTestFlightWorkflowValues {
            source_mode: AscTestFlightSourceMode::LocalProject,
            project_kind: Some(ProjectKind::AppleProject),
            project_path: Some(PathBuf::from("/tmp/IOSSample.xcodeproj")),
            scheme: Some(AppleWorkspaceSchemeSummary {
                id: "IOSSample".to_string(),
                label: "IOSSample".to_string(),
                bundle_id: Some("com.example.sample".to_string()),
                marketing_version: Some("1.0".to_string()),
                build_number: Some("7".to_string()),
                platform: Some("IOS".to_string()),
            }),
            ipa_path: String::new(),
            version: "1.0".to_string(),
            build_id: String::new(),
            build_number: String::new(),
            group: "Internal Testers".to_string(),
            configuration: "Release".to_string(),
            export_options: "/tmp/ExportOptions.plist".to_string(),
            wait_for_processing: true,
            notify_testers: false,
            clean_build: true,
        })
        .unwrap();

        assert_eq!(
            input.get("project_path").map(String::as_str),
            Some("/tmp/IOSSample.xcodeproj")
        );
        assert_eq!(input.get("scheme").map(String::as_str), Some("IOSSample"));
        assert_eq!(input.get("platform").map(String::as_str), Some("IOS"));
        assert_eq!(input.get("clean").map(String::as_str), Some("true"));
        assert_eq!(input.get("wait").map(String::as_str), Some("true"));
    }

    #[test]
    fn rejects_existing_build_mode_without_build_identifier() {
        let error = build_testflight_workflow_input(AscTestFlightWorkflowValues {
            source_mode: AscTestFlightSourceMode::ExistingBuild,
            project_kind: None,
            project_path: None,
            scheme: None,
            ipa_path: String::new(),
            version: String::new(),
            build_id: String::new(),
            build_number: String::new(),
            group: "Internal Testers".to_string(),
            configuration: String::new(),
            export_options: String::new(),
            wait_for_processing: false,
            notify_testers: false,
            clean_build: false,
        })
        .unwrap_err();

        assert_eq!(error, "Build ID or Build Number is required");
    }

    #[test]
    fn rejects_local_project_mode_without_export_options() {
        let error = build_testflight_workflow_input(AscTestFlightWorkflowValues {
            source_mode: AscTestFlightSourceMode::LocalProject,
            project_kind: Some(ProjectKind::AppleProject),
            project_path: Some(PathBuf::from("/tmp/IOSSample.xcodeproj")),
            scheme: Some(AppleWorkspaceSchemeSummary {
                id: "IOSSample".to_string(),
                label: "IOSSample".to_string(),
                bundle_id: Some("com.example.sample".to_string()),
                marketing_version: Some("1.0".to_string()),
                build_number: Some("7".to_string()),
                platform: Some("IOS".to_string()),
            }),
            ipa_path: String::new(),
            version: "1.0".to_string(),
            build_id: String::new(),
            build_number: String::new(),
            group: "Internal Testers".to_string(),
            configuration: "Release".to_string(),
            export_options: String::new(),
            wait_for_processing: true,
            notify_testers: false,
            clean_build: false,
        })
        .unwrap_err();

        assert_eq!(
            error,
            "Export Options is required for local project publishing."
        );
    }

    #[test]
    fn rejects_testflight_local_project_when_bundle_id_does_not_match_selected_app() {
        let error = validate_local_project_bundle_id_match(
            "com.qedit.tests.iossample4",
            &AppleWorkspaceSchemeSummary {
                id: "IOSSample".to_string(),
                label: "IOSSample".to_string(),
                bundle_id: Some("com.qedit.tests.iossample".to_string()),
                marketing_version: Some("1.0".to_string()),
                build_number: Some("1".to_string()),
                platform: Some("IOS".to_string()),
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            "The selected App Store Connect app uses bundle ID `com.qedit.tests.iossample4`, but the local scheme `IOSSample` builds `com.qedit.tests.iossample`. Choose the matching app or switch schemes before publishing."
        );
    }

    #[test]
    fn rejects_testflight_local_project_when_scheme_bundle_id_is_missing() {
        let error = validate_local_project_bundle_id_match(
            "com.qedit.tests.iossample4",
            &AppleWorkspaceSchemeSummary {
                id: "IOSSample".to_string(),
                label: "IOSSample".to_string(),
                bundle_id: None,
                marketing_version: Some("1.0".to_string()),
                build_number: Some("1".to_string()),
                platform: Some("IOS".to_string()),
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            "Qedit could not read the selected scheme bundle ID. Check PRODUCT_BUNDLE_IDENTIFIER for the Release configuration before publishing."
        );
    }

    #[test]
    fn summarizes_workflow_error_from_last_meaningful_line() {
        let error = summarize_workflow_error(
            "Command line invocation:\n  xcodebuild ...\n\nError: publish testflight: failed to add groups: Build is not assignable.: Build is not in an internally testable state.",
        );

        assert_eq!(
            error,
            "publish testflight: failed to add groups: Build is not assignable.: Build is not in an internally testable state."
        );
    }

    #[test]
    fn detects_testflight_distribution_pending_error() {
        assert!(is_testflight_distribution_pending_error(
            "Error: publish testflight: failed to add groups: Build is not assignable.: Build is not in an internally testable state."
        ));
    }
}
