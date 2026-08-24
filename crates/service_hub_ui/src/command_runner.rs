use anyhow::{Context as _, Result, anyhow};
use serde::Deserialize;
#[cfg(target_os = "macos")]
use service_hub::ServiceAuthActionRequest;
use service_hub::{
    ServiceCommandPlan, ServiceHub, ServiceOperationRequest, ServiceWorkflowRequest,
};
use util::command::new_command;

pub(crate) struct CommandExecution {
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl CommandExecution {
    pub(crate) fn combined_output(&self) -> String {
        let stdout = String::from_utf8_lossy(&self.stdout).trim().to_string();
        if self.stderr.is_empty() {
            stdout
        } else if stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stderr, stdout)
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) async fn run_auth_action(request: ServiceAuthActionRequest) -> Result<()> {
    let plan = ServiceHub::default()
        .build_auth_action(&request)
        .map_err(|error| anyhow!(error.to_string()))?;
    run_command_plan(plan).await.map(|_| ())
}

pub(crate) async fn run_json_operation<T>(request: ServiceOperationRequest) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let plan = ServiceHub::default()
        .build_operation(&request)
        .map_err(|error| anyhow!(error.to_string()))?;
    let output = run_command_plan(plan).await?;
    serde_json::from_slice(&output.stdout)
        .with_context(|| "Failed to parse JSON output from App Store Connect CLI")
}

pub(crate) async fn run_workflow(request: ServiceWorkflowRequest) -> Result<CommandExecution> {
    let plan = ServiceHub::default()
        .build_workflow(&request)
        .map_err(|error| anyhow!(error.to_string()))?;
    run_command_plan(plan).await
}

async fn run_command_plan(plan: ServiceCommandPlan) -> Result<CommandExecution> {
    let mut command = new_command(&plan.command);
    command.args(&plan.args);
    if let Some(cwd) = plan.cwd {
        command.current_dir(cwd);
    }
    for (key, value) in plan.env {
        command.env(key, value);
    }

    let output = command
        .output()
        .await
        .with_context(|| format!("Failed to start `{}`", plan.command))?;

    if output.status.success() {
        Ok(CommandExecution {
            stdout: output.stdout,
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(anyhow!(
            "{}",
            if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("`{}` exited unsuccessfully", plan.command)
            }
        ))
    }
}
