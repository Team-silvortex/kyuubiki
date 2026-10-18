#![cfg(unix)]

use super::*;

#[test]
fn live_owner_corruption_blocks_operations_without_rewriting_identity() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let record = admit(&mut store, &task('a'));
    let path = scratch.0.join("owner.lock");
    let original = fs::read(&path).unwrap();
    for bytes in [b"".as_slice(), b"truncated"] {
        fs::write(&path, bytes).unwrap();
        assert!(store.check_owner(Some(&store.store_id)).is_err());
        assert!(store.lookup(&record.execution_id).is_err());
        assert!(store.lookup(&"b".repeat(32)).is_err());
        assert!(store.list(&query()).is_err());
        assert_no_reservation(&mut store);
        assert!(store.finish(&record, response(&record)).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(
            !scratch
                .0
                .join(format!("{}.receipt", record.execution_id))
                .exists()
        );
    }
    fs::write(path, original).unwrap();
    assert_eq!(
        store.lookup(&record.execution_id).unwrap()["status"],
        "running"
    );
    assert_eq!(
        store.finish(&record, response(&record)).unwrap()["status"],
        "completed"
    );
}

#[cfg(unix)]
#[test]
fn relocated_live_store_never_reads_or_commits_into_its_replacement() {
    let scratch = Scratch::new();
    let relocated = Scratch::new();
    let mut original = Store::open(&scratch.0).unwrap();
    let record = admit(&mut original, &task('a'));
    fs::rename(&scratch.0, &relocated.0).unwrap();
    let replacement = Store::open(&scratch.0).unwrap();
    assert_ne!(original.store_id, replacement.store_id);
    // A partial restore may contain the exact admission, but not the locked owner.
    let name = format!("{}.admission", record.execution_id);
    fs::copy(relocated.0.join(&name), scratch.0.join(name)).unwrap();
    assert!(original.check_owner(Some(&original.store_id)).is_err());
    assert!(original.lookup(&record.execution_id).is_err());
    assert!(original.list(&query()).is_err());
    assert_no_reservation(&mut original);
    assert!(original.finish(&record, response(&record)).is_err());
    assert!(
        !scratch
            .0
            .join(format!("{}.receipt", record.execution_id))
            .exists()
    );
    drop(replacement);
    fs::remove_dir_all(&scratch.0).unwrap();
    fs::rename(&relocated.0, &scratch.0).unwrap();
    assert_eq!(
        original.lookup(&record.execution_id).unwrap()["status"],
        "running"
    );
    assert_eq!(
        original.finish(&record, response(&record)).unwrap()["status"],
        "completed"
    );
}

#[cfg(unix)]
#[test]
fn copied_owner_nonce_does_not_replace_the_original_locked_file() {
    let scratch = Scratch::new();
    let mut original = Store::open(&scratch.0).unwrap();
    let record = admit(&mut original, &task('a'));
    let lock = scratch.0.join("owner.lock");
    let moved = scratch.0.join("moved.lock");
    fs::rename(&lock, &moved).unwrap();
    fs::copy(&moved, &lock).unwrap();
    assert!(original.check_owner(None).is_err());
    assert!(original.lookup(&record.execution_id).is_err());
    assert_no_reservation(&mut original);
    assert!(original.finish(&record, response(&record)).is_err());
    fs::remove_file(&lock).unwrap();
    fs::rename(moved, lock).unwrap();
    assert!(original.lookup(&record.execution_id).is_ok());
}

fn query() -> kyuubiki_protocol::DurableAgentExecutionList {
    kyuubiki_protocol::DurableAgentExecutionList {
        expected_store_id: None,
        limit: 24,
        after: None,
    }
}

fn response(record: &Record) -> RpcResponse {
    RpcResponse::success(record.request_id.clone(), json!({"value":1}))
}

fn assert_no_reservation(store: &mut Store) {
    assert!(matches!(
        store.admit_with(&task('b'), "p", "r", || -> Result<(), String> {
            panic!("a detached store must not reserve or run work")
        }),
        Err(AdmissionFailure::Uncertain(_))
    ));
}
