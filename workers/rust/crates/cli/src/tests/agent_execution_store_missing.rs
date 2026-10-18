use super::*;
use std::cell::Cell;
use std::io::Write;

fn query(after: Option<String>) -> kyuubiki_protocol::DurableAgentExecutionList {
    kyuubiki_protocol::DurableAgentExecutionList {
        expected_store_id: None,
        limit: 24,
        after,
    }
}

fn restore(path: &Path, bytes: &[u8]) {
    let mut file = io::open_private(path, true).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

#[test]
fn missing_live_admission_never_reserves_a_second_execution() {
    for abandoned in [false, true] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let original = task('a');
        let record = admit(&mut store, &original);
        if abandoned {
            store.abandon(&record.execution_id);
        }
        let path = scratch.0.join(format!("{}.admission", record.execution_id));
        fs::remove_file(&path).unwrap();
        let reserved = Cell::new(false);
        let retry = store.admit_with(&original, "another-process", "duplicate", || {
            reserved.set(true);
            Ok(())
        });
        assert!(
            !reserved.get(),
            "missing durable evidence must not reserve a second execution"
        );
        assert!(matches!(retry, Err(AdmissionFailure::Uncertain(_))));
        assert!(!path.exists());
        assert_eq!(store.entries, 1);
        assert_eq!(store.active.len(), usize::from(!abandoned));
    }
}

#[test]
fn missing_admission_is_not_not_found_and_restoration_preserves_original_identity() {
    for reopen in [false, true] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let record = admit(&mut store, &task('a'));
        store.abandon(&record.execution_id);
        if reopen {
            drop(store);
            store = Store::open(&scratch.0).unwrap();
        }
        let path = scratch.0.join(format!("{}.admission", record.execution_id));
        let original = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert!(store.lookup(&record.execution_id).is_err());
        assert_eq!(
            store.lookup(&"f".repeat(32)).unwrap()["status"],
            "not_found"
        );
        restore(&path, &original);
        let observed = store.lookup(&record.execution_id).unwrap();
        assert_eq!(observed["status"], "outcome_unknown");
        assert_eq!(observed["request_id"], record.request_id);
        assert_eq!(observed["automatic_replay_authorized"], false);
        let Ok((Admission::Existing(duplicate), None)) =
            store.admit_with(&task('a'), "new", "retry", || -> Result<(), String> {
                panic!("replay")
            })
        else {
            panic!("restored admission must retain its identity")
        };
        assert_eq!(duplicate, observed);
    }
}

#[test]
fn missing_admissions_cannot_silently_shorten_a_paginated_recovery_index() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let missing = admit(&mut store, &task('a'));
    let healthy = admit(&mut store, &task('b'));
    let path = scratch
        .0
        .join(format!("{}.admission", missing.execution_id));
    let original = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    for after in [None, Some("b".repeat(32))] {
        assert!(
            store.list(&query(after)).is_err(),
            "missing evidence must not disappear behind the cursor"
        );
    }
    assert_eq!(
        store.lookup(&healthy.execution_id).unwrap()["status"],
        "running"
    );
    assert!(
        store
            .finish(
                &missing,
                RpcResponse::success(missing.request_id.clone(), json!({}))
            )
            .is_err()
    );
    let completed = store
        .finish(
            &healthy,
            RpcResponse::success(healthy.request_id.clone(), json!({"value":1})),
        )
        .unwrap();
    assert_eq!(completed["status"], "completed");
    assert!(!store.write_failed);
    assert!(store.admit(&task('c'), "p", "fresh").is_ok());
    restore(&path, &original);
    assert_eq!(
        store.list(&query(None)).unwrap()["executions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn orphan_receipts_are_unavailable_not_absent_even_after_restart() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let original = task('a');
    let record = admit(&mut store, &original);
    let completed = store
        .finish(
            &record,
            RpcResponse::success(record.request_id.clone(), json!({"value":1})),
        )
        .unwrap();
    let admission = scratch.0.join(format!("{}.admission", record.execution_id));
    let bytes = fs::read(&admission).unwrap();
    fs::remove_file(&admission).unwrap();
    for _ in 0..2 {
        assert!(store.lookup(&record.execution_id).is_err());
        assert!(store.list(&query(None)).is_err());
        assert!(store.list(&query(Some("b".repeat(32)))).is_err());
        assert!(matches!(
            store.admit_with(&original, "p", "retry", || -> Result<(), String> {
                panic!("orphan receipt must never authorize execution")
            }),
            Err(AdmissionFailure::Uncertain(_))
        ));
        drop(store);
        store = Store::open(&scratch.0).unwrap();
    }
    restore(&admission, &bytes);
    assert_eq!(store.lookup(&record.execution_id).unwrap(), completed);
    assert!(matches!(
        store.admit(&original, "p", "retry"),
        Ok(Admission::Existing(_))
    ));
}

#[test]
fn never_admitted_identity_still_has_an_explicit_not_found_state() {
    let scratch = Scratch::new();
    let store = Store::open(&scratch.0).unwrap();
    let absent = store.lookup(&"a".repeat(32)).unwrap();
    assert_eq!(absent["status"], "not_found");
    assert_eq!(absent["automatic_replay_authorized"], false);
    let index = store.list(&query(None)).unwrap();
    assert!(index["executions"].as_array().unwrap().is_empty());
    assert!(index["next_cursor"].is_null());
}
