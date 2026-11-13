use super::*;

fn fill(state: &Mutex<Registry>, count: usize) {
    for n in 0..count {
        register_cancel_in(state, format!("pending-{n}")).unwrap();
    }
}

#[test]
fn pending_cancellation_capacity_rejects_new_ids_without_eviction() {
    let state = Mutex::new(Registry::default());
    fill(&state, MAX_PENDING_JOB_CANCELLATIONS);
    assert!(
        register_cancel_in(&state, "excess".into()).is_err(),
        "unmatched cancellation must not grow the registry past its bound"
    );
    assert_eq!(
        state.lock().unwrap().pending_jobs.len(),
        MAX_PENDING_JOB_CANCELLATIONS
    );
    register_cancel_in(&state, "pending-0".into()).unwrap();
    let first = begin_in(&state, "first".into(), 1, Some("pending-0".into())).unwrap();
    assert!(first.solver.cancellation_requested());
    let next = begin_in(&state, "next".into(), 2, Some("pending-0".into())).unwrap();
    assert!(!next.solver.cancellation_requested());
    register_cancel_in(&state, "replacement".into()).unwrap();
    let rejected = begin_in(&state, "rejected".into(), 3, Some("excess".into())).unwrap();
    assert!(!rejected.solver.cancellation_requested());
    let state = state.lock().unwrap();
    assert_eq!(state.pending_jobs.len(), MAX_PENDING_JOB_CANCELLATIONS);
    for n in 1..MAX_PENDING_JOB_CANCELLATIONS {
        assert!(state.pending_jobs.contains(format!("pending-{n}").as_str()));
    }
}

#[test]
fn full_pending_registry_never_blocks_active_job_cancellation() {
    let state = Mutex::new(Registry::default());
    let first = begin_in(&state, "active-1".into(), 1, Some("active-job".into())).unwrap();
    let second = begin_in(&state, "active-2".into(), 2, Some("active-job".into())).unwrap();
    let unrelated = begin_in(&state, "unrelated".into(), 3, Some("other-job".into())).unwrap();
    fill(&state, MAX_PENDING_JOB_CANCELLATIONS);
    register_cancel_in(&state, "active-job".into()).unwrap();
    assert!(first.solver.cancellation_requested() && second.solver.cancellation_requested());
    assert!(!unrelated.solver.cancellation_requested());
    assert_eq!(
        state.lock().unwrap().pending_jobs.len(),
        MAX_PENDING_JOB_CANCELLATIONS
    );
    let later = begin_in(&state, "active-3".into(), 3, Some("active-job".into())).unwrap();
    assert!(!later.solver.cancellation_requested());
}

#[test]
fn simultaneous_admission_consumes_a_pending_intent_only_once() {
    let state = Mutex::new(Registry::default());
    register_cancel_in(&state, "one-shot".into()).unwrap();
    let barrier = std::sync::Barrier::new(2);
    let controls = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..2)
            .map(|n| {
                let (state, barrier) = (&state, &barrier);
                scope.spawn(move || {
                    barrier.wait();
                    begin_in(
                        state,
                        format!("same-job-{n}"),
                        n + 1,
                        Some("one-shot".into()),
                    )
                    .unwrap()
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        controls
            .iter()
            .filter(|c| c.solver.cancellation_requested())
            .count(),
        1
    );
    assert!(state.lock().unwrap().pending_jobs.is_empty());
}

#[test]
fn pending_cancellation_identifiers_have_a_byte_bound_and_are_not_normalized() {
    let state = Mutex::new(Registry::default());
    for invalid in [
        String::new(),
        "x".repeat(257),
        "\u{00e9}".repeat(129),
        "bad\njob".into(),
    ] {
        assert!(register_cancel_in(&state, invalid).is_err());
    }
    assert!(state.lock().unwrap().pending_jobs.is_empty());
    for valid in [
        "x".repeat(256),
        "\u{00e9}".repeat(128),
        " Job ".into(),
        "job".into(),
    ] {
        register_cancel_in(&state, valid.clone()).unwrap();
        assert!(state.lock().unwrap().pending_jobs.contains(valid.as_str()));
    }
    assert_eq!(state.lock().unwrap().pending_jobs.len(), 4);
}

#[test]
fn concurrent_pending_cancellation_cannot_overbook_remaining_capacity() {
    let state = Mutex::new(Registry::default());
    fill(&state, MAX_PENDING_JOB_CANCELLATIONS - 4);
    let barrier = std::sync::Barrier::new(16);
    let admitted = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..16)
            .map(|n| {
                let state = &state;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    register_cancel_in(state, format!("contender-{n}")).is_ok()
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| usize::from(w.join().unwrap()))
            .sum::<usize>()
    });
    assert_eq!(admitted, 4);
    assert_eq!(
        state.lock().unwrap().pending_jobs.len(),
        MAX_PENDING_JOB_CANCELLATIONS
    );
}

#[test]
fn pending_cancellation_policy_is_visible_without_exposing_pending_identities() {
    let state = Mutex::new(Registry::default());
    register_cancel_in(&state, "private-future-job".into()).unwrap();
    let snapshot = snapshot_in(&state);
    let policy = &snapshot["pending_job_cancellations"];
    assert_eq!(policy["count"], 1);
    assert_eq!(policy["max_count"], 1024);
    assert_eq!(policy["max_job_id_bytes"], 256);
    assert_eq!(policy["on_capacity"], "reject_new_pending_without_eviction");
    assert_eq!(policy["active_cancellation"], "allowed_at_pending_capacity");
    assert_eq!(policy["automatic_expiry"], false);
    assert_eq!(policy["survives_process_restart"], false);
    assert!(!snapshot.to_string().contains("private-future-job"));
}

#[test]
fn poisoned_pending_registry_does_not_silently_accept_cancellation() {
    let state = Mutex::new(Registry::default());
    let _ = std::panic::catch_unwind(|| {
        let _lock = state.lock().unwrap();
        panic!("test poison");
    });
    assert_eq!(
        register_cancel_in(&state, "job".into()).unwrap_err(),
        CancelRegistrationError::Unavailable
    );
    assert_eq!(snapshot_in(&state)["available"], false);
}
