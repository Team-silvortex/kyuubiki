use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::agent_watchdog;

const HOLD_ENV: &str = "KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_FILE";
const MAX_HOLD: Duration = Duration::from_secs(120);

pub(crate) fn configure_from_env() -> Result<(), String> {
    let path = std::env::var_os(HOLD_ENV).map(PathBuf::from);
    if path.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err(format!("{HOLD_ENV} must be an absolute path"));
    }
    if let Ok(mut configured) = hold_path().lock() {
        *configured = path;
    }
    Ok(())
}

pub(crate) fn wait_for_release(
    execution_guard: &agent_watchdog::ExecutionGuard,
    job_id: Option<&str>,
) {
    let path = hold_path().lock().ok().and_then(|path| path.clone());
    let (Some(path), Some(job_id)) = (path, job_id) else {
        return;
    };
    let deadline = Instant::now() + MAX_HOLD;
    while marker_matches(&path, job_id) && Instant::now() < deadline {
        let _ = agent_watchdog::mark_progress(execution_guard);
        thread::sleep(Duration::from_millis(10));
    }
}

pub(crate) fn snapshot() -> Value {
    json!({
        "schema_version": "kyuubiki.agent-fault-injection/v1",
        "execution_hold_enabled": hold_path().lock().is_ok_and(|path| path.is_some()),
        "activation": "explicit_environment_only",
        "job_scope": "exact_marker_content"
    })
}

fn marker_matches(path: &Path, job_id: &str) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .is_some_and(|contents| contents.trim() == job_id)
}

fn hold_path() -> &'static Mutex<Option<PathBuf>> {
    static PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    PATH.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
mod tests {
    use super::*;

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
