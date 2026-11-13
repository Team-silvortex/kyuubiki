use crate::config::AgentConfig;
use crate::operator_package_generation::{
    OwnedOperatorPackageGeneration, PreparedOperatorPackageGeneration,
    remove_owned_operator_package_generation,
};
use crate::operator_package_process::{IsolatedOperatorHost, LoadedOperatorPackage};
use kyuubiki_protocol::{
    OperatorRunContext, OperatorRunRequest, OperatorTaskExecutionSummary, OperatorTaskInputEnvelope,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock, Weak};

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

struct AgentOperatorPackageHost {
    session: IsolatedOperatorHost,
    packages: Vec<LoadedOperatorPackage>,
    owned_generation: Option<OwnedOperatorPackageGeneration>,
}

pub(crate) fn initialize_operator_package_runtime(
    config: &AgentConfig,
    allow_recovery: bool,
) -> Result<OperatorPackageRuntimeBinding, String> {
    let loaded = if allow_recovery {
        verify_managed_startup(config).and_then(|()| load_operator_package_runtime(config))
    } else {
        load_operator_package_runtime(config)
    };
    match loaded {
        Ok((binding, host)) => {
            replace_runtime(binding.clone(), host, None);
            Ok(binding)
        }
        Err(error) if allow_recovery => {
            eprintln!(
                "{}",
                json!({"event":"operator_package_startup_failed", "code":error.code,
                "detail":error.message, "recovery":"explicit_stopped_repair_and_restart"})
            );
            replace_runtime(
                OperatorPackageRuntimeBinding::Detached,
                None,
                Some(error.code),
            );
            Ok(OperatorPackageRuntimeBinding::Detached)
        }
        Err(error) => Err(error.message),
    }
}

struct PackageLoadError {
    code: &'static str,
    message: String,
}
impl From<String> for PackageLoadError {
    fn from(message: String) -> Self {
        Self {
            code: "operator_package_activation_failed",
            message,
        }
    }
}

fn verify_managed_startup(config: &AgentConfig) -> Result<(), PackageLoadError> {
    let store = config
        .operator_packages_root
        .as_deref()
        .and_then(|p| Path::new(p).parent())
        .ok_or_else(|| PackageLoadError {
            code: "operator_package_integrity_failed",
            message: "managed package store missing".into(),
        })?;
    kyuubiki_installer::managed_operator_package_status_in(store)
        .map(|_| ())
        .map_err(|message| PackageLoadError {
            code: "operator_package_integrity_failed",
            message,
        })
}

pub(crate) fn startup_activation_error() -> Option<&'static str> {
    runtime_state()
        .read()
        .map(|s| s.startup_error)
        .unwrap_or(Some("operator_package_activation_failed"))
}

pub(crate) fn activate_fetched_operator_package_runtime(
    config: &AgentConfig,
    summary: &OperatorTaskExecutionSummary,
    generation: PreparedOperatorPackageGeneration,
) -> Result<OperatorPackageRuntimeBinding, ExternalOperatorTaskError> {
    activate_operator_package_generation(config, generation, Some(summary))
}

pub(crate) fn activate_evicted_operator_package_runtime(
    config: &AgentConfig,
    generation: PreparedOperatorPackageGeneration,
) -> Result<OperatorPackageRuntimeBinding, ExternalOperatorTaskError> {
    activate_operator_package_generation(config, generation, None)
}

fn activate_operator_package_generation(
    config: &AgentConfig,
    generation: PreparedOperatorPackageGeneration,
    required_package: Option<&OperatorTaskExecutionSummary>,
) -> Result<OperatorPackageRuntimeBinding, ExternalOperatorTaskError> {
    let (binding, host) =
        load_operator_package_runtime(config).map_err(|error| ExternalOperatorTaskError {
            code: "operator_package_activation_failed",
            stage: "activate_operator_registry",
            message: error.message,
        })?;
    let mut host = host.ok_or_else(|| ExternalOperatorTaskError {
        code: "operator_package_runtime_not_attached",
        stage: "activate_operator_registry",
        message: "operator package runtime detached during fetched package activation".to_string(),
    })?;
    if let Some(summary) = required_package {
        if !host
            .packages
            .iter()
            .any(|package| package_matches_summary(package, summary))
        {
            drop(host);
            return Err(identity_error_value(
                "downloaded package does not satisfy the admitted TaskIR identity",
            ));
        }
    }
    let owned_generation = generation.commit();
    Arc::get_mut(&mut host)
        .expect("newly loaded operator package host must be uniquely owned")
        .owned_generation = Some(owned_generation);
    replace_runtime(binding.clone(), Some(host), None);
    Ok(binding)
}

fn load_operator_package_runtime(
    config: &AgentConfig,
) -> Result<
    (
        OperatorPackageRuntimeBinding,
        Option<Arc<AgentOperatorPackageHost>>,
    ),
    PackageLoadError,
> {
    let Some(root) = config.operator_packages_root.as_deref() else {
        return Ok((OperatorPackageRuntimeBinding::Detached, None));
    };
    let packages_root = PathBuf::from(root)
        .canonicalize()
        .map_err(|error| format!("failed to resolve operator packages root {root}: {error}"))?;
    let session = IsolatedOperatorHost::load(&packages_root).map_err(|error| {
        format!(
            "operator package host activation failed: {}: {}",
            error.code, error.message
        )
    })?;
    let packages = session.packages().to_vec();
    if config.operator_activated_package_count != 0
        && config.operator_activated_package_count != packages.len()
    {
        return Err(PackageLoadError {
            code: "operator_package_count_mismatch",
            message: format!(
                "operator package count mismatch: configured {}, activated {}",
                config.operator_activated_package_count,
                packages.len()
            ),
        });
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
    Ok((
        OperatorPackageRuntimeBinding::Attached(attachment),
        Some(Arc::new(AgentOperatorPackageHost {
            session,
            packages,
            owned_generation: None,
        })),
    ))
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

pub(crate) fn current_runtime_binding() -> OperatorPackageRuntimeBinding {
    runtime_state()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .binding
        .clone()
}

pub(crate) fn active_project_pin(id: &str) -> Option<kyuubiki_protocol::ProjectOperatorPin> {
    let state = runtime_state().read().ok()?;
    state
        .host
        .as_ref()?
        .packages
        .iter()
        .find(|p| p.package_id == id)
        .map(|p| p.project_pin.clone())
}

pub(crate) fn prepared_operator_package_runtime_for_task(
    summary: &OperatorTaskExecutionSummary,
) -> Option<(OperatorPackageRuntimeBinding, ExternalOperatorRuntimeLease)> {
    let current = runtime_state()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let host = current.host.clone()?;
    if !host
        .packages
        .iter()
        .any(|package| package_matches_summary(package, summary))
    {
        return None;
    }
    Some((
        current.binding.clone(),
        ExternalOperatorRuntimeLease { host: Some(host) },
    ))
}

pub(crate) fn current_operator_package_generation_id() -> Option<String> {
    let current = runtime_state()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    current
        .host
        .as_ref()
        .and_then(|host| host.owned_generation.as_ref())
        .map(|generation| generation.generation_id().to_string())
}

pub(crate) fn try_execute_external_operator_task(
    summary: &OperatorTaskExecutionSummary,
    task_ir: &Value,
    binding: &OperatorPackageRuntimeBinding,
    orchestra_cache_status: Option<&'static str>,
    prepared_host: Option<&ExternalOperatorRuntimeLease>,
) -> Result<Option<ExternalOperatorExecution>, ExternalOperatorTaskError> {
    let execution_mode = summary.execution_mode.as_deref();
    let supported_mode = execution_mode == Some("local_bundle")
        || (execution_mode == Some("orchestra_fetch") && orchestra_cache_status.is_some());
    if supported_mode {
        if let Some(code) = startup_activation_error() {
            return Err(ExternalOperatorTaskError { code, stage: "activate_operator_registry",
                message: "External package host failed startup. Inspect local Agent diagnostics, repair through Installer while stopped, then restart and verify.".into() });
        }
    }
    if !binding.is_attached() || !supported_mode {
        return Ok(None);
    }
    if let Some(host) = prepared_host {
        return execute_external_operator_task_with_host(
            summary,
            task_ir,
            orchestra_cache_status,
            host.host(),
        )
        .map(Some);
    }
    let host = current_host().ok_or_else(host_unavailable_error)?;
    execute_external_operator_task_with_host(summary, task_ir, orchestra_cache_status, host.host())
        .map(Some)
}

fn execute_external_operator_task_with_host(
    summary: &OperatorTaskExecutionSummary,
    task_ir: &Value,
    orchestra_cache_status: Option<&'static str>,
    host: &AgentOperatorPackageHost,
) -> Result<ExternalOperatorExecution, ExternalOperatorTaskError> {
    let execution_mode = summary.execution_mode.as_deref();
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
    let result = host
        .session
        .run_operator(request)
        .map_err(|error| ExternalOperatorTaskError {
            code: error.code,
            stage: "dispatch_entrypoint",
            message: error.message,
        })?;
    let cache_generation = generation_execution_receipt(host);
    Ok(ExternalOperatorExecution {
        result: serde_json::to_value(result)
            .expect("external operator result should serialize through protocol"),
        package_receipt: json!({
            "schema_version": "kyuubiki.agent-operator-package-execution/v1",
            "package_id": package.package_id,
            "package_version": package.package_version,
            "sdk_api_version": package.sdk_api_version,
            "execution_abi": package.execution_abi,
            "runtime": package.runtime,
            "validation_status": package.validation_status,
            "operator_id": summary.operator_id,
            "operator_kind": summary.operator_kind,
            "entrypoint_sha256": package.entrypoint_sha256,
            "integrity_verified": true,
            "isolation": crate::operator_package_process::snapshot(),
            "cache_status": orchestra_cache_status.unwrap_or("local_bundle"),
            "cache_generation": cache_generation,
            "origin": if execution_mode == Some("orchestra_fetch") {
                "bound_orchestra_fetch"
            } else {
                "external_local"
            }
        }),
    })
}

fn host_unavailable_error() -> ExternalOperatorTaskError {
    ExternalOperatorTaskError {
        code: "operator_package_host_unavailable",
        stage: "activate_operator_registry",
        message: "operator package runtime is attached without an active dynamic host".to_string(),
    }
}

fn generation_execution_receipt(host: &AgentOperatorPackageHost) -> Option<Value> {
    host.owned_generation.as_ref().map(|generation| {
        let janitor = generation.janitor_report();
        json!({
            "schema_version": "kyuubiki.agent-operator-generation-execution/v1",
            "session_id": generation.session_id(),
            "generation_id": generation.generation_id(),
            "retention_policy": "host_lease",
            "crash_recovery": "next_session_start",
            "janitor": {
                "removed_stale_session_count": janitor.removed_stale_session_count,
                "removed_initialization_staging_count": janitor.removed_initialization_staging_count,
                "pending_abandoned_generation_count": janitor.pending_abandoned_generation_count,
                "removed_abandoned_generation_count": janitor.removed_abandoned_generation_count,
                "retained_active_session_count": janitor.retained_active_session_count,
                "retained_invalid_session_count": janitor.retained_invalid_session_count
            }
        })
    })
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
    let expected_ref = match summary.execution_mode.as_deref() {
        Some("orchestra_fetch") => {
            if package.package_id != summary.operator_id {
                return identity_error(format!(
                    "central operator {} requires a same-id package",
                    summary.operator_id
                ));
            }
            format!("orchestra://operator-package/{}", package.package_id)
        }
        _ => format!("bundle://{}", package.package_id),
    };
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

fn package_matches_summary(
    package: &LoadedOperatorPackage,
    summary: &OperatorTaskExecutionSummary,
) -> bool {
    let expected_package_ref = match summary.execution_mode.as_deref() {
        Some("orchestra_fetch") if package.package_id == summary.operator_id => Some(format!(
            "orchestra://operator-package/{}",
            package.package_id
        )),
        Some("local_bundle") => Some(format!("bundle://{}", package.package_id)),
        _ => None,
    };
    expected_package_ref.is_some()
        && expected_package_ref.as_deref() == summary.package_ref.as_deref()
        && summary.package_version.as_deref() == Some(package.package_version.as_str())
        && package
            .operator_kinds
            .get(&summary.operator_id)
            .map(String::as_str)
            == Some(summary.operator_kind.as_str())
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

struct OperatorPackageRuntimeState {
    binding: OperatorPackageRuntimeBinding,
    host: Option<Arc<AgentOperatorPackageHost>>,
    startup_error: Option<&'static str>,
}

struct RetiredOperatorPackageGeneration {
    host: Weak<AgentOperatorPackageHost>,
    generation: OwnedOperatorPackageGeneration,
}

pub(crate) struct ExternalOperatorRuntimeLease {
    host: Option<Arc<AgentOperatorPackageHost>>,
}

impl ExternalOperatorRuntimeLease {
    fn host(&self) -> &AgentOperatorPackageHost {
        self.host
            .as_deref()
            .expect("operator package host lease must remain populated")
    }

    pub(crate) fn generation_id(&self) -> Option<&str> {
        self.host
            .as_deref()
            .and_then(|host| host.owned_generation.as_ref())
            .map(OwnedOperatorPackageGeneration::generation_id)
    }

    pub(crate) fn generation_receipt(&self) -> Option<Value> {
        self.host.as_deref().and_then(generation_execution_receipt)
    }
}

impl Drop for ExternalOperatorRuntimeLease {
    fn drop(&mut self) {
        drop(self.host.take());
        reap_retired_generations();
    }
}

fn runtime_state() -> &'static RwLock<OperatorPackageRuntimeState> {
    static STATE: OnceLock<RwLock<OperatorPackageRuntimeState>> = OnceLock::new();
    STATE.get_or_init(|| {
        RwLock::new(OperatorPackageRuntimeState {
            binding: OperatorPackageRuntimeBinding::Detached,
            host: None,
            startup_error: None,
        })
    })
}

fn retired_generations() -> &'static Mutex<Vec<RetiredOperatorPackageGeneration>> {
    static RETIRED: OnceLock<Mutex<Vec<RetiredOperatorPackageGeneration>>> = OnceLock::new();
    RETIRED.get_or_init(|| Mutex::new(Vec::new()))
}

fn current_host() -> Option<ExternalOperatorRuntimeLease> {
    runtime_state()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .host
        .clone()
        .map(|host| ExternalOperatorRuntimeLease { host: Some(host) })
}

fn replace_runtime(
    binding: OperatorPackageRuntimeBinding,
    host: Option<Arc<AgentOperatorPackageHost>>,
    startup_error: Option<&'static str>,
) {
    let previous = {
        let mut current = runtime_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        current.binding = binding;
        current.startup_error = startup_error;
        std::mem::replace(&mut current.host, host)
    };
    retire_host(previous);
}

fn retire_host(host: Option<Arc<AgentOperatorPackageHost>>) {
    if let Some(host) = host {
        if let Some(generation) = host.owned_generation.clone() {
            retired_generations()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(RetiredOperatorPackageGeneration {
                    host: Arc::downgrade(&host),
                    generation,
                });
        }
        drop(host);
    }
    reap_retired_generations();
}

fn reap_retired_generations() {
    let mut retired = retired_generations()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    reap_retired_entries(&mut retired);
}

fn reap_retired_entries(retired: &mut Vec<RetiredOperatorPackageGeneration>) {
    retired.retain(|entry| {
        entry.host.upgrade().is_some()
            || remove_owned_operator_package_generation(&entry.generation).is_err()
    });
}

#[cfg(test)]
mod retirement_tests;
