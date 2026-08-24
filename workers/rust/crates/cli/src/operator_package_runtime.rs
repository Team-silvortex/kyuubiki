use crate::config::AgentConfig;
use kyuubiki_engine::{
    BuiltInOperatorRegistryKind, DynamicOperatorHostSession,
    load_external_operator_packages_with_dynamic_host,
};
use kyuubiki_protocol::{
    OperatorRunContext, OperatorRunRequest, OperatorTaskExecutionSummary,
    OperatorTaskInputEnvelope, OperatorValidationStatus,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperatorPackageRuntimeAttachment {
    pub host_id: String,
    pub packages_root: String,
    pub activated_package_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OperatorPackageRuntimeBinding {
    Detached,
    Attached(OperatorPackageRuntimeAttachment),
}

#[derive(Debug)]
pub(crate) struct ExternalOperatorTaskError {
    pub code: &'static str,
    pub stage: &'static str,
    pub message: String,
}

pub(crate) struct ExternalOperatorExecution {
    pub result: Value,
    pub package_receipt: Value,
}

struct LoadedOperatorPackage {
    package_id: String,
    package_version: String,
    sdk_api_version: String,
    runtime: String,
    validation_status: &'static str,
    entrypoint_sha256: String,
    operator_kinds: BTreeMap<String, String>,
}

struct AgentOperatorPackageHost {
    session: DynamicOperatorHostSession,
    packages: Vec<LoadedOperatorPackage>,
}

pub(crate) fn initialize_operator_package_runtime(
    config: &AgentConfig,
) -> Result<OperatorPackageRuntimeBinding, String> {
    replace_host(None)?;
    let Some(root) = config.operator_packages_root.as_deref() else {
        return Ok(OperatorPackageRuntimeBinding::Detached);
    };
    let packages_root = PathBuf::from(root)
        .canonicalize()
        .map_err(|error| format!("failed to resolve operator packages root {root}: {error}"))?;
    let session = load_external_operator_packages_with_dynamic_host(
        BuiltInOperatorRegistryKind::Extract,
        &packages_root,
    )
    .map_err(|error| format!("operator package host activation failed: {error}"))?;
    if session.report().activated_packages.is_empty() {
        return Err(format!(
            "operator packages root {} contains no loadable package",
            packages_root.display()
        ));
    }

    let packages = session
        .report()
        .activated_packages
        .iter()
        .map(|plan| {
            let operator_kinds = plan
                .manifest
                .operators
                .iter()
                .map(|operator| (operator.operator_id.clone(), operator.kind.clone()))
                .collect();
            Ok(LoadedOperatorPackage {
                package_id: plan.manifest.package_id.clone(),
                package_version: plan.manifest.package_version.clone(),
                sdk_api_version: plan.manifest.sdk_api_version.clone(),
                runtime: plan.manifest.runtime.clone(),
                validation_status: validation_status_label(plan.manifest.validation_status),
                entrypoint_sha256: sha256_file(&plan.entrypoint_path)?,
                operator_kinds,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if config.operator_activated_package_count != 0
        && config.operator_activated_package_count != packages.len()
    {
        return Err(format!(
            "operator package count mismatch: configured {}, activated {}",
            config.operator_activated_package_count,
            packages.len()
        ));
    }

    let attachment = OperatorPackageRuntimeAttachment {
        host_id: config
            .operator_package_host_id
            .clone()
            .or_else(|| config.agent_id.clone())
            .unwrap_or_else(|| "agent-local/operator-host".to_string()),
        packages_root: packages_root.display().to_string(),
        activated_package_count: packages.len(),
    };
    replace_host(Some(Arc::new(AgentOperatorPackageHost {
        session,
        packages,
    })))?;
    Ok(OperatorPackageRuntimeBinding::Attached(attachment))
}

pub(crate) fn operator_package_runtime_binding_from_config(
    config: &AgentConfig,
) -> OperatorPackageRuntimeBinding {
    let Some(packages_root) = config.operator_packages_root.clone() else {
        return OperatorPackageRuntimeBinding::Detached;
    };
    OperatorPackageRuntimeBinding::Attached(OperatorPackageRuntimeAttachment {
        host_id: config
            .operator_package_host_id
            .clone()
            .or_else(|| config.agent_id.clone())
            .unwrap_or_else(|| "agent-local/operator-host".to_string()),
        packages_root,
        activated_package_count: config.operator_activated_package_count,
    })
}

pub(crate) fn store_operator_package_runtime_binding(binding: OperatorPackageRuntimeBinding) {
    if let Ok(mut current) = runtime_binding().lock() {
        *current = binding;
    }
}

pub(crate) fn current_runtime_binding() -> OperatorPackageRuntimeBinding {
    runtime_binding()
        .lock()
        .map(|binding| binding.clone())
        .unwrap_or(OperatorPackageRuntimeBinding::Detached)
}

pub(crate) fn try_execute_external_operator_task(
    summary: &OperatorTaskExecutionSummary,
    task_ir: &Value,
    binding: &OperatorPackageRuntimeBinding,
) -> Result<Option<ExternalOperatorExecution>, ExternalOperatorTaskError> {
    if !binding.is_attached() || summary.execution_mode.as_deref() != Some("local_bundle") {
        return Ok(None);
    }
    let host = current_host().ok_or_else(|| ExternalOperatorTaskError {
        code: "operator_package_host_unavailable",
        stage: "activate_operator_registry",
        message: "operator package runtime is attached without an active dynamic host".to_string(),
    })?;
    let package = host
        .packages
        .iter()
        .find(|package| package.operator_kinds.contains_key(&summary.operator_id))
        .ok_or_else(|| ExternalOperatorTaskError {
            code: "operator_package_not_loaded",
            stage: "resolve_package",
            message: format!(
                "no activated operator package provides {}",
                summary.operator_id
            ),
        })?;
    validate_package_identity(package, summary, task_ir)?;

    let input = OperatorTaskInputEnvelope {
        payload: task_ir
            .get("input_artifact")
            .cloned()
            .unwrap_or(Value::Null),
        config: task_ir.get("config").cloned().unwrap_or(Value::Null),
    };
    let request = OperatorRunRequest {
        operator_id: summary.operator_id.clone(),
        input: serde_json::to_value(input).expect("operator task input envelope should serialize"),
        context: task_context(task_ir, summary),
    };
    let result =
        host.session
            .run_operator(request)
            .map_err(|message| ExternalOperatorTaskError {
                code: "operator_package_dispatch_failed",
                stage: "dispatch_entrypoint",
                message,
            })?;
    Ok(Some(ExternalOperatorExecution {
        result: serde_json::to_value(result)
            .expect("external operator result should serialize through protocol"),
        package_receipt: json!({
            "schema_version": "kyuubiki.agent-operator-package-execution/v1",
            "package_id": package.package_id,
            "package_version": package.package_version,
            "sdk_api_version": package.sdk_api_version,
            "runtime": package.runtime,
            "validation_status": package.validation_status,
            "operator_id": summary.operator_id,
            "operator_kind": summary.operator_kind,
            "entrypoint_sha256": package.entrypoint_sha256,
            "integrity_verified": true,
            "origin": "external_local"
        }),
    }))
}

impl OperatorPackageRuntimeBinding {
    pub(crate) fn status(&self) -> &'static str {
        match self {
            Self::Detached => "not_attached",
            Self::Attached(_) => "attached",
        }
    }

    pub(crate) fn is_attached(&self) -> bool {
        matches!(self, Self::Attached(_))
    }

    pub(crate) fn activated_package_count(&self) -> usize {
        match self {
            Self::Detached => 0,
            Self::Attached(attachment) => attachment.activated_package_count,
        }
    }
}

fn validate_package_identity(
    package: &LoadedOperatorPackage,
    summary: &OperatorTaskExecutionSummary,
    task_ir: &Value,
) -> Result<(), ExternalOperatorTaskError> {
    let expected_ref = format!("bundle://{}", package.package_id);
    if summary.package_ref.as_deref() != Some(expected_ref.as_str()) {
        return identity_error(format!(
            "operator {} requires package_ref {expected_ref}",
            summary.operator_id
        ));
    }
    if summary.package_version.as_deref() != Some(package.package_version.as_str()) {
        return identity_error(format!(
            "operator {} requires package version {}",
            summary.operator_id, package.package_version
        ));
    }
    if package
        .operator_kinds
        .get(&summary.operator_id)
        .map(String::as_str)
        != Some(summary.operator_kind.as_str())
    {
        return identity_error(format!(
            "operator {} kind does not match its activated package manifest",
            summary.operator_id
        ));
    }
    let integrity = task_ir
        .pointer("/execution_program/package_integrity")
        .and_then(Value::as_object)
        .ok_or_else(|| identity_error_value("external package task must bind package_integrity"))?;
    if integrity.get("algorithm").and_then(Value::as_str) != Some("sha256")
        || integrity.get("digest").and_then(Value::as_str)
            != Some(package.entrypoint_sha256.as_str())
    {
        return identity_error(
            "external package TaskIR digest does not match the activated entrypoint".to_string(),
        );
    }
    Ok(())
}

fn identity_error(message: String) -> Result<(), ExternalOperatorTaskError> {
    Err(identity_error_value(message))
}

fn identity_error_value(message: impl Into<String>) -> ExternalOperatorTaskError {
    ExternalOperatorTaskError {
        code: "operator_package_identity_mismatch",
        stage: "verify_package_integrity",
        message: message.into(),
    }
}

fn task_context(task_ir: &Value, summary: &OperatorTaskExecutionSummary) -> OperatorRunContext {
    OperatorRunContext {
        orchestrated: matches!(
            summary.authority_mode.as_deref(),
            Some("central_operator_library" | "single_orchestrator")
        ),
        project_id: context_string(task_ir, "project_id"),
        model_id: context_string(task_ir, "model_id"),
        workflow_run_id: context_string(task_ir, "workflow_run_id"),
    }
}

fn context_string(task_ir: &Value, field: &str) -> Option<String> {
    task_ir
        .pointer(&format!("/orchestration_context/{field}"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| {
        format!(
            "failed to hash operator entrypoint {}: {error}",
            path.display()
        )
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn validation_status_label(status: OperatorValidationStatus) -> &'static str {
    match status {
        OperatorValidationStatus::Verified => "verified",
        OperatorValidationStatus::Partial => "partial",
        OperatorValidationStatus::Unverified => "unverified",
    }
}

fn runtime_binding() -> &'static Mutex<OperatorPackageRuntimeBinding> {
    static BINDING: OnceLock<Mutex<OperatorPackageRuntimeBinding>> = OnceLock::new();
    BINDING.get_or_init(|| Mutex::new(OperatorPackageRuntimeBinding::Detached))
}

fn operator_host() -> &'static RwLock<Option<Arc<AgentOperatorPackageHost>>> {
    static HOST: OnceLock<RwLock<Option<Arc<AgentOperatorPackageHost>>>> = OnceLock::new();
    HOST.get_or_init(|| RwLock::new(None))
}

fn current_host() -> Option<Arc<AgentOperatorPackageHost>> {
    operator_host().read().ok().and_then(|host| host.clone())
}

fn replace_host(host: Option<Arc<AgentOperatorPackageHost>>) -> Result<(), String> {
    let mut current = operator_host()
        .write()
        .map_err(|_| "operator package host lock is poisoned".to_string())?;
    *current = host;
    Ok(())
}
