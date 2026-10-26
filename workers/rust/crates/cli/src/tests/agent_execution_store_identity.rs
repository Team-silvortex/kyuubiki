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
fn empty_store_identity_initializes_once_and_keeps_exclusive_ownership() {
    let scratch = Scratch::new();
    io::prepare_root(&scratch.0).unwrap();
    drop(io::open_private(&scratch.0.join("owner.lock"), true).unwrap());
    let store = Store::open(&scratch.0).unwrap();
    let id = store.policy()["store_id"].clone();
    let owner = fs::read(scratch.0.join("owner.lock")).unwrap();
    assert!(!owner.is_empty());
    assert!(Store::open(&scratch.0).is_err());
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
    drop(store);
    assert_eq!(Store::open(&scratch.0).unwrap().policy()["store_id"], id);
    assert_eq!(fs::read(scratch.0.join("owner.lock")).unwrap(), owner);
}

#[test]
fn missing_or_empty_owner_never_relabels_existing_executions() {
    for missing in [false, true] {
        for terminal in [None, Some(true), Some(false)] {
            let scratch = Scratch::new();
            let mut store = Store::open(&scratch.0).unwrap();
            let record = admit(&mut store, &task('a'));
            if let Some(ok) = terminal {
                let response = if ok {
                    RpcResponse::success("request-one", json!({"value":2}))
                } else {
                    RpcResponse::error("request-one", "solve_failed", "original failure")
                };
                store.finish(&record, response).unwrap();
            } else {
                store.abandon(&record.execution_id);
            }
            let original = store.lookup(&record.execution_id).unwrap();
            let owner_path = scratch.0.join("owner.lock");
            let owner = fs::read(&owner_path).unwrap();
            let snapshot: Vec<_> = fs::read_dir(&scratch.0)
                .unwrap()
                .map(|entry| {
                    let path = entry.unwrap().path();
                    let bytes = fs::read(&path).unwrap();
                    (path, bytes)
                })
                .collect();
            drop(store);
            if missing {
                fs::remove_file(&owner_path).unwrap();
            } else {
                fs::write(&owner_path, b"").unwrap();
            }
            for _ in 0..2 {
                assert!(
                    Store::open(&scratch.0).is_err(),
                    "replaced missing={missing} owner with retained terminal={terminal:?}"
                );
                if missing {
                    assert!(!owner_path.exists(), "failed startup created an owner file");
                } else {
                    assert!(
                        fs::read(&owner_path).unwrap().is_empty(),
                        "failed startup rewrote owner identity"
                    );
                }
                for (path, bytes) in &snapshot {
                    if *path != owner_path {
                        assert_eq!(fs::read(path).unwrap(), *bytes);
                    }
                }
            }
            if missing {
                use std::io::Write;
                io::open_private(&owner_path, true)
                    .unwrap()
                    .write_all(&owner)
                    .unwrap();
            } else {
                fs::write(&owner_path, &owner).unwrap();
            }
            let mut store = Store::open(&scratch.0).unwrap();
            assert_eq!(store.lookup(&record.execution_id).unwrap(), original);
            assert!(matches!(
                store.admit_with(&task('a'), "p", "r", || -> Result<(), String> {
                    panic!("restoring identity must not rerun the original execution");
                }),
                Ok((Admission::Existing(_), None))
            ));
            assert_eq!(fs::read(&owner_path).unwrap(), owner);
            admit(&mut store, &task('b'));
        }
    }
}

#[test]
fn identity_bootstrap_never_deletes_pending_or_foreign_entries() {
    for empty_owner in [false, true] {
        for suffix in ["admission.next", "receipt.next", "receipt", "foreign"] {
            let scratch = Scratch::new();
            io::prepare_root(&scratch.0).unwrap();
            let owner = scratch.0.join("owner.lock");
            if empty_owner {
                drop(io::open_private(&owner, true).unwrap());
            }
            let path = scratch.0.join(format!("{}.{}", "a".repeat(32), suffix));
            use std::io::Write;
            io::open_private(&path, true)
                .unwrap()
                .write_all(b"retained evidence")
                .unwrap();
            assert!(
                Store::open(&scratch.0).is_err(),
                "initialized over {suffix}"
            );
            assert_eq!(fs::read(&path).unwrap(), b"retained evidence");
            assert_eq!(owner.exists(), empty_owner);
            if empty_owner {
                assert!(fs::read(&owner).unwrap().is_empty());
            }
        }
    }
}
