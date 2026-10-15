use super::*;

#[test]
fn store_identity_is_synced_stable_and_distinct_from_process_identity() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let id = store.policy()["store_id"].as_str().unwrap().to_string();
    let mut task = task('a');
    task.expected_store_id = Some(id.clone());
    let record = admit(&mut store, &task);
    assert_eq!(store.lookup(&record.execution_id).unwrap()["store_id"], id);
    let bytes = fs::read(scratch.0.join("owner.lock")).unwrap();
    drop(store);
    let mut reopened = Store::open(&scratch.0).unwrap();
    assert_eq!(reopened.policy()["store_id"], id);
    assert_eq!(fs::read(scratch.0.join("owner.lock")).unwrap(), bytes);
    match reopened.admit(&task, "new-process", "new-request").unwrap() {
        Admission::Existing(report) => {
            assert_eq!(report["status"], "outcome_unknown");
            assert_eq!(report["process_instance_id"], "process-one");
        }
        _ => panic!("restart replayed a fenced task"),
    }
    let other = Scratch::new();
    assert_ne!(Store::open(&other.0).unwrap().policy()["store_id"], id);
}

#[test]
fn foreign_owner_fence_precedes_admission_duplicate_lookup_and_index() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    admit(&mut store, &task('a'));
    for id in ['a', 'b'] {
        let mut task = task(id);
        task.expected_store_id = Some("0".repeat(32));
        assert!(
            matches!(store.admit_with(&task, "p", "r", || -> Result<(), String> {
            panic!("foreign owner must not reserve execution capacity")
        }), Err(AdmissionFailure::Uncertain(error)) if error.contains("identity mismatch"))
        );
    }
    assert_eq!(store.policy()["stored_entries"], 1);
    assert_eq!(
        store.lookup(&"b".repeat(32)).unwrap()["status"],
        "not_found"
    );
    assert!(
        store
            .list(&kyuubiki_protocol::DurableAgentExecutionList {
                expected_store_id: Some("0".repeat(32)),
                limit: 24,
                after: None,
            })
            .unwrap_err()
            .contains("identity mismatch")
    );
}

#[test]
fn malformed_nonempty_store_identity_is_not_reset() {
    let scratch = Scratch::new();
    drop(Store::open(&scratch.0).unwrap());
    let path = scratch.0.join("owner.lock");
    let original = fs::read(&path).unwrap();
    let mut wrong_checksum = original.clone();
    wrong_checksum[40] = if wrong_checksum[40] == b'a' {
        b'b'
    } else {
        b'a'
    };
    for bytes in [
        b"truncated".to_vec(),
        original[..35].to_vec(),
        wrong_checksum,
        vec![b'x'; 1024],
    ] {
        fs::write(&path, &bytes).unwrap();
        assert!(Store::open(&scratch.0).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    fs::write(&path, original).unwrap();
    assert!(Store::open(&scratch.0).is_ok());
}

#[test]
fn empty_legacy_lock_is_initialized_once_without_rewriting_task_records() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let record = admit(&mut store, &task('a'));
    drop(store);
    let record_path = scratch.0.join(format!("{}.admission", record.execution_id));
    let admission = fs::read(&record_path).unwrap();
    fs::write(scratch.0.join("owner.lock"), b"").unwrap();
    let store = Store::open(&scratch.0).unwrap();
    let id = store.policy()["store_id"].clone();
    assert_eq!(
        store.lookup(&record.execution_id).unwrap()["status"],
        "outcome_unknown"
    );
    assert_eq!(fs::read(&record_path).unwrap(), admission);
    drop(store);
    assert_eq!(Store::open(&scratch.0).unwrap().policy()["store_id"], id);
}
