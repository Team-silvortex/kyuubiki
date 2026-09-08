use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use kyuubiki_solver::solver_control::SolverControl;
use serde_json::{Value, json};

#[derive(Debug)]
pub(crate) struct ExecutionControl {
    request_id: String,
    generation: u64,
    job_id: Option<String>,
    pub(crate) solver: SolverControl,
}

#[derive(Default)]
struct Registry {
    active: Vec<Weak<ExecutionControl>>,
    pending_jobs: HashSet<String>,
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

pub(crate) fn begin(
    request_id: String,
    generation: u64,
    job_id: Option<String>,
) -> Result<Arc<ExecutionControl>, String> {
    let mut state = registry()
        .lock()
        .map_err(|_| "execution control registry is unavailable")?;
    state.active.retain(|entry| entry.strong_count() > 0);
    let control = Arc::new(ExecutionControl {
        request_id,
        generation,
        job_id,
        solver: SolverControl::default(),
    });
    if control
        .job_id
        .as_ref()
        .is_some_and(|job| state.pending_jobs.remove(job))
    {
        control.solver.request_cancel();
    }
    state.active.push(Arc::downgrade(&control));
    Ok(control)
}

pub(crate) fn register_cancel(job_id: String) {
    if let Ok(mut state) = registry().lock() {
        let mut found = false;
        state.active.retain(|entry| {
            let Some(active) = entry.upgrade() else {
                return false;
            };
            if active.job_id.as_deref() == Some(&job_id) {
                active.solver.request_cancel();
                found = true;
            }
            true
        });
        // Preserve the existing one-shot cancellation-before-admission contract.
        if !found {
            state.pending_jobs.insert(job_id);
        }
    }
}

#[cfg(test)]
pub(crate) fn take_cancelled(job_id: &str) -> bool {
    registry()
        .lock()
        .is_ok_and(|mut state| state.pending_jobs.remove(job_id))
}

pub(crate) fn cancel_generation(request_id: &str, generation: u64) -> bool {
    registry().lock().is_ok_and(|state| {
        let mut found = false;
        for active in state.active.iter().filter_map(Weak::upgrade) {
            if active.request_id == request_id && active.generation == generation {
                active.solver.request_cancel();
                found = true;
            }
        }
        found
    })
}

pub(crate) fn checkpoint_json(control: &SolverControl) -> Value {
    control.last_checkpoint().map_or(Value::Null, |point| {
        json!({
            "stage":point.stage.as_str(), "completed_steps":point.completed_steps,
            "resumable":false
        })
    })
}

pub(crate) fn snapshot() -> Value {
    match registry().lock() {
        Ok(state) => json!({
            "schema_version":"kyuubiki.agent-solver-control/v1",
            "available":true,
            "mode":"cooperative_safe_points",
            "scope":"same_thread_builtin_linear_kernels",
            "thread_preemption":false,
            "active":state.active.iter().filter_map(Weak::upgrade).map(|active| json!({
                "request_id":active.request_id, "generation":active.generation,
                "job_id":active.job_id,
                "cancel_requested":active.solver.cancellation_requested(),
                "interrupted":active.solver.was_interrupted(),
                "checkpoint":checkpoint_json(&active.solver)
            })).collect::<Vec<_>>()
        }),
        Err(_) => json!({"schema_version":"kyuubiki.agent-solver-control/v1","available":false}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_job_cancel_marks_all_current_executions_without_poisoning_the_next() {
        let first = begin(
            "control-job-first".into(),
            1,
            Some("control-shared-job".into()),
        )
        .unwrap();
        let second = begin(
            "control-job-second".into(),
            2,
            Some("control-shared-job".into()),
        )
        .unwrap();
        register_cancel("control-shared-job".into());
        assert!(first.solver.cancellation_requested());
        assert!(second.solver.cancellation_requested());
        let next = begin(
            "control-job-next".into(),
            3,
            Some("control-shared-job".into()),
        )
        .unwrap();
        assert!(!next.solver.cancellation_requested());
    }

    #[test]
    fn stale_watchdog_generation_cannot_cancel_a_reused_request() {
        let old = begin("control-reused".into(), 10, Some("control-old".into())).unwrap();
        let new = begin("control-reused".into(), 11, Some("control-new".into())).unwrap();
        assert!(cancel_generation("control-reused", 10));
        assert!(old.solver.cancellation_requested());
        assert!(!new.solver.cancellation_requested());
        drop(old);
        assert!(!cancel_generation("control-reused", 10));
        assert!(!new.solver.cancellation_requested());
    }

    #[test]
    fn pre_admission_job_cancel_is_consumed_once() {
        register_cancel("control-pending-job".into());
        let first = begin(
            "control-pending-first".into(),
            20,
            Some("control-pending-job".into()),
        )
        .unwrap();
        let next = begin(
            "control-pending-next".into(),
            21,
            Some("control-pending-job".into()),
        )
        .unwrap();
        assert!(first.solver.cancellation_requested());
        assert!(!next.solver.cancellation_requested());
    }
}
