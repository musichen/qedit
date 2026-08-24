use gpui::{App, AppContext, Entity, SharedString, Window};
use service_hub::{
    ServiceAuthAction, ServiceAuthActionDescriptor, ServiceAuthActionRequest,
    ServiceInputDescriptor,
};
use ui::{Severity, ToggleState};
use ui_input::InputField;

#[derive(Clone, Debug)]
pub(crate) struct ServiceAuthStatusSummary {
    pub severity: Severity,
    pub headline: String,
    pub detail: String,
    pub warnings: Vec<String>,
    pub authenticated: bool,
}

#[derive(Clone)]
pub(crate) struct ServiceAuthUiModel {
    pub provider_id: String,
    pub authenticate_label: SharedString,
    pub reauthenticate_label: SharedString,
    pub logout_label: SharedString,
    pub status: ServiceAuthStatusSummary,
    pub form: ServiceAuthFormState,
}

#[derive(Clone)]
pub(crate) struct ServiceAuthFormState {
    authenticate_descriptor: Option<ServiceAuthActionDescriptor>,
    pub(crate) fields: Vec<ServiceAuthFieldState>,
    pub(crate) expanded: bool,
    pub(crate) pending: bool,
    pub(crate) error_message: Option<SharedString>,
    pub(crate) logout_available: bool,
}

#[derive(Clone)]
pub(crate) enum ServiceAuthFieldState {
    Text {
        descriptor: ServiceInputDescriptor,
        input: Entity<InputField>,
    },
    Toggle {
        descriptor: ServiceInputDescriptor,
        value: ToggleState,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum ServiceAuthUiAction {
    ShowAuthenticate,
    CancelAuthenticate,
    SubmitAuthenticate,
    Logout,
    PickFile {
        field_key: String,
    },
    SetToggle {
        field_key: String,
        value: ToggleState,
    },
}

impl ServiceAuthFormState {
    pub(crate) fn new(
        provider: &service_hub::ServiceProviderDescriptor,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let authenticate_descriptor = provider.auth.as_ref().and_then(|auth| {
            auth.actions
                .iter()
                .find(|action| action.action == ServiceAuthAction::Authenticate)
                .cloned()
        });
        let logout_available = provider.auth.as_ref().is_some_and(|auth| {
            auth.actions
                .iter()
                .any(|action| action.action == ServiceAuthAction::Logout)
        });

        Self {
            fields: authenticate_descriptor
                .as_ref()
                .map(|descriptor| create_fields(&descriptor.inputs, window, cx))
                .unwrap_or_default(),
            authenticate_descriptor,
            expanded: false,
            pending: false,
            error_message: None,
            logout_available,
        }
    }

    pub(crate) fn show(&mut self) {
        self.expanded = true;
        self.error_message = None;
    }

    pub(crate) fn finish_success(&mut self) {
        self.expanded = false;
        self.pending = false;
        self.error_message = None;
    }

    pub(crate) fn cancel(&mut self) {
        self.expanded = false;
        self.pending = false;
        self.error_message = None;
    }

    pub(crate) fn set_pending(&mut self, pending: bool) {
        self.pending = pending;
    }

    pub(crate) fn set_error(&mut self, error: impl Into<SharedString>) {
        self.error_message = Some(error.into());
    }

    pub(crate) fn set_toggle(&mut self, key: &str, value: ToggleState) {
        for field in &mut self.fields {
            let ServiceAuthFieldState::Toggle {
                descriptor,
                value: current,
            } = field
            else {
                continue;
            };
            if descriptor.key == key {
                *current = value;
            }
        }
    }

    pub(crate) fn set_text(&self, key: &str, text: &str, window: &mut Window, cx: &mut App) {
        for field in &self.fields {
            let ServiceAuthFieldState::Text { descriptor, input } = field else {
                continue;
            };
            if descriptor.key == key {
                input.update(cx, |input, cx| {
                    input.set_text(text, window, cx);
                });
            }
        }
    }

    pub(crate) fn build_authenticate_request(
        &self,
        provider_id: &str,
        cx: &App,
    ) -> Result<ServiceAuthActionRequest, SharedString> {
        let Some(descriptor) = self.authenticate_descriptor.as_ref() else {
            return Err("Authentication is not available for this provider".into());
        };

        let mut input = std::collections::BTreeMap::new();
        for field in &self.fields {
            match field {
                ServiceAuthFieldState::Text {
                    descriptor,
                    input: editor,
                } => {
                    let value = editor.read(cx).text(cx).trim().to_string();
                    if descriptor.required && value.is_empty() {
                        return Err(format!("{} is required", descriptor.label).into());
                    }
                    if !value.is_empty() {
                        input.insert(descriptor.key.clone(), value);
                    }
                }
                ServiceAuthFieldState::Toggle { descriptor, value } => {
                    input.insert(descriptor.key.clone(), value.selected().to_string());
                }
            }
        }

        Ok(ServiceAuthActionRequest {
            provider_id: provider_id.to_string(),
            action: descriptor.action,
            input,
        })
    }

    pub(crate) fn build_logout_request(
        &self,
        provider_id: &str,
    ) -> Option<ServiceAuthActionRequest> {
        self.logout_available.then(|| ServiceAuthActionRequest {
            provider_id: provider_id.to_string(),
            action: ServiceAuthAction::Logout,
            input: Default::default(),
        })
    }
}

fn create_fields(
    descriptors: &[ServiceInputDescriptor],
    window: &mut Window,
    cx: &mut App,
) -> Vec<ServiceAuthFieldState> {
    descriptors
        .iter()
        .enumerate()
        .map(|(index, descriptor)| match descriptor.kind {
            service_hub::ServiceInputKind::Text | service_hub::ServiceInputKind::FilePath => {
                let input = cx.new(|cx| {
                    InputField::new(
                        window,
                        cx,
                        descriptor.placeholder.as_deref().unwrap_or_default(),
                    )
                    .label(descriptor.label.clone())
                    .tab_index(index as isize + 1)
                    .tab_stop(true)
                });
                ServiceAuthFieldState::Text {
                    descriptor: descriptor.clone(),
                    input,
                }
            }
            service_hub::ServiceInputKind::Toggle => ServiceAuthFieldState::Toggle {
                descriptor: descriptor.clone(),
                value: ToggleState::Unselected,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use service_hub::{
        ServiceAuthAction, ServiceAuthActionDescriptor, ServiceInputDescriptor, ServiceInputKind,
    };
    use ui::ToggleState;

    use super::{ServiceAuthFieldState, ServiceAuthFormState};

    fn toggle_descriptor() -> ServiceInputDescriptor {
        ServiceInputDescriptor {
            key: "repo_local".to_string(),
            label: "Store In Repository".to_string(),
            kind: ServiceInputKind::Toggle,
            required: false,
            placeholder: None,
            help: None,
        }
    }

    fn toggle_only_form(toggle_state: ToggleState) -> ServiceAuthFormState {
        ServiceAuthFormState {
            authenticate_descriptor: Some(ServiceAuthActionDescriptor {
                action: ServiceAuthAction::Authenticate,
                label: "Authenticate".to_string(),
                description: "Authenticate".to_string(),
                inputs: vec![toggle_descriptor()],
            }),
            fields: vec![ServiceAuthFieldState::Toggle {
                descriptor: toggle_descriptor(),
                value: toggle_state,
            }],
            expanded: false,
            pending: false,
            error_message: None,
            logout_available: true,
        }
    }

    #[test]
    fn set_toggle_updates_matching_toggle_field() {
        let mut form = toggle_only_form(ToggleState::Unselected);

        form.set_toggle("repo_local", ToggleState::Selected);

        match &form.fields[0] {
            ServiceAuthFieldState::Toggle { value, .. } => {
                assert_eq!(*value, ToggleState::Selected);
            }
            ServiceAuthFieldState::Text { .. } => panic!("expected toggle field"),
        }
    }

    #[test]
    fn state_transitions_clear_form_error_and_pending_state() {
        let mut form = toggle_only_form(ToggleState::Unselected);
        form.show();
        form.set_pending(true);
        form.set_error("bad token");

        form.finish_success();
        assert!(!form.expanded);
        assert!(!form.pending);
        assert!(form.error_message.is_none());

        form.show();
        form.set_pending(true);
        form.set_error("bad token");
        form.cancel();
        assert!(!form.expanded);
        assert!(!form.pending);
        assert!(form.error_message.is_none());
    }

    #[test]
    fn show_clears_existing_error() {
        let mut form = toggle_only_form(ToggleState::Unselected);
        form.set_error("bad token");

        form.show();

        assert!(form.expanded);
        assert!(form.error_message.is_none());
    }

    #[test]
    fn build_logout_request_reflects_provider_logout_support() {
        let logout_form = toggle_only_form(ToggleState::Unselected);
        let no_logout_form = ServiceAuthFormState {
            logout_available: false,
            ..toggle_only_form(ToggleState::Unselected)
        };

        assert_eq!(
            logout_form
                .build_logout_request("test-provider")
                .map(|request| request.action),
            Some(ServiceAuthAction::Logout)
        );
        assert_eq!(no_logout_form.build_logout_request("test-provider"), None);
    }
}
