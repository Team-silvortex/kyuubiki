use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use kyuubiki_protocol::RpcMethod;
use serde_json::{Value, json};

use crate::agent_lifecycle;

const HOLD_ENV: &str = "KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_FILE";
const METHOD_ENV: &str = "KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_METHOD";
const MAX_HOLD: Duration = Duration::from_secs(120);

#[derive(Clone, Default)]
struct HoldConfig {
    path: Option<PathBuf>,
    method: Option<String>,
}

pub(crate) fn configure_from_env() -> Result<(), String> {
    let path = std::env::var_os(HOLD_ENV).map(PathBuf::from);
    if path.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err(format!("{HOLD_ENV} must be an absolute path"));
    }
    let method = match std::env::var(METHOD_ENV) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err(format!("{METHOD_ENV} must be valid Unicode")),
    };
    validate_method(path.as_deref(), method.as_deref())?;
    *hold_config()
        .lock()
        .map_err(|_| "Agent hold configuration lock poisoned")? = HoldConfig { path, method };
    Ok(())
}

pub(crate) fn wait_for_release(
    execution_guard: &agent_lifecycle::ExecutionGuard,
    request_id: &str,
    job_id: Option<&str>,
    method: &str,
) {
    let config = hold_config()
        .lock()
        .ok()
        .map(|config| config.clone())
        .unwrap_or_default();
    if config
        .method
        .as_deref()
        .is_some_and(|expected| expected != method)
    {
        return;
    }
    let (Some(path), Some(job_id)) = (config.path, job_id) else {
        return;
    };
    let deadline = Instant::now() + MAX_HOLD;
    while marker_matches(&path, job_id)
        && Instant::now() < deadline
        && !execution_guard.cancellation_requested()
        && !crate::agent_state::cancellation_pending(request_id, job_id)
    {
        let _ = agent_lifecycle::mark_progress(execution_guard);
        thread::sleep(Duration::from_millis(10));
    }
}

pub(crate) fn snapshot() -> Value {
    let config = hold_config()
        .lock()
        .ok()
        .map(|config| config.clone())
        .unwrap_or_default();
    json!({
        "schema_version": "kyuubiki.agent-fault-injection/v1",
        "execution_hold_enabled": config.path.is_some(),
        "activation": "explicit_environment_only",
        "job_scope": "exact_marker_content",
        "method_scope": config.method,
        "method_configuration_env": METHOD_ENV
    })
}

fn validate_method(path: Option<&Path>, method: Option<&str>) -> Result<(), String> {
    if let Some(method) = method {
        if path.is_none() {
            return Err(format!("{METHOD_ENV} requires {HOLD_ENV}"));
        }
        if !(method.starts_with("solve_") || method == "run_operator_task_ir")
            || serde_json::from_value::<RpcMethod>(json!(method)).is_err()
        {
            return Err(format!(
                "{METHOD_ENV} must name a supported execution RPC method"
            ));
        }
    }
    Ok(())
}

fn marker_matches(path: &Path, job_id: &str) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .is_some_and(|contents| contents.trim() == job_id)
}

fn hold_config() -> &'static Mutex<HoldConfig> {
    static CONFIG: OnceLock<Mutex<HoldConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| Mutex::new(HoldConfig::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_hold_is_explicit_and_only_accepts_supported_execution_methods() {
        let path = Path::new("/test/hold");
        assert!(validate_method(None, None).is_ok());
        assert!(validate_method(Some(path), None).is_ok());
        assert!(validate_method(None, Some("solve_bar_1d")).is_err());
        for method in ["solve_thermal_plane_quad_2d", "run_operator_task_ir"] {
            assert!(validate_method(Some(path), Some(method)).is_ok());
        }
        for method in ["", "ping", "describe_agent", "solve_unknown"] {
            assert!(validate_method(Some(path), Some(method)).is_err());
        }
    }

    #[test]
    fn marker_is_scoped_to_exact_job_id() {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-agent-hold-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("wall clock")
                .as_nanos()
        ));
        std::fs::write(&path, "held-job\n").expect("write hold marker");
        assert!(marker_matches(&path, "held-job"));
        assert!(!marker_matches(&path, "other-job"));
        std::fs::remove_file(path).expect("remove hold marker");
    }
}
