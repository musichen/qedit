#![cfg(target_os = "macos")]

use anyhow::Result;
use serde::Deserialize;
use service_hub::ServiceOperationRequest;

use crate::command_runner::run_json_operation;

#[cfg(target_os = "macos")]
#[derive(Clone, Debug)]
pub(crate) struct AscAuthSummary {
    pub headline: String,
    pub detail: String,
    pub warnings: Vec<String>,
    pub healthy: bool,
    pub authenticated: bool,
}

#[derive(Deserialize)]
struct AscAuthStatusResponse {
    #[serde(rename = "storageBackend")]
    storage_backend: String,
    warnings: Option<Vec<String>>,
    credentials: Vec<AscCredential>,
    #[serde(rename = "environmentNote")]
    environment_note: Option<String>,
}

#[derive(Deserialize)]
struct AscCredential {
    name: String,
    #[serde(rename = "isDefault")]
    is_default: bool,
    validation: Option<String>,
    #[serde(rename = "validationDetail")]
    validation_detail: Option<String>,
    #[serde(rename = "validationError")]
    validation_error: Option<String>,
}

#[cfg(target_os = "macos")]
pub(crate) async fn load_auth_status() -> Result<AscAuthSummary> {
    let response: AscAuthStatusResponse = run_json_operation(ServiceOperationRequest {
        provider_id: "app-store-connect".to_string(),
        operation: "auth_status".to_string(),
        resource: None,
        artifact: None,
        input: Default::default(),
    })
    .await?;

    Ok(summarize_auth_status(response))
}

#[cfg(target_os = "macos")]
fn summarize_auth_status(response: AscAuthStatusResponse) -> AscAuthSummary {
    let mut warnings = response.warnings.unwrap_or_default();
    if let Some(note) = response
        .environment_note
        .filter(|note| !note.trim().is_empty())
    {
        warnings.push(note);
    }

    let default_credential = response
        .credentials
        .iter()
        .find(|credential| credential.is_default)
        .or_else(|| response.credentials.first());

    let Some(default_credential) = default_credential else {
        return AscAuthSummary {
            headline: "Not authenticated".to_string(),
            detail: format!(
                "No App Store Connect credentials are stored in {}.",
                response.storage_backend
            ),
            warnings,
            healthy: false,
            authenticated: false,
        };
    };

    let validation = default_credential
        .validation
        .as_deref()
        .unwrap_or("unknown");
    let healthy = validation.eq_ignore_ascii_case("works");
    let detail = default_credential
        .validation_error
        .clone()
        .or_else(|| default_credential.validation_detail.clone())
        .unwrap_or_else(|| {
            format!(
                "Default credential: {} in {}",
                default_credential.name, response.storage_backend
            )
        });

    AscAuthSummary {
        headline: if healthy {
            "Authentication validated".to_string()
        } else {
            format!("Authentication status: {validation}")
        },
        detail,
        warnings,
        healthy,
        authenticated: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{AscAuthStatusResponse, AscCredential, summarize_auth_status};

    #[test]
    fn summarizes_missing_credentials_as_not_authenticated() {
        let summary = summarize_auth_status(AscAuthStatusResponse {
            storage_backend: "System Keychain".to_string(),
            warnings: None,
            credentials: Vec::new(),
            environment_note: None,
        });

        assert!(!summary.authenticated);
        assert!(!summary.healthy);
        assert_eq!(summary.headline, "Not authenticated");
    }

    #[test]
    fn summarizes_validated_credentials_as_healthy() {
        let summary = summarize_auth_status(AscAuthStatusResponse {
            storage_backend: "System Keychain".to_string(),
            warnings: Some(vec!["Using local credential override".to_string()]),
            credentials: vec![AscCredential {
                name: "Personal".to_string(),
                is_default: true,
                validation: Some("works".to_string()),
                validation_detail: Some("Validated successfully".to_string()),
                validation_error: None,
            }],
            environment_note: Some("Network validation enabled".to_string()),
        });

        assert!(summary.authenticated);
        assert!(summary.healthy);
        assert_eq!(summary.headline, "Authentication validated");
        assert_eq!(summary.detail, "Validated successfully");
        assert_eq!(summary.warnings.len(), 2);
    }
}
