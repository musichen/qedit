use std::collections::BTreeMap;

use gpui::{App, AppContext, Entity, SharedString, Window};
use service_hub::{ServiceInputDescriptor, ServiceRunState, ServiceWorkflowDescriptor};
use ui::ToggleState;
use ui_input::InputField;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceWorkflowOption {
    pub id: String,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ServiceWorkflowRunSummary {
    pub state: ServiceRunState,
    pub headline: String,
    pub detail: String,
}

#[derive(Clone)]
pub(crate) struct ServiceWorkflowUiModel {
    pub provider_id: String,
    pub target_label: SharedString,
    pub selected_target_id: Option<String>,
    pub targets: Vec<ServiceWorkflowOption>,
    pub workflow_label: SharedString,
    pub selected_workflow_id: Option<String>,
    pub workflows: Vec<ServiceWorkflowOption>,
    pub execute_label: SharedString,
    pub form: ServiceWorkflowFormState,
    pub run: Option<ServiceWorkflowRunSummary>,
    pub disabled_reason: Option<SharedString>,
}

#[derive(Clone)]
pub(crate) struct ServiceWorkflowFormState {
    pub(crate) fields: Vec<ServiceWorkflowFieldState>,
    pub(crate) pending: bool,
    pub(crate) error_message: Option<SharedString>,
}

#[derive(Clone)]
pub(crate) enum ServiceWorkflowFieldState {
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
pub(crate) enum ServiceWorkflowUiAction {
    SelectTarget {
        target_id: String,
    },
    SelectWorkflow {
        workflow_id: String,
    },
    Submit,
    PickFile {
        field_key: String,
    },
    SetToggle {
        field_key: String,
        value: ToggleState,
    },
}

impl ServiceWorkflowFormState {
    pub(crate) fn new(
        descriptor: &ServiceWorkflowDescriptor,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        Self {
            fields: create_fields(&descriptor.inputs, window, cx),
            pending: false,
            error_message: None,
        }
    }

    pub(crate) fn set_pending(&mut self, pending: bool) {
        self.pending = pending;
    }

    pub(crate) fn finish_success(&mut self) {
        self.pending = false;
        self.error_message = None;
    }

    pub(crate) fn set_error(&mut self, error: impl Into<SharedString>) {
        self.pending = false;
        self.error_message = Some(error.into());
    }

    pub(crate) fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub(crate) fn set_toggle(&mut self, key: &str, value: ToggleState) {
        for field in &mut self.fields {
            let ServiceWorkflowFieldState::Toggle {
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
            let ServiceWorkflowFieldState::Text { descriptor, input } = field else {
                continue;
            };
            if descriptor.key == key {
                input.update(cx, |input, cx| {
                    input.set_text(text, window, cx);
                });
            }
        }
    }

    pub(crate) fn build_input(&self, cx: &App) -> Result<BTreeMap<String, String>, SharedString> {
        let mut input = BTreeMap::new();

        for field in &self.fields {
            match field {
                ServiceWorkflowFieldState::Text {
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
                ServiceWorkflowFieldState::Toggle { descriptor, value } => {
                    input.insert(descriptor.key.clone(), value.selected().to_string());
                }
            }
        }

        Ok(input)
    }
}

fn create_fields(
    descriptors: &[ServiceInputDescriptor],
    window: &mut Window,
    cx: &mut App,
) -> Vec<ServiceWorkflowFieldState> {
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
                ServiceWorkflowFieldState::Text {
                    descriptor: descriptor.clone(),
                    input,
                }
            }
            service_hub::ServiceInputKind::Toggle => ServiceWorkflowFieldState::Toggle {
                descriptor: descriptor.clone(),
                value: ToggleState::Unselected,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use service_hub::{ServiceInputDescriptor, ServiceInputKind};
    use ui::ToggleState;

    use super::{ServiceWorkflowFieldState, ServiceWorkflowFormState};

    fn toggle_descriptor() -> ServiceInputDescriptor {
        ServiceInputDescriptor {
            key: "submit".to_string(),
            label: "Submit".to_string(),
            kind: ServiceInputKind::Toggle,
            required: false,
            placeholder: None,
            help: None,
        }
    }

    #[test]
    fn toggles_update_existing_field_values() {
        let mut form = ServiceWorkflowFormState {
            fields: vec![ServiceWorkflowFieldState::Toggle {
                descriptor: toggle_descriptor(),
                value: ToggleState::Unselected,
            }],
            pending: false,
            error_message: None,
        };

        form.set_toggle("submit", ToggleState::Selected);

        let ServiceWorkflowFieldState::Toggle { value, .. } = &form.fields[0] else {
            panic!("expected toggle field");
        };

        assert!(value.selected());
    }
}
