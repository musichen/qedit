use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceProviderDescriptor {
    pub id: String,
    pub label: String,
    pub logo_asset_path: Option<String>,
    pub shell: ServiceShellDescriptor,
    pub auth_kind: ServiceAuthKind,
    pub auth: Option<ServiceAuthConfiguration>,
    pub targets: Vec<ServiceTargetDescriptor>,
    pub workflows: Vec<ServiceWorkflowDescriptor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceShellDescriptor {
    pub resource_kind: Option<ServiceResourceKindDescriptor>,
    pub navigation_items: Vec<ServiceNavigationItemDescriptor>,
    pub default_navigation_item_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceResourceKindDescriptor {
    pub singular_label: String,
    pub plural_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceNavigationItemDescriptor {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceTargetDescriptor {
    pub id: String,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceWorkflowKind {
    Deploy,
    Release,
    Status,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceWorkflowDescriptor {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub kind: ServiceWorkflowKind,
    pub resource_kind: Option<String>,
    pub target_ids: BTreeSet<String>,
    pub inputs: Vec<ServiceInputDescriptor>,
}

impl ServiceWorkflowDescriptor {
    pub fn supports_target(&self, target_id: Option<&str>) -> bool {
        if self.target_ids.is_empty() {
            return true;
        }

        target_id.is_some_and(|target_id| self.target_ids.contains(target_id))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceAuthKind {
    None,
    ApiKey,
    OAuth,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceAuthConfiguration {
    pub kind: ServiceAuthKind,
    pub actions: Vec<ServiceAuthActionDescriptor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceAuthAction {
    Authenticate,
    Logout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceAuthActionDescriptor {
    pub action: ServiceAuthAction,
    pub label: String,
    pub description: String,
    pub inputs: Vec<ServiceInputDescriptor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceInputDescriptor {
    pub key: String,
    pub label: String,
    pub kind: ServiceInputKind,
    pub required: bool,
    pub placeholder: Option<String>,
    pub help: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceInputKind {
    Text,
    FilePath,
    Toggle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceResourceRef {
    pub provider_id: String,
    pub kind: String,
    pub external_id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceArtifactRef {
    pub kind: ServiceArtifactKind,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceArtifactKind {
    Ipa,
    Pkg,
    AppBundle,
    Binary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceOperationRequest {
    pub provider_id: String,
    pub operation: String,
    pub resource: Option<ServiceResourceRef>,
    pub artifact: Option<ServiceArtifactRef>,
    pub input: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceWorkflowRequest {
    pub provider_id: String,
    pub workflow: String,
    pub target_id: Option<String>,
    pub resource: Option<ServiceResourceRef>,
    pub artifact: Option<ServiceArtifactRef>,
    pub input: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceAuthActionRequest {
    pub provider_id: String,
    pub action: ServiceAuthAction,
    pub input: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceCommandPlan {
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceRunDescriptor {
    pub workflow: String,
    pub target_id: Option<String>,
    pub state: ServiceRunState,
    pub headline: String,
    pub detail: String,
    pub output: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceRunState {
    Pending,
    Running,
    Warning,
    Succeeded,
    Failed,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceError {
    #[error("service provider `{0}` is not supported")]
    UnknownProvider(String),
    #[error("service operation `{0}` is not supported")]
    UnsupportedOperation(String),
    #[error("service workflow `{0}` is not supported")]
    UnsupportedWorkflow(String),
    #[error("service authentication action `{0}` is not supported")]
    UnsupportedAuthAction(String),
    #[error("service operation requires resource kind `{expected}`, got `{actual}`")]
    UnexpectedResourceKind { expected: String, actual: String },
    #[error("service workflow requires target `{expected}`, got `{actual}`")]
    UnexpectedTarget { expected: String, actual: String },
    #[error("service workflow requires target `{0}`")]
    MissingTarget(String),
    #[error("service operation requires an artifact")]
    ArtifactRequired,
    #[error("service operation requires artifact kind `{expected}`, got `{actual}`")]
    UnexpectedArtifactKind { expected: String, actual: String },
    #[error("service input `{0}` is required")]
    MissingInput(&'static str),
    #[error("{0}")]
    InvalidInput(String),
}

pub trait ServiceProvider {
    fn descriptor(&self) -> ServiceProviderDescriptor;
    fn build_auth_action(
        &self,
        request: &ServiceAuthActionRequest,
    ) -> Result<ServiceCommandPlan, ServiceError>;
    fn build_operation(
        &self,
        request: &ServiceOperationRequest,
    ) -> Result<ServiceCommandPlan, ServiceError>;
    fn build_workflow(
        &self,
        request: &ServiceWorkflowRequest,
    ) -> Result<ServiceCommandPlan, ServiceError>;
}

pub struct ServiceHub {
    providers: Vec<Box<dyn ServiceProvider + Send + Sync>>,
}

impl Default for ServiceHub {
    fn default() -> Self {
        Self {
            providers: vec![Box::new(AscServiceProvider)],
        }
    }
}

impl ServiceHub {
    pub fn providers(&self) -> Vec<ServiceProviderDescriptor> {
        self.providers
            .iter()
            .map(|provider| provider.descriptor())
            .collect()
    }

    pub fn build_operation(
        &self,
        request: &ServiceOperationRequest,
    ) -> Result<ServiceCommandPlan, ServiceError> {
        let provider = self
            .providers
            .iter()
            .find(|provider| provider.descriptor().id == request.provider_id)
            .ok_or_else(|| ServiceError::UnknownProvider(request.provider_id.clone()))?;
        provider.build_operation(request)
    }

    pub fn build_auth_action(
        &self,
        request: &ServiceAuthActionRequest,
    ) -> Result<ServiceCommandPlan, ServiceError> {
        let provider = self
            .providers
            .iter()
            .find(|provider| provider.descriptor().id == request.provider_id)
            .ok_or_else(|| ServiceError::UnknownProvider(request.provider_id.clone()))?;
        provider.build_auth_action(request)
    }

    pub fn build_workflow(
        &self,
        request: &ServiceWorkflowRequest,
    ) -> Result<ServiceCommandPlan, ServiceError> {
        let provider = self
            .providers
            .iter()
            .find(|provider| provider.descriptor().id == request.provider_id)
            .ok_or_else(|| ServiceError::UnknownProvider(request.provider_id.clone()))?;
        provider.build_workflow(request)
    }
}

pub struct AscServiceProvider;

impl ServiceProvider for AscServiceProvider {
    fn descriptor(&self) -> ServiceProviderDescriptor {
        ServiceProviderDescriptor {
            id: "app-store-connect".to_string(),
            label: "App Store Connect".to_string(),
            logo_asset_path: Some("images/asc_logo.png".to_string()),
            shell: ServiceShellDescriptor {
                resource_kind: Some(ServiceResourceKindDescriptor {
                    singular_label: "App".to_string(),
                    plural_label: "Apps".to_string(),
                }),
                navigation_items: vec![
                    ServiceNavigationItemDescriptor {
                        id: "overview".to_string(),
                        label: "Overview".to_string(),
                    },
                    ServiceNavigationItemDescriptor {
                        id: "builds".to_string(),
                        label: "Builds".to_string(),
                    },
                    ServiceNavigationItemDescriptor {
                        id: "release".to_string(),
                        label: "Release".to_string(),
                    },
                ],
                default_navigation_item_id: "overview".to_string(),
            },
            auth_kind: ServiceAuthKind::ApiKey,
            auth: Some(ServiceAuthConfiguration {
                kind: ServiceAuthKind::ApiKey,
                actions: vec![
                    ServiceAuthActionDescriptor {
                        action: ServiceAuthAction::Authenticate,
                        label: "Authenticate".to_string(),
                        description:
                            "Register an App Store Connect API key for this machine or repository."
                                .to_string(),
                        inputs: vec![
                            ServiceInputDescriptor {
                                key: "profile_name".to_string(),
                                label: "Profile Name".to_string(),
                                kind: ServiceInputKind::Text,
                                required: true,
                                placeholder: Some("Personal".to_string()),
                                help: Some(
                                    "Friendly label used for the stored App Store Connect credential."
                                        .to_string(),
                                ),
                            },
                            ServiceInputDescriptor {
                                key: "key_id".to_string(),
                                label: "Key ID".to_string(),
                                kind: ServiceInputKind::Text,
                                required: true,
                                placeholder: Some("ABC123".to_string()),
                                help: None,
                            },
                            ServiceInputDescriptor {
                                key: "issuer_id".to_string(),
                                label: "Issuer ID".to_string(),
                                kind: ServiceInputKind::Text,
                                required: true,
                                placeholder: Some("00000000-0000-0000-0000-000000000000".to_string()),
                                help: None,
                            },
                            ServiceInputDescriptor {
                                key: "private_key_path".to_string(),
                                label: "Private Key".to_string(),
                                kind: ServiceInputKind::FilePath,
                                required: true,
                                placeholder: Some("/path/to/AuthKey_ABC123.p8".to_string()),
                                help: Some("Choose the downloaded App Store Connect API key file.".to_string()),
                            },
                            ServiceInputDescriptor {
                                key: "repo_local".to_string(),
                                label: "Store In Repository".to_string(),
                                kind: ServiceInputKind::Toggle,
                                required: false,
                                placeholder: None,
                                help: Some(
                                    "Store credentials in ./.asc/config.json instead of the system keychain."
                                        .to_string(),
                                ),
                            },
                            ServiceInputDescriptor {
                                key: "validate_network".to_string(),
                                label: "Validate Network Access".to_string(),
                                kind: ServiceInputKind::Toggle,
                                required: false,
                                placeholder: None,
                                help: Some(
                                    "Run a lightweight App Store Connect request during authentication."
                                        .to_string(),
                                ),
                            },
                        ],
                    },
                    ServiceAuthActionDescriptor {
                        action: ServiceAuthAction::Logout,
                        label: "Log Out".to_string(),
                        description: "Remove stored App Store Connect credentials.".to_string(),
                        inputs: Vec::new(),
                    },
                ],
            }),
            targets: vec![
                ServiceTargetDescriptor {
                    id: "testflight".to_string(),
                    label: "TestFlight".to_string(),
                    detail: Some("Distribute a build to TestFlight beta groups.".to_string()),
                },
                ServiceTargetDescriptor {
                    id: "app_store".to_string(),
                    label: "App Store".to_string(),
                    detail: Some("Attach a build to an App Store version and submit it.".to_string()),
                },
            ],
            workflows: vec![
                ServiceWorkflowDescriptor {
                    id: "publish_testflight".to_string(),
                    label: "Publish to TestFlight".to_string(),
                    detail: "Upload or select a build and distribute it to beta groups."
                        .to_string(),
                    kind: ServiceWorkflowKind::Release,
                    resource_kind: Some("app".to_string()),
                    target_ids: BTreeSet::from(["testflight".to_string()]),
                    inputs: vec![
                        ServiceInputDescriptor {
                            key: "ipa_path".to_string(),
                            label: "IPA Path".to_string(),
                            kind: ServiceInputKind::FilePath,
                            required: false,
                            placeholder: Some("./build/Qedit.ipa".to_string()),
                            help: Some(
                                "Provide an .ipa to upload. Leave blank when distributing an existing build number."
                                    .to_string(),
                            ),
                        },
                        ServiceInputDescriptor {
                            key: "build_number".to_string(),
                            label: "Build Number".to_string(),
                            kind: ServiceInputKind::Text,
                            required: false,
                            placeholder: Some("42".to_string()),
                            help: Some(
                                "Distribute an existing build by CFBundleVersion when no IPA is provided."
                                    .to_string(),
                            ),
                        },
                        ServiceInputDescriptor {
                            key: "version".to_string(),
                            label: "Version".to_string(),
                            kind: ServiceInputKind::Text,
                            required: false,
                            placeholder: Some("1.2.3".to_string()),
                            help: Some(
                                "Optional when uploading an IPA. The CLI auto-detects it from the archive when omitted."
                                    .to_string(),
                            ),
                        },
                        ServiceInputDescriptor {
                            key: "group".to_string(),
                            label: "Beta Groups".to_string(),
                            kind: ServiceInputKind::Text,
                            required: true,
                            placeholder: Some("External Testers".to_string()),
                            help: Some(
                                "Comma-separated TestFlight group names or IDs. Required by asc publish testflight."
                                    .to_string(),
                            ),
                        },
                    ],
                },
                ServiceWorkflowDescriptor {
                    id: "publish_appstore".to_string(),
                    label: "Publish to App Store".to_string(),
                    detail: "Upload an IPA, attach it to an App Store version, and optionally submit it."
                        .to_string(),
                    kind: ServiceWorkflowKind::Release,
                    resource_kind: Some("app".to_string()),
                    target_ids: BTreeSet::from(["app_store".to_string()]),
                    inputs: vec![
                        ServiceInputDescriptor {
                            key: "ipa_path".to_string(),
                            label: "IPA Path".to_string(),
                            kind: ServiceInputKind::FilePath,
                            required: true,
                            placeholder: Some("./build/Qedit.ipa".to_string()),
                            help: Some("Path to the built .ipa archive to upload.".to_string()),
                        },
                        ServiceInputDescriptor {
                            key: "version".to_string(),
                            label: "Version".to_string(),
                            kind: ServiceInputKind::Text,
                            required: false,
                            placeholder: Some("1.2.3".to_string()),
                            help: Some(
                                "Optional. The CLI auto-detects the version from the IPA when left blank."
                                    .to_string(),
                            ),
                        },
                        ServiceInputDescriptor {
                            key: "build_number".to_string(),
                            label: "Build Number".to_string(),
                            kind: ServiceInputKind::Text,
                            required: false,
                            placeholder: Some("42".to_string()),
                            help: Some(
                                "Optional. The CLI auto-detects the build number from the IPA when left blank."
                                    .to_string(),
                            ),
                        },
                        ServiceInputDescriptor {
                            key: "submit".to_string(),
                            label: "Submit For Review".to_string(),
                            kind: ServiceInputKind::Toggle,
                            required: false,
                            placeholder: None,
                            help: Some(
                                "Attach the build only when disabled. Enable to submit the prepared version for review."
                                    .to_string(),
                            ),
                        },
                        ServiceInputDescriptor {
                            key: "confirm".to_string(),
                            label: "Confirm Submission".to_string(),
                            kind: ServiceInputKind::Toggle,
                            required: false,
                            placeholder: None,
                            help: Some(
                                "Required when Submit For Review is enabled.".to_string(),
                            ),
                        },
                    ],
                },
            ],
        }
    }

    fn build_auth_action(
        &self,
        request: &ServiceAuthActionRequest,
    ) -> Result<ServiceCommandPlan, ServiceError> {
        match request.action {
            ServiceAuthAction::Authenticate => build_asc_authenticate(request),
            ServiceAuthAction::Logout => Ok(build_asc_logout()),
        }
    }

    fn build_operation(
        &self,
        request: &ServiceOperationRequest,
    ) -> Result<ServiceCommandPlan, ServiceError> {
        match request.operation.as_str() {
            "auth_status" => Ok(build_asc_auth_status()),
            "web_auth_status" => Ok(build_asc_web_auth_status(request)),
            "web_auth_login" => build_asc_web_auth_login(request),
            "list_apps" => Ok(build_asc_list_apps(request)),
            "create_app" => build_asc_create_app(request),
            "list_builds" => build_asc_list_builds(request),
            "create_testflight_group" => build_asc_create_testflight_group(request),
            "build_pre_release_version" => build_asc_pre_release_version(request),
            "build_beta_detail" => build_asc_build_beta_detail(request),
            "build_app_store_version_link" => build_asc_build_app_store_version_link(request),
            "version_view" => build_asc_version_view(request),
            "upload_build" => build_asc_upload_build(request),
            other => Err(ServiceError::UnsupportedOperation(other.to_string())),
        }
    }

    fn build_workflow(
        &self,
        request: &ServiceWorkflowRequest,
    ) -> Result<ServiceCommandPlan, ServiceError> {
        match request.workflow.as_str() {
            "publish_testflight" => build_asc_publish_testflight(request),
            "publish_appstore" => build_asc_publish_appstore(request),
            other => Err(ServiceError::UnsupportedWorkflow(other.to_string())),
        }
    }
}

fn build_asc_auth_status() -> ServiceCommandPlan {
    ServiceCommandPlan {
        label: "Validate App Store Connect authentication".to_string(),
        command: "asc".to_string(),
        args: vec![
            "auth".to_string(),
            "status".to_string(),
            "--validate".to_string(),
            "--output".to_string(),
            "json".to_string(),
            "--pretty".to_string(),
        ],
        cwd: None,
        env: BTreeMap::new(),
    }
}

fn build_asc_web_auth_status(request: &ServiceOperationRequest) -> ServiceCommandPlan {
    let mut args = vec![
        "web".to_string(),
        "auth".to_string(),
        "status".to_string(),
        "--output".to_string(),
        "json".to_string(),
        "--pretty".to_string(),
    ];

    if let Some(apple_id) = request
        .input
        .get("apple_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--apple-id".to_string());
        args.push(apple_id.to_string());
    }

    ServiceCommandPlan {
        label: "Check App Store Connect web-session authentication".to_string(),
        command: "asc".to_string(),
        args,
        cwd: None,
        env: BTreeMap::new(),
    }
}

fn build_asc_web_auth_login(
    request: &ServiceOperationRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let apple_id = request
        .input
        .get("apple_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or(ServiceError::MissingInput("apple_id"))?;

    let mut args = vec![
        "web".to_string(),
        "auth".to_string(),
        "login".to_string(),
        "--apple-id".to_string(),
        apple_id.to_string(),
        "--output".to_string(),
        "json".to_string(),
        "--pretty".to_string(),
    ];

    if let Some(two_factor_code_command) = request
        .input
        .get("two_factor_code_command")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--two-factor-code-command".to_string());
        args.push(two_factor_code_command.to_string());
    }
    if let Some(two_factor_code) = request
        .input
        .get("two_factor_code")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--two-factor-code".to_string());
        args.push(two_factor_code.to_string());
    }

    let mut env = BTreeMap::new();
    if let Some(password) = request
        .input
        .get("password")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        env.insert("ASC_WEB_PASSWORD".to_string(), password.to_string());
    }

    Ok(ServiceCommandPlan {
        label: "Authenticate App Store Connect web session".to_string(),
        command: "asc".to_string(),
        args,
        cwd: None,
        env,
    })
}

fn build_asc_authenticate(
    request: &ServiceAuthActionRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let profile_name = request
        .input
        .get("profile_name")
        .ok_or(ServiceError::MissingInput("profile_name"))?;
    let key_id = request
        .input
        .get("key_id")
        .ok_or(ServiceError::MissingInput("key_id"))?;
    let issuer_id = request
        .input
        .get("issuer_id")
        .ok_or(ServiceError::MissingInput("issuer_id"))?;
    let private_key_path = request
        .input
        .get("private_key_path")
        .ok_or(ServiceError::MissingInput("private_key_path"))?;

    let mut args = vec![
        "auth".to_string(),
        "login".to_string(),
        "--name".to_string(),
        profile_name.clone(),
        "--key-id".to_string(),
        key_id.clone(),
        "--issuer-id".to_string(),
        issuer_id.clone(),
        "--private-key".to_string(),
        private_key_path.clone(),
    ];

    if request
        .input
        .get("repo_local")
        .is_some_and(|value| value == "true")
    {
        args.push("--bypass-keychain".to_string());
        args.push("--local".to_string());
    }

    if request
        .input
        .get("validate_network")
        .is_some_and(|value| value == "true")
    {
        args.push("--network".to_string());
    }

    Ok(ServiceCommandPlan {
        label: "Authenticate App Store Connect".to_string(),
        command: "asc".to_string(),
        args,
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn build_asc_logout() -> ServiceCommandPlan {
    ServiceCommandPlan {
        label: "Log out of App Store Connect".to_string(),
        command: "asc".to_string(),
        args: vec![
            "auth".to_string(),
            "logout".to_string(),
            "--all".to_string(),
        ],
        cwd: None,
        env: BTreeMap::new(),
    }
}

fn build_asc_list_apps(request: &ServiceOperationRequest) -> ServiceCommandPlan {
    let mut args = vec![
        "apps".to_string(),
        "list".to_string(),
        "--output".to_string(),
        "json".to_string(),
        "--pretty".to_string(),
    ];

    if let Some(limit) = request.input.get("limit") {
        args.push("--limit".to_string());
        args.push(limit.clone());
    }
    if let Some(name) = request.input.get("name") {
        args.push("--name".to_string());
        args.push(name.clone());
    }
    if let Some(bundle_id) = request.input.get("bundle_id") {
        args.push("--bundle-id".to_string());
        args.push(bundle_id.clone());
    }
    if request
        .input
        .get("paginate")
        .is_some_and(|value| value == "true")
    {
        args.push("--paginate".to_string());
    }

    ServiceCommandPlan {
        label: "List App Store Connect apps".to_string(),
        command: "asc".to_string(),
        args,
        cwd: None,
        env: BTreeMap::new(),
    }
}

fn build_asc_create_app(
    request: &ServiceOperationRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let name = request
        .input
        .get("name")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or(ServiceError::MissingInput("name"))?;
    let bundle_id = request
        .input
        .get("bundle_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or(ServiceError::MissingInput("bundle_id"))?;
    let sku = request
        .input
        .get("sku")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or(ServiceError::MissingInput("sku"))?;

    let mut args = vec![
        "web".to_string(),
        "apps".to_string(),
        "create".to_string(),
        "--name".to_string(),
        name.to_string(),
        "--bundle-id".to_string(),
        bundle_id.to_string(),
        "--sku".to_string(),
        sku.to_string(),
        "--output".to_string(),
        "json".to_string(),
        "--pretty".to_string(),
    ];

    if let Some(platform) = request
        .input
        .get("platform")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--platform".to_string());
        args.push(platform.to_string());
    }
    if let Some(primary_locale) = request
        .input
        .get("primary_locale")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--primary-locale".to_string());
        args.push(primary_locale.to_string());
    }
    if let Some(version) = request
        .input
        .get("version")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--version".to_string());
        args.push(version.to_string());
    }
    if let Some(company_name) = request
        .input
        .get("company_name")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--company-name".to_string());
        args.push(company_name.to_string());
    }
    if let Some(apple_id) = request
        .input
        .get("apple_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--apple-id".to_string());
        args.push(apple_id.to_string());
    }
    if let Some(two_factor_code_command) = request
        .input
        .get("two_factor_code_command")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--two-factor-code-command".to_string());
        args.push(two_factor_code_command.to_string());
    }
    if let Some(two_factor_code) = request
        .input
        .get("two_factor_code")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--two-factor-code".to_string());
        args.push(two_factor_code.to_string());
    }

    let mut env = BTreeMap::new();
    if let Some(password) = request
        .input
        .get("password")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        env.insert("ASC_WEB_PASSWORD".to_string(), password.to_string());
    }

    Ok(ServiceCommandPlan {
        label: format!("Create App Store Connect app {}", name),
        command: "asc".to_string(),
        args,
        cwd: None,
        env,
    })
}

fn build_asc_list_builds(
    request: &ServiceOperationRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let app = request
        .resource
        .as_ref()
        .ok_or(ServiceError::MissingInput("app"))?;
    ensure_resource_kind(app, "app")?;

    let mut args = vec![
        "builds".to_string(),
        "list".to_string(),
        "--app".to_string(),
        app.external_id.clone(),
        "--output".to_string(),
        "json".to_string(),
        "--pretty".to_string(),
    ];

    if let Some(version) = request.input.get("version") {
        args.push("--version".to_string());
        args.push(version.clone());
    }
    if let Some(build_number) = request.input.get("build_number") {
        args.push("--build-number".to_string());
        args.push(build_number.clone());
    }
    if let Some(limit) = request.input.get("limit") {
        args.push("--limit".to_string());
        args.push(limit.clone());
    }
    if let Some(next) = request.input.get("next") {
        args.push("--next".to_string());
        args.push(next.clone());
    }
    if let Some(sort) = request.input.get("sort") {
        args.push("--sort".to_string());
        args.push(sort.clone());
    }
    if let Some(processing_state) = request.input.get("processing_state") {
        args.push("--processing-state".to_string());
        args.push(processing_state.clone());
    }
    if request
        .input
        .get("paginate")
        .is_some_and(|value| value == "true")
    {
        args.push("--paginate".to_string());
    }

    Ok(ServiceCommandPlan {
        label: format!("List ASC builds for {}", app.label),
        command: "asc".to_string(),
        args,
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn build_asc_pre_release_version(
    request: &ServiceOperationRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let build = request
        .resource
        .as_ref()
        .ok_or(ServiceError::MissingInput("build"))?;
    ensure_resource_kind(build, "build")?;

    Ok(ServiceCommandPlan {
        label: format!("Fetch ASC pre-release version for build {}", build.label),
        command: "asc".to_string(),
        args: vec![
            "builds".to_string(),
            "pre-release-version".to_string(),
            "get".to_string(),
            "--build".to_string(),
            build.external_id.clone(),
            "--output".to_string(),
            "json".to_string(),
            "--pretty".to_string(),
        ],
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn build_asc_upload_build(
    request: &ServiceOperationRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let app = request
        .resource
        .as_ref()
        .ok_or(ServiceError::MissingInput("app"))?;
    ensure_resource_kind(app, "app")?;

    let artifact = request
        .artifact
        .as_ref()
        .ok_or(ServiceError::ArtifactRequired)?;
    let (flag, platform) = match artifact.kind {
        ServiceArtifactKind::Ipa => ("--ipa", None),
        ServiceArtifactKind::Pkg => ("--pkg", Some("MAC_OS")),
        ServiceArtifactKind::AppBundle => {
            return Err(ServiceError::UnexpectedArtifactKind {
                expected: "ipa or pkg".to_string(),
                actual: "app_bundle".to_string(),
            });
        }
        ServiceArtifactKind::Binary => {
            return Err(ServiceError::UnexpectedArtifactKind {
                expected: "ipa or pkg".to_string(),
                actual: "binary".to_string(),
            });
        }
    };

    let mut args = vec![
        "builds".to_string(),
        "upload".to_string(),
        "--app".to_string(),
        app.external_id.clone(),
        flag.to_string(),
        artifact.path.to_string_lossy().into_owned(),
        "--output".to_string(),
        "json".to_string(),
        "--pretty".to_string(),
    ];

    if let Some(platform) = platform {
        args.push("--platform".to_string());
        args.push(platform.to_string());
    }
    if let Some(version) = request.input.get("version") {
        args.push("--version".to_string());
        args.push(version.clone());
    }
    if let Some(build_number) = request.input.get("build_number") {
        args.push("--build-number".to_string());
        args.push(build_number.clone());
    }
    if request
        .input
        .get("wait")
        .is_some_and(|value| value == "true")
    {
        args.push("--wait".to_string());
    }

    Ok(ServiceCommandPlan {
        label: format!("Upload build to App Store Connect for {}", app.label),
        command: "asc".to_string(),
        args,
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn build_asc_create_testflight_group(
    request: &ServiceOperationRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let app = request
        .resource
        .as_ref()
        .ok_or(ServiceError::MissingInput("app"))?;
    ensure_resource_kind(app, "app")?;

    let name = request
        .input
        .get("name")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or(ServiceError::MissingInput("name"))?;

    let mut args = vec![
        "testflight".to_string(),
        "groups".to_string(),
        "create".to_string(),
        "--app".to_string(),
        app.external_id.clone(),
        "--name".to_string(),
        name.to_string(),
        "--output".to_string(),
        "json".to_string(),
        "--pretty".to_string(),
    ];

    if request
        .input
        .get("internal")
        .is_some_and(|value| value == "true")
    {
        args.push("--internal".to_string());
    }

    Ok(ServiceCommandPlan {
        label: format!("Create TestFlight group {} for {}", name, app.label),
        command: "asc".to_string(),
        args,
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn build_asc_build_beta_detail(
    request: &ServiceOperationRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let build = request
        .resource
        .as_ref()
        .ok_or(ServiceError::MissingInput("build"))?;
    ensure_resource_kind(build, "build")?;

    Ok(ServiceCommandPlan {
        label: format!("Fetch ASC beta detail for build {}", build.label),
        command: "asc".to_string(),
        args: vec![
            "builds".to_string(),
            "build-beta-detail".to_string(),
            "view".to_string(),
            "--build-id".to_string(),
            build.external_id.clone(),
            "--output".to_string(),
            "json".to_string(),
            "--pretty".to_string(),
        ],
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn build_asc_build_app_store_version_link(
    request: &ServiceOperationRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let build = request
        .resource
        .as_ref()
        .ok_or(ServiceError::MissingInput("build"))?;
    ensure_resource_kind(build, "build")?;

    Ok(ServiceCommandPlan {
        label: format!(
            "Fetch ASC App Store version linkage for build {}",
            build.label
        ),
        command: "asc".to_string(),
        args: vec![
            "builds".to_string(),
            "links".to_string(),
            "view".to_string(),
            "--build-id".to_string(),
            build.external_id.clone(),
            "--type".to_string(),
            "appStoreVersion".to_string(),
            "--output".to_string(),
            "json".to_string(),
            "--pretty".to_string(),
        ],
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn build_asc_version_view(
    request: &ServiceOperationRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let version = request
        .resource
        .as_ref()
        .ok_or(ServiceError::MissingInput("version"))?;
    ensure_resource_kind(version, "app_store_version")?;

    Ok(ServiceCommandPlan {
        label: format!("Fetch ASC App Store version {}", version.label),
        command: "asc".to_string(),
        args: vec![
            "versions".to_string(),
            "view".to_string(),
            "--version-id".to_string(),
            version.external_id.clone(),
            "--output".to_string(),
            "json".to_string(),
            "--pretty".to_string(),
        ],
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn build_asc_publish_testflight(
    request: &ServiceWorkflowRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let app = request
        .resource
        .as_ref()
        .ok_or(ServiceError::MissingInput("app"))?;
    ensure_resource_kind(app, "app")?;
    ensure_target_matches(request.target_id.as_deref(), "testflight")?;

    let group = request
        .input
        .get("group")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or(ServiceError::MissingInput("group"))?;
    let ipa_path = request
        .input
        .get("ipa_path")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let existing_build_id = request
        .input
        .get("build_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let build_number = request
        .input
        .get("build_number")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let export_options = request
        .input
        .get("export_options")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let project_path = request
        .input
        .get("project_path")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let workspace_path = request
        .input
        .get("workspace_path")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let scheme = request
        .input
        .get("scheme")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());

    if project_path.is_some() && workspace_path.is_some() {
        return Err(ServiceError::InvalidInput(
            "Publish to TestFlight accepts either a project path or a workspace path, not both."
                .to_string(),
        ));
    }

    let uses_local_build = project_path.is_some() || workspace_path.is_some();
    if uses_local_build && scheme.is_none() {
        return Err(ServiceError::InvalidInput(
            "Publish to TestFlight requires a scheme when using a workspace project.".to_string(),
        ));
    }

    if ipa_path.is_none()
        && existing_build_id.is_none()
        && build_number.is_none()
        && !uses_local_build
    {
        return Err(ServiceError::InvalidInput(
            "Publish to TestFlight requires an IPA path, an existing build, a build number, or a workspace project."
                .to_string(),
        ));
    }

    let mut args = vec![
        "publish".to_string(),
        "testflight".to_string(),
        "--app".to_string(),
        app.external_id.clone(),
        "--group".to_string(),
        group.to_string(),
        "--output".to_string(),
        "json".to_string(),
        "--pretty".to_string(),
    ];

    if let Some(ipa_path) = ipa_path {
        args.push("--ipa".to_string());
        args.push(ipa_path.to_string());
    }
    if let Some(existing_build_id) = existing_build_id {
        args.push("--build".to_string());
        args.push(existing_build_id.to_string());
    }
    if let Some(build_number) = build_number {
        args.push("--build-number".to_string());
        args.push(build_number.to_string());
    }
    if let Some(project_path) = project_path {
        args.push("--project".to_string());
        args.push(project_path.to_string());
    }
    if let Some(workspace_path) = workspace_path {
        args.push("--workspace".to_string());
        args.push(workspace_path.to_string());
    }
    if let Some(scheme) = scheme {
        args.push("--scheme".to_string());
        args.push(scheme.to_string());
    }
    if let Some(export_options) = export_options {
        args.push("--export-options".to_string());
        args.push(export_options.to_string());
    }
    if let Some(version) = request
        .input
        .get("version")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--version".to_string());
        args.push(version.to_string());
    }
    if let Some(platform) = request
        .input
        .get("platform")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--platform".to_string());
        args.push(platform.to_string());
    }
    if request
        .input
        .get("wait")
        .is_some_and(|value| value == "true")
    {
        args.push("--wait".to_string());
    }
    if request
        .input
        .get("notify")
        .is_some_and(|value| value == "true")
    {
        args.push("--notify".to_string());
    }
    if request
        .input
        .get("clean")
        .is_some_and(|value| value == "true")
    {
        args.push("--clean".to_string());
    }
    if let Some(configuration) = request
        .input
        .get("configuration")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--configuration".to_string());
        args.push(configuration.to_string());
    }

    Ok(ServiceCommandPlan {
        label: format!("Publish {} to TestFlight", app.label),
        command: "asc".to_string(),
        args,
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn build_asc_publish_appstore(
    request: &ServiceWorkflowRequest,
) -> Result<ServiceCommandPlan, ServiceError> {
    let app = request
        .resource
        .as_ref()
        .ok_or(ServiceError::MissingInput("app"))?;
    ensure_resource_kind(app, "app")?;
    ensure_target_matches(request.target_id.as_deref(), "app_store")?;

    let ipa_path = request
        .input
        .get("ipa_path")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let project_path = request
        .input
        .get("project_path")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let workspace_path = request
        .input
        .get("workspace_path")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let scheme = request
        .input
        .get("scheme")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let export_options = request
        .input
        .get("export_options")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());

    if project_path.is_some() && workspace_path.is_some() {
        return Err(ServiceError::InvalidInput(
            "Publish to the App Store accepts either a project path or a workspace path, not both."
                .to_string(),
        ));
    }

    let uses_local_build = project_path.is_some() || workspace_path.is_some();
    if uses_local_build && scheme.is_none() {
        return Err(ServiceError::InvalidInput(
            "Publish to the App Store requires a scheme when using a workspace project."
                .to_string(),
        ));
    }
    if ipa_path.is_none() && !uses_local_build {
        return Err(ServiceError::MissingInput("ipa_path"));
    }

    let mut args = vec![
        "publish".to_string(),
        "appstore".to_string(),
        "--app".to_string(),
        app.external_id.clone(),
        "--output".to_string(),
        "json".to_string(),
        "--pretty".to_string(),
    ];

    if let Some(ipa_path) = ipa_path {
        args.push("--ipa".to_string());
        args.push(ipa_path.to_string());
    }
    if let Some(project_path) = project_path {
        args.push("--project".to_string());
        args.push(project_path.to_string());
    }
    if let Some(workspace_path) = workspace_path {
        args.push("--workspace".to_string());
        args.push(workspace_path.to_string());
    }
    if let Some(scheme) = scheme {
        args.push("--scheme".to_string());
        args.push(scheme.to_string());
    }
    if let Some(export_options) = export_options {
        args.push("--export-options".to_string());
        args.push(export_options.to_string());
    }

    if let Some(version) = request
        .input
        .get("version")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--version".to_string());
        args.push(version.to_string());
    }
    if let Some(build_number) = request
        .input
        .get("build_number")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--build-number".to_string());
        args.push(build_number.to_string());
    }
    if let Some(platform) = request
        .input
        .get("platform")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--platform".to_string());
        args.push(platform.to_string());
    }
    if request
        .input
        .get("wait")
        .is_some_and(|value| value == "true")
    {
        args.push("--wait".to_string());
    }
    if request
        .input
        .get("clean")
        .is_some_and(|value| value == "true")
    {
        args.push("--clean".to_string());
    }
    if let Some(configuration) = request
        .input
        .get("configuration")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        args.push("--configuration".to_string());
        args.push(configuration.to_string());
    }

    let submit = request
        .input
        .get("submit")
        .is_some_and(|value| value == "true");
    let confirm = request
        .input
        .get("confirm")
        .is_some_and(|value| value == "true");

    if submit {
        args.push("--submit".to_string());
        if confirm {
            args.push("--confirm".to_string());
        } else {
            return Err(ServiceError::InvalidInput(
                "Confirm Submission must be enabled when Submit For Review is selected."
                    .to_string(),
            ));
        }
    }

    Ok(ServiceCommandPlan {
        label: format!("Publish {} to the App Store", app.label),
        command: "asc".to_string(),
        args,
        cwd: None,
        env: BTreeMap::new(),
    })
}

fn ensure_resource_kind(resource: &ServiceResourceRef, expected: &str) -> Result<(), ServiceError> {
    if resource.kind == expected {
        Ok(())
    } else {
        Err(ServiceError::UnexpectedResourceKind {
            expected: expected.to_string(),
            actual: resource.kind.clone(),
        })
    }
}

fn ensure_target_matches(target_id: Option<&str>, expected: &str) -> Result<(), ServiceError> {
    match target_id {
        Some(target_id) if target_id == expected => Ok(()),
        Some(target_id) => Err(ServiceError::UnexpectedTarget {
            expected: expected.to_string(),
            actual: target_id.to_string(),
        }),
        None => Err(ServiceError::MissingTarget(expected.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AscServiceProvider, ServiceArtifactKind, ServiceArtifactRef, ServiceAuthAction,
        ServiceAuthActionRequest, ServiceAuthKind, ServiceHub, ServiceOperationRequest,
        ServiceProvider, ServiceResourceRef, ServiceWorkflowKind, ServiceWorkflowRequest,
    };
    use std::{collections::BTreeMap, path::PathBuf};

    #[test]
    fn advertises_targets_and_workflows_without_binding_to_a_transport() {
        let descriptor = AscServiceProvider.descriptor();

        assert!(
            descriptor
                .targets
                .iter()
                .any(|target| target.id == "testflight")
        );
        assert!(
            descriptor
                .workflows
                .iter()
                .any(|workflow| workflow.id == "publish_appstore")
        );
        assert_eq!(descriptor.auth_kind, ServiceAuthKind::ApiKey);
    }

    #[test]
    fn builds_real_asc_auth_status_command() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "auth_status".to_string(),
                resource: None,
                artifact: None,
                input: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(plan.command, "asc");
        assert_eq!(plan.args[0], "auth");
        assert!(plan.args.contains(&"--validate".to_string()));
    }

    #[test]
    fn advertises_reusable_auth_actions_for_app_store_connect() {
        let descriptor = AscServiceProvider.descriptor();
        let auth = descriptor.auth.as_ref().unwrap();

        assert_eq!(auth.kind, ServiceAuthKind::ApiKey);
        assert!(
            auth.actions
                .iter()
                .any(|action| action.action == ServiceAuthAction::Authenticate)
        );
        assert!(
            auth.actions
                .iter()
                .any(|action| action.action == ServiceAuthAction::Logout)
        );
    }

    #[test]
    fn advertises_shell_metadata_for_app_store_connect() {
        let descriptor = AscServiceProvider.descriptor();
        let resource_kind = descriptor.shell.resource_kind.as_ref().unwrap();

        assert_eq!(resource_kind.singular_label, "App");
        assert_eq!(resource_kind.plural_label, "Apps");
        assert_eq!(descriptor.shell.default_navigation_item_id, "overview");
        assert_eq!(descriptor.shell.navigation_items.len(), 3);
        assert!(
            descriptor
                .shell
                .navigation_items
                .iter()
                .any(|item| item.id == "builds")
        );
    }

    #[test]
    fn builds_real_asc_authenticate_command() {
        let hub = ServiceHub::default();
        let plan = hub
            .build_auth_action(&ServiceAuthActionRequest {
                provider_id: "app-store-connect".to_string(),
                action: ServiceAuthAction::Authenticate,
                input: [
                    ("profile_name".to_string(), "Personal".to_string()),
                    ("key_id".to_string(), "ABC123".to_string()),
                    ("issuer_id".to_string(), "ISSUER456".to_string()),
                    (
                        "private_key_path".to_string(),
                        "/tmp/AuthKey_ABC123.p8".to_string(),
                    ),
                    ("repo_local".to_string(), "true".to_string()),
                    ("validate_network".to_string(), "true".to_string()),
                ]
                .into_iter()
                .collect(),
            })
            .unwrap();

        assert_eq!(plan.command, "asc");
        assert_eq!(plan.args[0], "auth");
        assert_eq!(plan.args[1], "login");
        assert!(plan.args.contains(&"--name".to_string()));
        assert!(plan.args.contains(&"Personal".to_string()));
        assert!(plan.args.contains(&"--local".to_string()));
        assert!(plan.args.contains(&"--bypass-keychain".to_string()));
        assert!(plan.args.contains(&"--network".to_string()));
    }

    #[test]
    fn builds_real_asc_logout_command() {
        let hub = ServiceHub::default();
        let plan = hub
            .build_auth_action(&ServiceAuthActionRequest {
                provider_id: "app-store-connect".to_string(),
                action: ServiceAuthAction::Logout,
                input: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(plan.command, "asc");
        assert_eq!(plan.args, vec!["auth", "logout", "--all"]);
    }

    #[test]
    fn builds_real_asc_list_apps_command() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "list_apps".to_string(),
                resource: None,
                artifact: None,
                input: BTreeMap::from([
                    ("limit".to_string(), "25".to_string()),
                    ("paginate".to_string(), "true".to_string()),
                ]),
            })
            .unwrap();

        assert_eq!(plan.command, "asc");
        assert_eq!(plan.args[0], "apps");
        assert!(plan.args.contains(&"--limit".to_string()));
        assert!(plan.args.contains(&"--paginate".to_string()));
    }

    #[test]
    fn builds_real_asc_web_auth_login_command_with_env_password() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "web_auth_login".to_string(),
                resource: None,
                artifact: None,
                input: BTreeMap::from([
                    ("apple_id".to_string(), "person@example.com".to_string()),
                    ("password".to_string(), "top-secret".to_string()),
                    ("two_factor_code".to_string(), "123456".to_string()),
                    (
                        "two_factor_code_command".to_string(),
                        "security find-generic-password".to_string(),
                    ),
                ]),
            })
            .unwrap();

        assert_eq!(plan.command, "asc");
        assert_eq!(plan.args[0], "web");
        assert_eq!(plan.args[1], "auth");
        assert_eq!(plan.args[2], "login");
        assert!(plan.args.contains(&"--apple-id".to_string()));
        assert!(plan.args.contains(&"--two-factor-code".to_string()));
        assert!(plan.args.contains(&"123456".to_string()));
        assert!(plan.env.contains_key("ASC_WEB_PASSWORD"));
    }

    #[test]
    fn builds_real_asc_create_app_command() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "create_app".to_string(),
                resource: None,
                artifact: None,
                input: BTreeMap::from([
                    ("name".to_string(), "IOSSample".to_string()),
                    (
                        "bundle_id".to_string(),
                        "com.qedit.tests.iossample".to_string(),
                    ),
                    ("sku".to_string(), "com.qedit.tests.iossample".to_string()),
                    ("platform".to_string(), "IOS".to_string()),
                    ("primary_locale".to_string(), "en-US".to_string()),
                ]),
            })
            .unwrap();

        assert_eq!(plan.command, "asc");
        assert_eq!(plan.args[0], "web");
        assert_eq!(plan.args[1], "apps");
        assert_eq!(plan.args[2], "create");
        assert!(plan.args.contains(&"--bundle-id".to_string()));
        assert!(plan.args.contains(&"com.qedit.tests.iossample".to_string()));
    }

    #[test]
    fn builds_real_asc_list_builds_command_supports_manual_pagination() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "list_builds".to_string(),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "app".to_string(),
                    external_id: "123456789".to_string(),
                    label: "Qedit".to_string(),
                }),
                artifact: None,
                input: BTreeMap::from([
                    ("limit".to_string(), "50".to_string()),
                    (
                        "next".to_string(),
                        "https://api.appstoreconnect.apple.com/v1/builds?cursor=AQ".to_string(),
                    ),
                    ("sort".to_string(), "-uploadedDate".to_string()),
                ]),
            })
            .unwrap();

        assert_eq!(plan.command, "asc");
        assert_eq!(plan.args[0], "builds");
        assert!(plan.args.contains(&"--limit".to_string()));
        assert!(plan.args.contains(&"50".to_string()));
        assert!(plan.args.contains(&"--next".to_string()));
        assert!(plan.args.iter().any(|arg| arg.contains("cursor=AQ")));
    }

    #[test]
    fn builds_real_asc_upload_command_from_artifact_handoff() {
        let hub = ServiceHub::default();
        let plan = hub
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "upload_build".to_string(),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "app".to_string(),
                    external_id: "123456789".to_string(),
                    label: "Qedit".to_string(),
                }),
                artifact: Some(ServiceArtifactRef {
                    kind: ServiceArtifactKind::Ipa,
                    path: PathBuf::from("/tmp/Qedit.ipa"),
                }),
                input: BTreeMap::from([
                    ("version".to_string(), "1.2.3".to_string()),
                    ("build_number".to_string(), "42".to_string()),
                    ("wait".to_string(), "true".to_string()),
                ]),
            })
            .unwrap();

        assert_eq!(plan.command, "asc");
        assert_eq!(plan.args[0], "builds");
        assert!(plan.args.contains(&"--ipa".to_string()));
        assert!(plan.args.contains(&"/tmp/Qedit.ipa".to_string()));
        assert!(plan.args.contains(&"--wait".to_string()));
    }

    #[test]
    fn builds_real_asc_create_testflight_group_command() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "create_testflight_group".to_string(),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "app".to_string(),
                    external_id: "123456789".to_string(),
                    label: "Qedit".to_string(),
                }),
                artifact: None,
                input: BTreeMap::from([
                    ("name".to_string(), "Internal Testers".to_string()),
                    ("internal".to_string(), "true".to_string()),
                ]),
            })
            .unwrap();

        assert_eq!(plan.args[0], "testflight");
        assert_eq!(plan.args[1], "groups");
        assert_eq!(plan.args[2], "create");
        assert!(plan.args.contains(&"--internal".to_string()));
    }

    #[test]
    fn rejects_app_bundle_for_asc_upload() {
        let provider = AscServiceProvider;
        let result = provider.build_operation(&ServiceOperationRequest {
            provider_id: "app-store-connect".to_string(),
            operation: "upload_build".to_string(),
            resource: Some(ServiceResourceRef {
                provider_id: "app-store-connect".to_string(),
                kind: "app".to_string(),
                external_id: "123456789".to_string(),
                label: "Qedit".to_string(),
            }),
            artifact: Some(ServiceArtifactRef {
                kind: ServiceArtifactKind::AppBundle,
                path: PathBuf::from("/tmp/Qedit.app"),
            }),
            input: BTreeMap::new(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn builds_real_asc_pre_release_version_command() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "build_pre_release_version".to_string(),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "build".to_string(),
                    external_id: "BUILD-ID".to_string(),
                    label: "42".to_string(),
                }),
                artifact: None,
                input: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(plan.args[0], "builds");
        assert_eq!(plan.args[1], "pre-release-version");
        assert!(plan.args.contains(&"--build".to_string()));
        assert!(plan.args.contains(&"BUILD-ID".to_string()));
    }

    #[test]
    fn builds_real_asc_build_beta_detail_command() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "build_beta_detail".to_string(),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "build".to_string(),
                    external_id: "BUILD-ID".to_string(),
                    label: "42".to_string(),
                }),
                artifact: None,
                input: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(plan.args[0], "builds");
        assert_eq!(plan.args[1], "build-beta-detail");
        assert_eq!(plan.args[2], "view");
        assert!(plan.args.contains(&"--build-id".to_string()));
    }

    #[test]
    fn builds_real_asc_build_app_store_version_link_command() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "build_app_store_version_link".to_string(),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "build".to_string(),
                    external_id: "BUILD-ID".to_string(),
                    label: "42".to_string(),
                }),
                artifact: None,
                input: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(plan.args[0], "builds");
        assert_eq!(plan.args[1], "links");
        assert_eq!(plan.args[2], "view");
        assert!(plan.args.contains(&"appStoreVersion".to_string()));
    }

    #[test]
    fn builds_real_asc_version_view_command() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_operation(&ServiceOperationRequest {
                provider_id: "app-store-connect".to_string(),
                operation: "version_view".to_string(),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "app_store_version".to_string(),
                    external_id: "VERSION-ID".to_string(),
                    label: "1.0".to_string(),
                }),
                artifact: None,
                input: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(plan.args[0], "versions");
        assert_eq!(plan.args[1], "view");
        assert!(plan.args.contains(&"--version-id".to_string()));
        assert!(plan.args.contains(&"VERSION-ID".to_string()));
    }

    #[test]
    fn builds_real_asc_publish_testflight_workflow() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_workflow(&ServiceWorkflowRequest {
                provider_id: "app-store-connect".to_string(),
                workflow: "publish_testflight".to_string(),
                target_id: Some("testflight".to_string()),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "app".to_string(),
                    external_id: "123456789".to_string(),
                    label: "Qedit".to_string(),
                }),
                artifact: None,
                input: BTreeMap::from([
                    ("ipa_path".to_string(), "/tmp/Qedit.ipa".to_string()),
                    ("version".to_string(), "1.2.3".to_string()),
                    ("group".to_string(), "External Testers".to_string()),
                ]),
            })
            .unwrap();

        assert_eq!(plan.args[0], "publish");
        assert_eq!(plan.args[1], "testflight");
        assert!(plan.args.contains(&"--group".to_string()));
        assert!(plan.args.contains(&"External Testers".to_string()));
    }

    #[test]
    fn builds_real_asc_publish_testflight_workflow_from_workspace_project() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_workflow(&ServiceWorkflowRequest {
                provider_id: "app-store-connect".to_string(),
                workflow: "publish_testflight".to_string(),
                target_id: Some("testflight".to_string()),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "app".to_string(),
                    external_id: "123456789".to_string(),
                    label: "Qedit".to_string(),
                }),
                artifact: None,
                input: BTreeMap::from([
                    (
                        "project_path".to_string(),
                        "/tmp/IOSSample.xcodeproj".to_string(),
                    ),
                    ("scheme".to_string(), "IOSSample".to_string()),
                    (
                        "export_options".to_string(),
                        "/tmp/ExportOptions.plist".to_string(),
                    ),
                    ("group".to_string(), "Internal Testers".to_string()),
                    ("wait".to_string(), "true".to_string()),
                    ("clean".to_string(), "true".to_string()),
                ]),
            })
            .unwrap();

        assert!(plan.args.contains(&"--project".to_string()));
        assert!(plan.args.contains(&"/tmp/IOSSample.xcodeproj".to_string()));
        assert!(plan.args.contains(&"--scheme".to_string()));
        assert!(plan.args.contains(&"IOSSample".to_string()));
        assert!(plan.args.contains(&"--export-options".to_string()));
        assert!(plan.args.contains(&"/tmp/ExportOptions.plist".to_string()));
        assert!(plan.args.contains(&"--wait".to_string()));
        assert!(plan.args.contains(&"--clean".to_string()));
    }

    #[test]
    fn builds_real_asc_publish_appstore_workflow() {
        let provider = AscServiceProvider;
        let plan = provider
            .build_workflow(&ServiceWorkflowRequest {
                provider_id: "app-store-connect".to_string(),
                workflow: "publish_appstore".to_string(),
                target_id: Some("app_store".to_string()),
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "app".to_string(),
                    external_id: "123456789".to_string(),
                    label: "Qedit".to_string(),
                }),
                artifact: None,
                input: BTreeMap::from([
                    ("ipa_path".to_string(), "/tmp/Qedit.ipa".to_string()),
                    ("version".to_string(), "1.2.3".to_string()),
                    ("submit".to_string(), "true".to_string()),
                    ("confirm".to_string(), "true".to_string()),
                ]),
            })
            .unwrap();

        assert_eq!(plan.args[0], "publish");
        assert_eq!(plan.args[1], "appstore");
        assert!(plan.args.contains(&"--submit".to_string()));
        assert!(plan.args.contains(&"--confirm".to_string()));
    }

    #[test]
    fn workflow_descriptors_report_target_support() {
        let descriptor = AscServiceProvider
            .descriptor()
            .workflows
            .into_iter()
            .find(|workflow| workflow.id == "publish_testflight")
            .unwrap();

        assert_eq!(descriptor.kind, ServiceWorkflowKind::Release);
        assert!(descriptor.supports_target(Some("testflight")));
        assert!(!descriptor.supports_target(Some("app_store")));
    }

    #[test]
    fn rejects_targeted_workflows_without_an_explicit_target() {
        let provider = AscServiceProvider;
        let error = provider
            .build_workflow(&ServiceWorkflowRequest {
                provider_id: "app-store-connect".to_string(),
                workflow: "publish_testflight".to_string(),
                target_id: None,
                resource: Some(ServiceResourceRef {
                    provider_id: "app-store-connect".to_string(),
                    kind: "app".to_string(),
                    external_id: "123456789".to_string(),
                    label: "Qedit".to_string(),
                }),
                artifact: None,
                input: BTreeMap::from([
                    ("build_number".to_string(), "42".to_string()),
                    ("group".to_string(), "External Testers".to_string()),
                ]),
            })
            .unwrap_err();

        assert_eq!(
            error,
            super::ServiceError::MissingTarget("testflight".to_string())
        );
    }
}
