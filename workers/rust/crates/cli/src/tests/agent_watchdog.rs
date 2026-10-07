use super::*;
use std::sync::{Arc, Barrier};

#[test]
fn execution_counters_are_monotonic_and_do_not_double_count_late_failures() {
    let state = Mutex::new(WatchdogState::default());
    let failed = begin_execution_in(
        &state,
        "failed-request".to_string(),
        Some("failed-job".to_string()),
        "run_operator_task_ir".to_string(),
    )
    .expect("unique failed request should be admitted");
    let late = failed.clone();
    fail_execution_in(&state, failed, "injected", "injected failure");
    fail_execution_in(&state, late, "late", "late failure");

    let completed = begin_execution_in(
        &state,
        "completed-request".to_string(),
        Some("completed-job".to_string()),
        "solve_bar_1d".to_string(),
    )
    .expect("unique completed request should be admitted");
    complete_execution_in(&state, completed);

    let snapshot = snapshot_from(&state);
    assert_eq!(snapshot.total_started_execution_count, 2);
    assert_eq!(snapshot.total_completed_execution_count, 1);
    assert_eq!(snapshot.total_failed_execution_count, 1);
}

#[test]
fn duplicate_active_request_is_rejected_without_overwriting_the_original() {
    let state = Mutex::new(WatchdogState::default());
    let original = begin_execution_at(
        &state,
        "duplicate-request".to_string(),
        Some("original-job".to_string()),
        "solve_bar_1d".to_string(),
        100,
    )
    .expect("first request should be admitted");
    let duplicate = begin_execution_at(
        &state,
        "duplicate-request".to_string(),
        Some("replacement-job".to_string()),
        "solve_heat_bar_1d".to_string(),
        200,
    )
    .expect_err("active request id must be unique");

    assert_eq!(duplicate.reason_code, "duplicate_active_request_id");
    let snapshot = snapshot_from_at(&state, 200);
    assert_eq!(snapshot.total_started_execution_count, 1);
    assert_eq!(snapshot.active_execution_count, 1);
    assert_eq!(
        snapshot.active_executions[0].job_id.as_deref(),
        Some("original-job")
    );
    complete_execution_in(&state, original);
}

#[test]
fn late_guard_cannot_finish_a_reused_request_generation() {
    let state = Mutex::new(WatchdogState::default());
    configure_policy_in(&state, 10, 100);
    let stale = begin_execution_at(
        &state,
        "reused-request".to_string(),
        Some("stale-job".to_string()),
        "solve_bar_1d".to_string(),
        100,
    )
    .expect("stale request should be admitted");
    assert_eq!(scan_stale_executions_at(&state, 200).len(), 1);
    let current = begin_execution_at(
        &state,
        "reused-request".to_string(),
        Some("current-job".to_string()),
        "solve_heat_bar_1d".to_string(),
        201,
    )
    .expect("request id may be reused after timeout");

    complete_execution_in(&state, stale);
    let active = snapshot_from_at(&state, 202);
    assert_eq!(active.active_execution_count, 1);
    assert_eq!(
        active.active_executions[0].job_id.as_deref(),
        Some("current-job")
    );
    assert_eq!(active.total_completed_execution_count, 0);

    complete_execution_in(&state, current);
    let completed = snapshot_from_at(&state, 203);
    assert_eq!(completed.active_execution_count, 0);
    assert_eq!(completed.total_completed_execution_count, 1);
}

#[test]
fn fault_injection_releases_slot_and_preserves_reason() {
    let report = run_fault_injection_probe().expect("watchdog probe should pass");
    assert_eq!(report["failure_reason_code"], "invalid_params");
    assert_eq!(report["slot_released_after_failure"], true);
    assert_eq!(report["slot_released_after_healthy"], true);
    assert_eq!(report["new_failure_after_healthy"], false);
    assert_eq!(report["probe_cleanup_completed"], true);
}

#[test]
fn timeout_injection_releases_slot_and_deduplicates_late_failure() {
    let report = run_timeout_fault_injection_probe().expect("watchdog timeout probe should pass");
    assert_eq!(report["timeout_reason_code"], "watchdog_timeout");
    assert_eq!(report["expired_before_budget"], false);
    assert_eq!(report["slot_released_after_timeout"], true);
    assert_eq!(report["late_failure_reused_timeout"], true);
    assert_eq!(report["duplicate_failure_created"], false);
}

fn rotate_recent_failures(state: &Mutex<WatchdogState>) {
    for n in 0..RECENT_FAILURE_LIMIT {
        let guard = begin_execution_at(
            state,
            format!("churn-{n}"),
            None,
            "solve_bar_1d".into(),
            2000 + n as u128,
        )
        .unwrap();
        fail_execution_in(
            state,
            guard,
            "invalid_params",
            format!("unrelated failure {n}"),
        );
    }
}

#[test]
fn concurrent_first_failures_pin_one_cause_and_increment_the_counter_once() {
    let state = Arc::new(Mutex::new(WatchdogState::default()));
    let guard = begin_execution_in(
        &state,
        "concurrent-first-failure".into(),
        Some("shared-job".into()),
        "run_operator_task_ir".into(),
    )
    .unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let reports = std::thread::scope(|scope| {
        let callbacks: Vec<_> = (0..8)
            .map(|n| {
                let state = state.clone();
                let guard = guard.clone();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    fail_execution_in(&state, guard, &format!("failure-{n}"), format!("cause {n}"))
                })
            })
            .collect();
        callbacks
            .into_iter()
            .map(|callback| callback.join().unwrap())
            .collect::<Vec<_>>()
    });
    let original = serde_json::to_value(&reports[0]).unwrap();
    assert_eq!(original["method"], "run_operator_task_ir");
    assert_eq!(original["job_id"], "shared-job");
    for report in reports {
        assert_eq!(serde_json::to_value(report).unwrap(), original);
    }
    let snapshot = snapshot_from(&state);
    assert_eq!(snapshot.total_started_execution_count, 1);
    assert_eq!(snapshot.total_failed_execution_count, 1);
    assert_eq!(snapshot.recent_failure_count, 1);
    assert_eq!(snapshot.active_execution_count, 0);
}

#[test]
fn evicted_failure_keeps_original_cause_and_counts_once_under_concurrent_late_callbacks() {
    let state = Arc::new(Mutex::new(WatchdogState::default()));
    let old = begin_execution_at(
        &state,
        "reused-after-eviction".into(),
        Some("original-job".into()),
        "run_operator_task_ir".into(),
        1000,
    )
    .unwrap();
    let original = fail_execution_in(&state, old.clone(), "invalid_params", "original cause");
    rotate_recent_failures(&state);
    assert!(
        !snapshot_from(&state)
            .recent_failures
            .iter()
            .any(|r| r.request_id == original.request_id)
    );

    let current = begin_execution_at(
        &state,
        "reused-after-eviction".into(),
        Some("current-job".into()),
        "solve_heat_bar_1d".into(),
        3000,
    )
    .unwrap();
    assert!(current.generation() > old.generation());
    let barrier = Arc::new(Barrier::new(8));
    let reports = std::thread::scope(|scope| {
        let callbacks: Vec<_> = (0..8)
            .map(|_| {
                let state = state.clone();
                let old = old.clone();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    fail_execution_in(
                        &state,
                        old,
                        "result_delivery_failed",
                        "late transport failure",
                    )
                })
            })
            .collect();
        callbacks
            .into_iter()
            .map(|callback| callback.join().unwrap())
            .collect::<Vec<_>>()
    });
    for report in reports {
        assert_eq!(
            serde_json::to_value(report).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
    }
    let snapshot = snapshot_from(&state);
    assert_eq!(
        snapshot.total_failed_execution_count,
        1 + RECENT_FAILURE_LIMIT as u64
    );
    assert_eq!(snapshot.recent_failure_count, RECENT_FAILURE_LIMIT);
    assert!(
        !snapshot
            .recent_failures
            .iter()
            .any(|r| r.request_id == original.request_id)
    );
    assert_eq!(snapshot.active_execution_count, 1);
    assert_eq!(
        snapshot.active_executions[0].generation,
        current.generation()
    );
    assert_eq!(
        snapshot.active_executions[0].job_id.as_deref(),
        Some("current-job")
    );
    complete_execution_in(&state, current);
    assert_eq!(snapshot_from(&state).total_completed_execution_count, 1);
}

#[test]
fn evicted_watchdog_timeout_preserves_identity_timing_and_reason_without_reinserting_history() {
    let state = Mutex::new(WatchdogState::default());
    configure_policy_in(&state, 10, 100);
    let old = begin_execution_at(
        &state,
        "timed-out-before-eviction".into(),
        Some("timed-out-job".into()),
        "run_operator_task_ir".into(),
        1000,
    )
    .unwrap();
    let expired = scan_stale_executions_at(&state, 1150);
    assert_eq!(expired.len(), 1);
    let original = &expired[0];
    assert_eq!(original.elapsed_ms, 150);
    rotate_recent_failures(&state);
    let late = fail_execution_in(&state, old, "cancelled", "solver observed timeout later");
    assert_eq!(
        serde_json::to_value(late).unwrap(),
        serde_json::to_value(original).unwrap()
    );
    let snapshot = snapshot_from(&state);
    assert_eq!(
        snapshot.total_failed_execution_count,
        1 + RECENT_FAILURE_LIMIT as u64
    );
    assert_eq!(snapshot.recent_failure_count, RECENT_FAILURE_LIMIT);
    assert!(
        !snapshot
            .recent_failures
            .iter()
            .any(|r| r.request_id == original.request_id)
    );
}

#[test]
fn terminal_failure_storage_is_released_with_the_last_guard_after_history_eviction() {
    let state = Mutex::new(WatchdogState::default());
    let guard = begin_execution_in(
        &state,
        "failure-lifetime".into(),
        None,
        "solve_bar_1d".into(),
    )
    .unwrap();
    let retained = Arc::downgrade(&guard.terminal_failure);
    let last_guard = guard.clone();
    fail_execution_in(&state, guard.clone(), "solve_failed", "one final cause");
    rotate_recent_failures(&state);
    assert_eq!(
        retained.strong_count(),
        2,
        "history must not retain an execution guard"
    );
    drop(guard);
    assert!(retained.upgrade().unwrap().get().is_some());
    drop(last_guard);
    assert!(
        retained.upgrade().is_none(),
        "no unbounded terminal archive may retain the cell"
    );
}

#[test]
fn known_terminal_failure_survives_watchdog_state_poison_without_reopening_admission() {
    let state = Arc::new(Mutex::new(WatchdogState::default()));
    let guard = begin_execution_in(
        &state,
        "poisoned-late-reply".into(),
        Some("original-job".into()),
        "run_operator_task_ir".into(),
    )
    .unwrap();
    let original = fail_execution_in(
        &state,
        guard.clone(),
        "watchdog_timeout",
        "original timeout",
    );
    rotate_recent_failures(&state);
    let poisoned = state.clone();
    assert!(
        std::thread::spawn(move || {
            let _locked = poisoned.lock().unwrap();
            panic!("poison an isolated test watchdog");
        })
        .join()
        .is_err()
    );
    let late = fail_execution_in(
        &state,
        guard,
        "result_delivery_failed",
        "later transport error",
    );
    assert_eq!(
        serde_json::to_value(late).unwrap(),
        serde_json::to_value(original).unwrap()
    );
    let rejected = begin_execution_in(
        &state,
        "must-remain-rejected".into(),
        None,
        "solve_bar_1d".into(),
    )
    .unwrap_err();
    assert_eq!(rejected.reason_code, "watchdog_state_unavailable");
}
