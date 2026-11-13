use super::*;
use std::sync::mpsc;
use std::thread;

fn state() -> Arc<Mutex<LifecycleState>> {
    Arc::new(Mutex::new(LifecycleState {
        max_active_executions: Some(1),
        ..LifecycleState::default()
    }))
}

#[test]
fn capacity_handoff_waits_for_the_last_delivery_owner_without_replaying_work() {
    let state = state();
    let execution = acquire_execution_slot(&state, "previous").unwrap();
    let response = execution.clone();
    drop(execution);
    let (ready_tx, ready_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| {
        let state = &state;
        scope.spawn(move || {
            let mut ready = Some(ready_tx);
            tx.send(acquire_execution_slot_waiting(
                state,
                "next",
                Instant::now() + Duration::from_secs(2),
                || {
                    if let Some(tx) = ready.take() {
                        tx.send(()).unwrap();
                    }
                },
            ))
            .unwrap();
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(matches!(
            rx.recv_timeout(Duration::from_millis(20)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        assert_eq!(snapshot_in(state).active_execution_count, 1);
        drop(response);
        let next = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        assert_eq!(snapshot_in(state).active_execution_count, 1);
        drop(next);
    });
    assert_eq!(snapshot_in(&state).active_execution_count, 0);
}

#[test]
fn a_held_slot_still_rejects_at_the_original_handoff_deadline() {
    let state = state();
    let held = acquire_execution_slot(&state, "held").unwrap();
    let started = Instant::now();
    let error =
        acquire_execution_slot_before(&state, "excess", started + Duration::from_millis(20))
            .unwrap_err();
    assert_eq!(error.reason_code, "agent_at_capacity");
    assert!(started.elapsed() >= Duration::from_millis(20));
    assert_eq!(snapshot_in(&state).active_execution_count, 1);
    drop(held);
}

#[test]
fn waiting_admission_rechecks_drain_and_shutdown_before_taking_a_slot() {
    for shutdown in [false, true] {
        let state = state();
        let held = acquire_execution_slot(&state, "held").unwrap();
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::scope(|scope| {
            let state = &state;
            scope.spawn(move || {
                let mut ready = Some(ready_tx);
                tx.send(acquire_execution_slot_waiting(
                    state,
                    "waiting",
                    Instant::now() + Duration::from_secs(2),
                    || {
                        if let Some(tx) = ready.take() {
                            tx.send(()).unwrap();
                        }
                    },
                ))
                .unwrap();
            });
            ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
            assert!(matches!(
                rx.recv_timeout(Duration::from_millis(20)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ));
            if shutdown {
                begin_shutdown_in(state).unwrap();
            } else {
                begin_drain_in(state, "installer", "test handoff fencing").unwrap();
            }
            let error = rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap_err();
            assert_eq!(error.reason_code, "agent_draining");
            assert_eq!(snapshot_in(state).active_execution_count, 1);
            drop(held);
            assert!(!snapshot_in(state).accepting_new_work);
        });
    }
}

#[test]
fn notifications_do_not_extend_the_capacity_handoff_deadline() {
    let state = state();
    let held = acquire_execution_slot(&state, "held").unwrap();
    let signal = state.lock().unwrap().slot_released.clone();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| {
        scope.spawn(|| {
            while !stop.load(Ordering::Acquire) {
                signal.notify_all();
                thread::sleep(Duration::from_millis(1));
            }
        });
        let state = &state;
        scope.spawn(move || {
            tx.send(acquire_execution_slot_before(
                state,
                "excess",
                Instant::now() + Duration::from_millis(20),
            ))
            .unwrap();
        });
        let result = rx.recv_timeout(Duration::from_millis(500));
        stop.store(true, Ordering::Release);
        let active = snapshot_in(state).active_execution_count;
        // Unblock the worker even when the deadline implementation under test is broken.
        drop(held);
        let error = result
            .expect("wakeups must not restart the deadline")
            .unwrap_err();
        assert_eq!(error.reason_code, "agent_at_capacity");
        assert_eq!(active, 1);
    });
}

#[test]
fn competing_handoff_waiters_cannot_overcommit_one_released_slot() {
    let state = state();
    let held = acquire_execution_slot(&state, "held").unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| {
        for n in 0..3 {
            let (state, tx, ready_tx) = (&state, tx.clone(), ready_tx.clone());
            scope.spawn(move || {
                let mut ready = Some(ready_tx);
                let slot = acquire_execution_slot_waiting(
                    state,
                    &format!("waiting-{n}"),
                    Instant::now() + Duration::from_secs(2),
                    || {
                        if let Some(tx) = ready.take() {
                            tx.send(()).unwrap();
                        }
                    },
                );
                tx.send(slot).unwrap();
            });
        }
        for _ in 0..3 {
            ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        }
        drop(held);
        for _ in 0..3 {
            let slot = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
            assert_eq!(snapshot_in(&state).active_execution_count, 1);
            assert!(matches!(
                rx.recv_timeout(Duration::from_millis(10)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ));
            drop(slot);
        }
    });
    assert_eq!(snapshot_in(&state).active_execution_count, 0);
}
