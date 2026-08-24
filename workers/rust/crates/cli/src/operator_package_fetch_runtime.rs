use crate::config::AgentConfig;
use crate::operator_package_generation::prepare_operator_package_generation;
use crate::operator_package_runtime::{
    ExternalOperatorTaskError, OperatorPackageRuntimeBinding,
    activate_fetched_operator_package_runtime, current_runtime_binding,
    operator_package_ready_for_task,
};
use kyuubiki_installer::fetch_operator_package_into;
use kyuubiki_protocol::OperatorTaskExecutionSummary;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, RwLock};

#[derive(Clone)]
struct OperatorPackageFetchRuntimeConfig {
    agent: AgentConfig,
    central_url: String,
    bearer_token: Option<String>,
    cache_store_root: Option<PathBuf>,
}

pub(crate) struct PreparedOrchestraOperatorPackage {
    pub binding: OperatorPackageRuntimeBinding,
    pub cache_status: &'static str,
}

pub(crate) fn configure_operator_package_fetch_runtime(config: &AgentConfig) {
    let configured = config.orchestrator_url.as_ref().map(|central_url| {
        let mut agent = config.clone();
        // The startup count is an admission assertion, not a hot-reload ceiling.
        agent.operator_activated_package_count = 0;
        let cache_store_root = managed_store_root(&agent).ok();
        OperatorPackageFetchRuntimeConfig {
            agent,
            central_url: central_url.clone(),
            bearer_token: config.cluster_api_token.clone(),
            cache_store_root,
        }
    });
    let mut current = fetch_runtime_config()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *current = configured;
}

pub(crate) fn prepare_orchestra_operator_package(
    summary: &OperatorTaskExecutionSummary,
) -> Result<Option<PreparedOrchestraOperatorPackage>, ExternalOperatorTaskError> {
    if summary.execution_mode.as_deref() != Some("orchestra_fetch") {
        return Ok(None);
    }
    let Some(initial_config) = configured_fetch_runtime() else {
        return Ok(None);
    };
    if operator_package_ready_for_task(summary) {
        return Ok(Some(PreparedOrchestraOperatorPackage {
            binding: current_runtime_binding(),
            cache_status: "verified_cache_hit",
        }));
    }
    let _guard = fetch_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if operator_package_ready_for_task(summary) {
        return Ok(Some(PreparedOrchestraOperatorPackage {
            binding: current_runtime_binding(),
            cache_status: "verified_cache_hit",
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
    let cache_store_root = config
        .cache_store_root
        .clone()
        .map(Ok)
        .unwrap_or_else(|| managed_store_root(&config.agent))?;
    let active_packages_root = active_packages_root(&config.agent)?;
    let generation = prepare_operator_package_generation(
        &cache_store_root,
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
    fetch_operator_package_into(
        &config.central_url,
        &summary.operator_id,
        package_version,
        generation.store_root(),
        config.bearer_token.as_deref(),
    )
    .map_err(|error| {
        fetch_error(
            "operator_package_fetch_failed",
            "fetch_package",
            format!("bound Orchestra package fetch failed: {error}"),
        )
    })?;

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
    store_active_fetch_config(next_agent, cache_store_root);
    Ok(Some(PreparedOrchestraOperatorPackage {
        binding,
        cache_status: "fetched_and_activated",
    }))
}

fn store_active_fetch_config(agent: AgentConfig, cache_store_root: PathBuf) {
    let mut config = fetch_runtime_config()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(config) = config.as_mut() {
        config.agent = agent;
        config.cache_store_root = Some(cache_store_root);
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
