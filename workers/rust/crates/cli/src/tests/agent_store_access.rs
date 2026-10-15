use super::*;
use std::sync::{Arc, mpsc};
use std::thread;

#[test]
fn uncontended_access_does_not_require_a_wait_budget() {
    let mutex = Mutex::new(7);
    assert_eq!(*lock_for(&mutex, Duration::ZERO).unwrap(), 7);
}

#[test]
fn expired_waiters_never_queue_a_late_mutation() {
    let mutex = Arc::new(Mutex::new(0));
    let held = mutex.lock().unwrap();
    let waiters: Vec<_> = (0..4)
        .map(|_| {
            let mutex = Arc::clone(&mutex);
            thread::spawn(move || {
                let started = Instant::now();
                let result =
                    lock_for(&mutex, Duration::from_millis(20)).map(|mut guard| *guard += 1);
                assert!(matches!(result, Err(AccessError::Busy)));
                assert!(started.elapsed() < Duration::from_secs(1));
            })
        })
        .collect();
    for waiter in waiters {
        waiter.join().unwrap();
    }
    assert_eq!(*held, 0);
    drop(held);
    assert_eq!(*lock(&mutex).unwrap(), 0);
    *lock(&mutex).unwrap() += 1;
    assert_eq!(*lock(&mutex).unwrap(), 1);
}

#[test]
fn short_contention_can_clear_within_the_same_wait_budget() {
    let mutex = Arc::new(Mutex::new(0));
    let (ready, received) = mpsc::channel();
    let other = Arc::clone(&mutex);
    let holder = thread::spawn(move || {
        let mut guard = other.lock().unwrap();
        ready.send(()).unwrap();
        thread::sleep(Duration::from_millis(10));
        *guard = 4;
    });
    received.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(*lock_for(&mutex, Duration::from_secs(1)).unwrap(), 4);
    holder.join().unwrap();
}

#[test]
fn poisoned_store_is_unavailable_not_busy_or_silently_recovered() {
    let mutex = Arc::new(Mutex::new(0));
    let other = Arc::clone(&mutex);
    assert!(
        thread::spawn(move || {
            let _guard = other.lock().unwrap();
            panic!("private fault context");
        })
        .join()
        .is_err()
    );
    match lock(&mutex) {
        Err(AccessError::Unavailable(message)) => {
            assert_eq!(message, "execution store is unavailable")
        }
        other => panic!("unexpected store access: {other:?}"),
    }
    assert!(mutex.is_poisoned());
}

#[test]
fn descriptor_policy_exposes_only_the_lock_wait_boundary() {
    let policy = policy();
    assert_eq!(policy["descriptor_lock_wait_ms"], 0);
    assert_eq!(
        policy["recovery_lock_wait_ms"],
        LOCK_WAIT.as_millis() as u64
    );
    assert_eq!(
        policy["deadline_scope"],
        "mutex_acquisition_only_not_filesystem_or_reply"
    );
    assert_eq!(policy["automatic_replay_authorized"], false);
}
