use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, TryLockError};
use std::time::{Duration, Instant};

const HOLD_ENV: &str = "KYUUBIKI_AGENT_FAULT_INJECTION_STORE_READ_HOLD_FILE";
const MAX_HOLD_MS: u64 = 10_000;
static READ_HOLD: OnceLock<PathBuf> = OnceLock::new();
pub(super) const LOCK_WAIT: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub(super) enum AccessError {
    Busy,
    Unavailable(String),
}

impl From<String> for AccessError {
    fn from(message: String) -> Self {
        Self::Unavailable(message)
    }
}

impl From<serde_json::Error> for AccessError {
    fn from(error: serde_json::Error) -> Self {
        Self::Unavailable(error.to_string())
    }
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, AccessError> {
    lock_for(mutex, LOCK_WAIT)
}

fn lock_for<T>(mutex: &Mutex<T>, budget: Duration) -> Result<MutexGuard<'_, T>, AccessError> {
    let deadline = Instant::now() + budget;
    loop {
        match mutex.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(TryLockError::Poisoned(_)) => {
                return Err(AccessError::Unavailable(
                    "execution store is unavailable".into(),
                ));
            }
            Err(TryLockError::WouldBlock) => {}
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(AccessError::Busy);
        }
        std::thread::sleep(remaining.min(Duration::from_millis(2)));
        if Instant::now() >= deadline {
            return Err(AccessError::Busy);
        }
    }
}

pub(super) fn policy() -> serde_json::Value {
    serde_json::json!({"schema_version":"kyuubiki.agent-store-access/v1",
        "configuration":"fixed_read_only", "descriptor_lock_wait_ms":0,
        "recovery_lock_wait_ms":LOCK_WAIT.as_millis(),
        "recovery_methods":["fetch_durable_agent_execution", "list_durable_agent_executions", "cancel_durable_agent_execution"],
        "deadline_scope":"mutex_acquisition_only_not_filesystem_or_reply",
        "busy_error_code":"durable_execution_busy", "busy_is_not_absence_or_terminal_evidence":true,
        "writes":"retain_existing_serialized_commit_boundary",
        "poison_policy":"unavailable_never_bypass", "automatic_retry":false,
        "automatic_replay_authorized":false})
}

pub(super) fn configure() -> Result<(), String> {
    if let Some(path) = std::env::var_os(HOLD_ENV) {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(format!("{HOLD_ENV} must be an absolute path"));
        }
        READ_HOLD
            .set(path)
            .map_err(|_| "store read hold already configured")?;
    }
    Ok(())
}

pub(super) fn fault_policy() -> serde_json::Value {
    serde_json::json!({"enabled":READ_HOLD.get().is_some(), "configuration_env":HOLD_ENV,
        "phase":"after_store_lock_before_fetch", "scope":"exact_execution_id",
        "max_hold_ms":MAX_HOLD_MS})
}

pub(super) fn hold_read(id: &str) {
    let Some(path) = READ_HOLD.get() else { return };
    let selected = || {
        let mut text = String::new();
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        options.open(path).ok().is_some_and(|file| {
            file.metadata()
                .is_ok_and(|metadata| metadata.is_file() && metadata.len() <= 128)
                && file.take(129).read_to_string(&mut text).is_ok()
                && text.trim() == id
        })
    };
    if !selected() {
        return;
    }
    eprintln!("agent store-read fault hold acquired: {id}");
    let deadline = Instant::now() + Duration::from_millis(MAX_HOLD_MS);
    while Instant::now() < deadline && selected() {
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
#[path = "tests/agent_store_access.rs"]
mod tests;
