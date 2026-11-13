use crate::config::AgentConfig;
use crate::operator_package_generation::{
    prepare_operator_package_generation, prepare_operator_package_generation_excluding,
};
use crate::operator_package_generation_session::OperatorPackageGenerationSession;
use crate::operator_package_retention::{
    JobReleasePlan, RetentionError, commit_job_release, package_is_retained, plan_job_release,
    register_package, reset as reset_package_retention, validate_registration,
};
use crate::operator_package_runtime::{
    ExternalOperatorRuntimeLease, ExternalOperatorTaskError, OperatorPackageRuntimeBinding,
    activate_evicted_operator_package_runtime, activate_fetched_operator_package_runtime,
    current_operator_package_generation_id, current_runtime_binding,
    prepared_operator_package_runtime_for_task,
};
use kyuubiki_protocol::OperatorTaskExecutionSummary;
use serde_json::{Value, json};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock, TryLockError};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct OperatorPackageFetchRuntimeConfig {
    agent: AgentConfig,
    central_url: String,
    bearer_token: Option<String>,
    generation_session: Option<Arc<OperatorPackageGenerationSession>>,
}

pub(crate) struct PreparedOrchestraOperatorPackage {
    pub binding: OperatorPackageRuntimeBinding,
    pub cache_status: &'static str,
    pub runtime_lease: ExternalOperatorRuntimeLease,
    generation_guard: Option<MutexGuard<'static, ()>>,
}

pub(crate) fn configure_operator_package_fetch_runtime(config: &AgentConfig) -> Result<(), String> {
    let configured = config
        .orchestrator_url
        .as_ref()
        .map(|central_url| {
            let mut agent = config.clone();
            // The startup count is an admission assertion, not a hot-reload ceiling.
            agent.operator_activated_package_count = 0;
            let generation_session = agent
                .operator_packages_root
                .as_ref()
                .map(|_| {
                    managed_store_root(&agent)
                        .map_err(|error| error.message)
                        .and_then(|root| OperatorPackageGenerationSession::open(&root))
                })
                .transpose()?;
            Ok::<_, String>(OperatorPackageFetchRuntimeConfig {
                agent,
                central_url: central_url.clone(),
                bearer_token: config.cluster_api_token.clone(),
                generation_session,
            })
        })
        .transpose()?;
    let mut current = fetch_runtime_config()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *current = configured;
    reset_package_retention();
    Ok(())
}

pub(crate) fn prepare_orchestra_operator_package(
    summary: &OperatorTaskExecutionSummary,
    job_id: Option<&str>,
) -> Result<Option<PreparedOrchestraOperatorPackage>, ExternalOperatorTaskError> {
    if summary.execution_mode.as_deref() != Some("orchestra_fetch") {
        return Ok(None);
    }
    let Some(initial_config) = configured_fetch_runtime() else {
        return Ok(None);
    };
    validate_registration(summary.cache_scope.as_deref(), job_id)
        .map_err(retention_registration_error)?;
    let serializes_lifecycle = matches!(
        summary.cache_scope.as_deref(),
        Some("none" | "job" | "session" | "agent")
    );
    let retains_generation_guard = summary.cache_scope.as_deref() == Some("none");
    if !serializes_lifecycle {
        if let Some((binding, runtime_lease)) = prepared_operator_package_runtime_for_task(summary)
        {
            register_package(summary.cache_scope.as_deref(), job_id, &summary.operator_id)
                .map_err(retention_registration_error)?;
            return Ok(Some(PreparedOrchestraOperatorPackage {
                binding,
                cache_status: "verified_cache_hit",
                runtime_lease,
                generation_guard: None,
            }));
        }
    }
    let fetch_deadline = crate::operator_package_process::fetch_deadline();
    let generation_guard = acquire_fetch_lock(fetch_lock(), fetch_deadline)?;

    if let Some((binding, runtime_lease)) = prepared_operator_package_runtime_for_task(summary) {
        register_package(summary.cache_scope.as_deref(), job_id, &summary.operator_id)
            .map_err(retention_registration_error)?;
        return Ok(Some(PreparedOrchestraOperatorPackage {
            binding,
            cache_status: "verified_cache_hit",
            runtime_lease,
            generation_guard: retains_generation_guard.then_some(generation_guard),
        }));
    }
    let config = configured_fetch_runtime().unwrap_or(initial_config);

    let package_version = summary.package_version.as_deref().ok_or_else(|| {
        fetch_error(
            "operator_package_version_missing",
            "resolve_package",
            "central operator TaskIR must declare package_version",
        )
    })?;
    let generation_session = config.generation_session.clone().ok_or_else(|| {
        fetch_error(
            "operator_package_cache_not_configured",
            "prepare_cache_generation",
            "orchestrated Agent requires a managed package cache session",
        )
    })?;
    let active_packages_root = active_packages_root(&config.agent)?;
    let generation = prepare_operator_package_generation(
        generation_session,
        &active_packages_root,
        &summary.operator_id,
    )
    .map_err(|error| {
        fetch_error(
            "operator_package_generation_failed",
            "prepare_cache_generation",
            format!("failed to prepare isolated operator package generation: {error}"),
        )
    })?;
    crate::operator_package_process::fetch_package(
        &config.central_url,
        &summary.operator_id,
        package_version,
        generation.store_root(),
        config.bearer_token.as_deref(),
        fetch_deadline,
    )
    .map_err(|error| {
        fetch_error(
            error.code,
            "fetch_package",
            format!("bound Orchestra package fetch failed: {}", error.message),
        )
    })?;

    check_fetch_progress(fetch_deadline, "fetch_package")?;

    let mut next_agent = config.agent.clone();
    next_agent.operator_packages_root = Some(
        generation
            .packages_root()
            .to_str()
            .ok_or_else(|| {
                fetch_error(
                    "operator_package_cache_path_invalid",
                    "activate_operator_registry",
                    "generated operator package cache path is not UTF-8",
                )
            })?
            .to_string(),
    );
    let binding = activate_fetched_operator_package_runtime(&next_agent, summary, generation)?;
    store_active_fetch_config(next_agent);
    let (_, runtime_lease) =
        prepared_operator_package_runtime_for_task(summary).ok_or_else(|| {
            fetch_error(
                "operator_package_activation_failed",
                "activate_operator_registry",
                "activated central package did not produce an executable host lease",
            )
        })?;
    register_package(summary.cache_scope.as_deref(), job_id, &summary.operator_id)
        .map_err(retention_registration_error)?;
    Ok(Some(PreparedOrchestraOperatorPackage {
        binding,
        cache_status: "fetched_and_activated",
        runtime_lease,
        generation_guard: retains_generation_guard.then_some(generation_guard),
    }))
}

pub(crate) fn finalize_orchestra_operator_package(
    summary: &OperatorTaskExecutionSummary,
    prepared: Option<&PreparedOrchestraOperatorPackage>,
) -> Result<Option<Value>, ExternalOperatorTaskError> {
    if summary.execution_mode.as_deref() != Some("orchestra_fetch")
        || summary.cache_scope.as_deref() != Some("none")
    {
        return Ok(None);
    }
    let prepared = prepared.ok_or_else(|| {
        fetch_error(
            "operator_package_cache_eviction_failed",
            "evict_package_cache",
            "disposable central package has no execution host lease",
        )
    })?;
    let expected_generation_id = prepared.runtime_lease.generation_id().ok_or_else(|| {
        fetch_error(
            "operator_package_cache_eviction_failed",
            "evict_package_cache",
            "disposable central package host has no owned generation",
        )
    })?;
    let expected_generation_receipt =
        prepared.runtime_lease.generation_receipt().ok_or_else(|| {
            fetch_error(
                "operator_package_cache_eviction_failed",
                "evict_package_cache",
                "disposable central package host has no generation receipt",
            )
        })?;
    if prepared.generation_guard.is_none() {
        return Err(fetch_error(
            "operator_package_cache_eviction_failed",
            "evict_package_cache",
            "disposable central package execution did not retain its generation guard",
        ));
    }
    let config = configured_fetch_runtime().ok_or_else(|| {
        fetch_error(
            "operator_package_cache_eviction_failed",
            "evict_package_cache",
            "disposable central package has no configured Agent cache",
        )
    })?;
    if package_is_retained(&summary.operator_id) {
        return Ok(Some(cache_eviction_receipt(
            summary,
            "retained_by_other_scope",
            current_runtime_binding().activated_package_count(),
            expected_generation_receipt,
        )));
    }
    if current_operator_package_generation_id().as_deref() != Some(expected_generation_id) {
        return Ok(Some(cache_eviction_receipt(
            summary,
            "superseded_generation_released",
            current_runtime_binding().activated_package_count(),
            expected_generation_receipt,
        )));
    }
    let active_packages_root = active_packages_root(&config.agent)?;
    let generation = prepare_operator_package_generation(
        config.generation_session.clone().ok_or_else(|| {
            fetch_error(
                "operator_package_cache_eviction_failed",
                "evict_package_cache",
                "disposable central package has no managed cache session",
            )
        })?,
        &active_packages_root,
        &summary.operator_id,
    )
    .map_err(|error| {
        fetch_error(
            "operator_package_cache_eviction_failed",
            "evict_package_cache",
            format!("failed to prepare disposable package eviction: {error}"),
        )
    })?;
    let generation_id = generation.generation_id().to_string();
    let session_id = generation.session_id().to_string();
    let janitor = generation.janitor_report();
    let packages_root = generation.packages_root();
    let mut next_agent = config.agent.clone();
    next_agent.operator_packages_root = Some(
        packages_root
            .to_str()
            .ok_or_else(|| {
                fetch_error(
                    "operator_package_cache_eviction_failed",
                    "evict_package_cache",
                    "evicted operator package cache path is not UTF-8",
                )
            })?
            .to_string(),
    );
    let binding =
        activate_evicted_operator_package_runtime(&next_agent, generation).map_err(|error| {
            fetch_error(
                "operator_package_cache_eviction_failed",
                "evict_package_cache",
                format!(
                    "failed to activate disposable package eviction: {}",
                    error.message
                ),
            )
        })?;
    store_active_fetch_config(next_agent);
    let generation_receipt = generation_receipt(generation_id, session_id, janitor);
    Ok(Some(cache_eviction_receipt(
        summary,
        "evicted_after_execution",
        binding.activated_package_count(),
        generation_receipt,
    )))
}

pub(crate) fn release_orchestra_operator_job(
    job_id: &str,
) -> Result<Value, ExternalOperatorTaskError> {
    // Recovery RPCs must not queue behind network fetches or disposable executions.
    let _generation_guard = try_job_release_lock(job_id, fetch_lock())?;
    let plan = plan_job_release(job_id).map_err(retention_registration_error)?;
    if !plan.was_bound {
        return Ok(job_release_receipt(
            &plan,
            "already_released",
            current_runtime_binding().activated_package_count(),
            Value::Null,
        ));
    }
    if plan.evicted_package_ids.is_empty() {
        commit_job_release(&plan);
        return Ok(job_release_receipt(
            &plan,
            "released_retained_packages",
            current_runtime_binding().activated_package_count(),
            Value::Null,
        ));
    }

    let config = configured_fetch_runtime().ok_or_else(|| {
        fetch_error(
            "operator_package_job_release_failed",
            "release_job_cache",
            "job-bound package cache has no configured Agent runtime",
        )
    })?;
    let active_packages_root = active_packages_root(&config.agent)?;
    let generation = prepare_operator_package_generation_excluding(
        config.generation_session.clone().ok_or_else(|| {
            fetch_error(
                "operator_package_job_release_failed",
                "release_job_cache",
                "job-bound package cache has no managed generation session",
            )
        })?,
        &active_packages_root,
        &plan.evicted_package_ids,
    )
    .map_err(|error| {
        fetch_error(
            "operator_package_job_release_failed",
            "release_job_cache",
            format!("failed to prepare job package release: {error}"),
        )
    })?;
    let generation_id = generation.generation_id().to_string();
    let session_id = generation.session_id().to_string();
    let janitor = generation.janitor_report();
    let mut next_agent = config.agent.clone();
    next_agent.operator_packages_root = Some(
        generation
            .packages_root()
            .to_str()
            .ok_or_else(|| {
                fetch_error(
                    "operator_package_job_release_failed",
                    "release_job_cache",
                    "job release generation path is not UTF-8",
                )
            })?
            .to_string(),
    );
    let binding =
        activate_evicted_operator_package_runtime(&next_agent, generation).map_err(|error| {
            fetch_error(
                "operator_package_job_release_failed",
                "release_job_cache",
                format!("failed to activate job package release: {}", error.message),
            )
        })?;
    store_active_fetch_config(next_agent);
    commit_job_release(&plan);
    Ok(job_release_receipt(
        &plan,
        "evicted_after_job_release",
        binding.activated_package_count(),
        generation_receipt(generation_id, session_id, janitor),
    ))
}

pub(crate) fn validate_operator_package_job_id(
    job_id: &str,
) -> Result<(), ExternalOperatorTaskError> {
    validate_registration(Some("job"), Some(job_id)).map_err(retention_registration_error)
}

fn try_job_release_lock<'a>(
    job_id: &str,
    lock: &'a Mutex<()>,
) -> Result<MutexGuard<'a, ()>, ExternalOperatorTaskError> {
    validate_operator_package_job_id(job_id)?;
    lock.try_lock().map_err(|error| match error {
        TryLockError::WouldBlock => fetch_error(
            "operator_package_job_release_busy",
            "acquire_package_lifecycle",
            "package lifecycle is busy; no cleanup performed; retry release_operator_package_job after it becomes idle, not task execution or cancellation",
        ),
        TryLockError::Poisoned(_) => fetch_error(
            "operator_package_job_release_unavailable",
            "acquire_package_lifecycle",
            "package lifecycle lock is poisoned; no cleanup performed; explicitly stop, inspect and restart the Agent before retrying cleanup",
        ),
    })
}

fn check_fetch_progress(
    deadline: Instant,
    stage: &'static str,
) -> Result<(), ExternalOperatorTaskError> {
    kyuubiki_solver::solver_control::check_cancellation()
        .map_err(|message| fetch_error("operator_package_fetch_cancelled", stage, message))?;
    if Instant::now() >= deadline {
        return Err(fetch_error(
            "operator_package_fetch_timeout",
            stage,
            "package acquisition exceeded the download budget; no retry",
        ));
    }
    Ok(())
}

fn acquire_fetch_lock(
    lock: &Mutex<()>,
    deadline: Instant,
) -> Result<MutexGuard<'_, ()>, ExternalOperatorTaskError> {
    loop {
        check_fetch_progress(deadline, "acquire_package_lifecycle")?;
        match lock.try_lock() {
            Ok(guard) => {
                check_fetch_progress(deadline, "acquire_package_lifecycle")?;
                return Ok(guard);
            }
            Err(TryLockError::WouldBlock) => std::thread::sleep(
                Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
            ),
            Err(TryLockError::Poisoned(_)) => {
                return Err(fetch_error(
                    "operator_package_lifecycle_unavailable",
                    "acquire_package_lifecycle",
                    "package lifecycle lock is poisoned; inspect and restart before fetching",
                ));
            }
        }
    }
}

pub(crate) fn job_release_policy() -> Value {
    json!({
        "on_lifecycle_contention": "reject_without_waiting_or_mutation",
        "on_poisoned_lock": "reject_without_mutation",
        "cancel_registration_independent": true,
        "automatic_cleanup_retry": false,
        "retry_method": "release_operator_package_job",
        "automatic_replay_authorized": false,
        "scope": "lifecycle_lock_admission_only"
    })
}

fn cache_eviction_receipt(
    summary: &OperatorTaskExecutionSummary,
    disposition: &str,
    remaining_activated_package_count: usize,
    generation: Value,
) -> Value {
    json!({
        "schema_version": "kyuubiki.agent-operator-cache-eviction/v1",
        "requested_cache_scope": "none",
        "resolved_cache_policy": "task_required_disposable",
        "disposition": disposition,
        "package_id": summary.operator_id,
        "package_version": summary.package_version,
        "remaining_activated_package_count": remaining_activated_package_count,
        "generation": generation
    })
}

fn job_release_receipt(
    plan: &JobReleasePlan,
    disposition: &str,
    remaining_activated_package_count: usize,
    generation: Value,
) -> Value {
    json!({
        "schema_version": "kyuubiki.agent-operator-job-cache-release/v1",
        "release_boundary": "explicit_job_terminal_rpc",
        "job_id": plan.job_id,
        "disposition": disposition,
        "released_package_ids": plan.released_package_ids,
        "evicted_package_ids": plan.evicted_package_ids,
        "retained_package_ids": plan.retained_package_ids,
        "remaining_activated_package_count": remaining_activated_package_count,
        "generation": generation
    })
}

fn generation_receipt(
    generation_id: String,
    session_id: String,
    janitor: crate::operator_package_generation_session::GenerationJanitorReport,
) -> Value {
    json!({
        "schema_version": "kyuubiki.agent-operator-generation-execution/v1",
        "session_id": session_id,
        "generation_id": generation_id,
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
}

fn retention_registration_error(error: RetentionError) -> ExternalOperatorTaskError {
    match error {
        RetentionError::MissingJobId => fetch_error(
            "operator_package_job_id_missing",
            "resolve_cache_scope",
            "cache_scope job requires a non-empty RPC job_id",
        ),
        RetentionError::InvalidJobId => fetch_error(
            "operator_package_job_id_invalid",
            "resolve_cache_scope",
            "operator package job_id must use 1-256 bytes without control characters",
        ),
    }
}

fn store_active_fetch_config(agent: AgentConfig) {
    let mut config = fetch_runtime_config()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(config) = config.as_mut() {
        config.agent = agent;
    }
}

fn configured_fetch_runtime() -> Option<OperatorPackageFetchRuntimeConfig> {
    let config = fetch_runtime_config()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    config.clone()
}

fn managed_store_root(config: &AgentConfig) -> Result<PathBuf, ExternalOperatorTaskError> {
    let packages_root = active_packages_root(config)?;
    packages_root
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            fetch_error(
                "operator_package_cache_layout_invalid",
                "resolve_package",
                "operator package cache has no managed store parent",
            )
        })
}

fn active_packages_root(config: &AgentConfig) -> Result<PathBuf, ExternalOperatorTaskError> {
    let packages_root = config.operator_packages_root.as_deref().ok_or_else(|| {
        fetch_error(
            "operator_package_cache_not_configured",
            "resolve_package",
            "orchestrated Agent requires operator_packages_root for fetch-on-demand",
        )
    })?;
    let packages_root = Path::new(packages_root);
    if packages_root
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(fetch_error(
            "operator_package_cache_layout_invalid",
            "resolve_package",
            "operator package cache must not be a symlink",
        ));
    }
    let packages_root = packages_root.canonicalize().map_err(|error| {
        fetch_error(
            "operator_package_cache_unavailable",
            "resolve_package",
            format!("failed to resolve operator package cache: {error}"),
        )
    })?;
    if packages_root.file_name() != Some(OsStr::new("packages")) {
        return Err(fetch_error(
            "operator_package_cache_layout_invalid",
            "resolve_package",
            "automatic fetch requires operator_packages_root to name the managed packages directory",
        ));
    }
    Ok(packages_root)
}

fn fetch_error(
    code: &'static str,
    stage: &'static str,
    message: impl Into<String>,
) -> ExternalOperatorTaskError {
    ExternalOperatorTaskError {
        code,
        stage,
        message: message.into(),
    }
}

fn fetch_runtime_config() -> &'static RwLock<Option<OperatorPackageFetchRuntimeConfig>> {
    static CONFIG: OnceLock<RwLock<Option<OperatorPackageFetchRuntimeConfig>>> = OnceLock::new();
    CONFIG.get_or_init(|| RwLock::new(None))
}

fn fetch_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
mod receipt_tests {
    use super::*;

    #[test]
    fn generation_receipt_separates_staging_and_session_cleanup_counts() {
        let report = crate::operator_package_generation_session::GenerationJanitorReport {
            removed_stale_session_count: 2,
            removed_initialization_staging_count: 3,
            retained_active_session_count: 4,
            retained_invalid_session_count: 5,
            pending_abandoned_generation_count: 6,
            removed_abandoned_generation_count: 7,
        };
        let receipt = generation_receipt("generation".into(), "session".into(), report);
        assert_eq!(receipt["janitor"]["removed_stale_session_count"], 2);
        assert_eq!(
            receipt["janitor"]["removed_initialization_staging_count"],
            3
        );
        assert_eq!(receipt["janitor"]["retained_active_session_count"], 4);
        assert_eq!(receipt["janitor"]["retained_invalid_session_count"], 5);
        assert_eq!(receipt["janitor"]["pending_abandoned_generation_count"], 6);
        assert_eq!(receipt["janitor"]["removed_abandoned_generation_count"], 7);
    }
}

#[cfg(test)]
mod release_lock_tests {
    use super::*;

    #[test]
    fn busy_job_release_never_waits_and_can_retry_after_owner_finishes() {
        let lock = Mutex::new(());
        let owner = lock.lock().unwrap();
        let error = try_job_release_lock("job", &lock).unwrap_err();
        assert_eq!(error.code, "operator_package_job_release_busy");
        assert_eq!(error.stage, "acquire_package_lifecycle");
        assert!(lock.try_lock().is_err());
        drop(owner);
        let release = try_job_release_lock("job", &lock).unwrap();
        assert!(
            lock.try_lock().is_err(),
            "successful cleanup must retain exclusion"
        );
        drop(release);
        assert!(lock.try_lock().is_ok());
    }

    #[test]
    fn invalid_job_release_is_rejected_even_when_lifecycle_is_busy() {
        let lock = Mutex::new(());
        let _owner = lock.lock().unwrap();
        for job in ["", "bad\njob", &"x".repeat(257)] {
            assert_eq!(
                try_job_release_lock(job, &lock).unwrap_err().code,
                "operator_package_job_id_invalid"
            );
        }
    }

    #[test]
    fn poisoned_lifecycle_does_not_authorize_package_cleanup() {
        let lock = Mutex::new(());
        let panic = std::panic::catch_unwind(|| {
            let _owner = lock.lock().unwrap();
            panic!("injected package lifecycle panic");
        });
        assert!(panic.is_err());
        let error = try_job_release_lock("job", &lock).unwrap_err();
        assert_eq!(error.code, "operator_package_job_release_unavailable");
        assert_eq!(error.stage, "acquire_package_lifecycle");
        assert_eq!(
            acquire_fetch_lock(&lock, Instant::now() + Duration::from_secs(1))
                .unwrap_err()
                .code,
            "operator_package_lifecycle_unavailable"
        );
        assert!(
            lock.is_poisoned(),
            "recovery must not silently reset the lock"
        );
    }

    #[test]
    fn waiting_fetch_observes_cancellation_without_unlocking_the_owner() {
        use kyuubiki_solver::solver_control::{SolverControl, with_solver_control};
        let lock = Mutex::new(());
        let owner = lock.lock().unwrap();
        let control = SolverControl::default();
        let result: Result<(), String> = with_solver_control(&control, || {
            control.request_cancel();
            let error =
                acquire_fetch_lock(&lock, Instant::now() + Duration::from_secs(1)).unwrap_err();
            assert_eq!(error.code, "operator_package_fetch_cancelled");
            assert_eq!(error.stage, "acquire_package_lifecycle");
            assert!(lock.try_lock().is_err());
            Err(error.message)
        });
        assert!(result.is_err());
        assert!(control.was_interrupted());
        drop(owner);
        assert!(acquire_fetch_lock(&lock, Instant::now() + Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn waiting_fetch_has_a_total_budget_without_a_late_lock_acquisition() {
        let lock = Mutex::new(());
        let owner = lock.lock().unwrap();
        let error =
            acquire_fetch_lock(&lock, Instant::now() + Duration::from_millis(10)).unwrap_err();
        assert_eq!(error.code, "operator_package_fetch_timeout");
        assert!(lock.try_lock().is_err());
        drop(owner);
        assert!(lock.try_lock().is_ok());
    }
}
