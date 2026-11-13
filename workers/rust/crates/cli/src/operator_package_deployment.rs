use crate::{config::AgentConfig, transport::AgentReply};
use kyuubiki_protocol::*;
use std::{path::PathBuf, sync::OnceLock};
struct Deployment {
    root: Option<PathBuf>,
    allowed: bool,
}
static CONFIG: OnceLock<Deployment> = OnceLock::new();
pub(crate) fn configure(config: &AgentConfig) -> Result<(), String> {
    let enabled = match std::env::var("KYUUBIKI_AGENT_PACKAGE_DEPLOYMENT").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("false") => false,
        Ok("true") => true,
        _ => return Err("KYUUBIKI_AGENT_PACKAGE_DEPLOYMENT must be true or false".into()),
    };
    let root = config.operator_packages_root.as_ref().and_then(|p| {
        let p = PathBuf::from(p);
        (p.file_name()?.to_str()? == "packages")
            .then(|| p.parent().map(PathBuf::from))
            .flatten()
    });
    if enabled && (root.is_none() || config.orchestrator_url.is_some()) {
        return Err("package deployment requires a standalone managed packages root; bound Orchestra agents fetch from their controller".into());
    }
    CONFIG
        .set(Deployment {
            root,
            allowed: enabled,
        })
        .map_err(|_| "package deployment already configured".into())
}
pub(crate) fn handle(request: RpcRequest) -> AgentReply {
    let result = (|| -> Result<OperatorDeploymentStatus, String> {
        let request: OperatorDeploymentRequest =
            serde_json::from_value(request.params).map_err(|_| "deployment_invalid_request")?;
        request.package.validate()?;
        let config = CONFIG.get().ok_or("deployment_not_configured")?;
        let process = crate::agent_lifecycle::snapshot().process_instance_id;
        if !matches!(request.operation, OperatorDeploymentAction::Inspect) {
            if !config.allowed {
                return Err("deployment_disabled".into());
            }
            if request.expected_process_instance_id.as_deref() != Some(process.as_str()) {
                return Err("deployment_agent_changed".into());
            }
        }
        let mut status = kyuubiki_installer::operator_deployment_in(
            config.root.as_deref().ok_or("deployment_not_configured")?,
            &request.package,
            request.operation,
        )?;
        status.active =
            crate::operator_package_runtime::active_project_pin(&request.package.package_id);
        if status.installed.as_ref() == Some(&request.package)
            && status.active.as_ref() == Some(&request.package)
        {
            status.state = "ready".into();
        }
        status.process_instance_id = Some(process);
        status.deployment_allowed = config.allowed;
        status.validate()?;
        Ok(status)
    })();
    let response = match result {
        Ok(value) => RpcResponse::success(
            request.id,
            serde_json::to_value(value).expect("deployment response"),
        ),
        Err(error) => {
            let code = if error.len() <= 64
                && error.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
            {
                error
            } else {
                "deployment_unconfirmed".into()
            };
            RpcResponse::error(
                request.id,
                code,
                "Package state is unconfirmed; inspect the target and do not overwrite or replay execution",
            )
        }
    };
    AgentReply::Stream(Vec::new(), response)
}
