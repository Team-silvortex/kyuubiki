use super::*;
use std::cell::Cell;
use std::io::Write;

fn restore(path: &Path, bytes: &[u8]) {
    let mut file = io::open_private(path, true).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn blocked_before_reservation(store: &mut Store) {
    let reserved = Cell::new(false);
    let outcome = store.admit_with(&task('f'), "new-process", "new-request", || {
        reserved.set(true);
        Ok(())
    });
    assert!(
        !reserved.get(),
        "changed inventory must not reserve execution"
    );
    match outcome {
        Err(AdmissionFailure::Uncertain(message)) => assert!(message.contains("inventory")),
        _ => panic!("changed inventory must require inspection, not claim capacity rejection"),
    }
    assert_eq!(
        store.policy()["admission_blocked_after_inventory_change"],
        true
    );
    assert!(!store.write_failed);
    assert!(
        !store
            .root
            .join(format!("{}.admission", "f".repeat(32)))
            .exists()
    );
}

fn complete(store: &mut Store, id: char) -> Value {
    let record = admit(store, &task(id));
    store
        .finish(
            &record,
            RpcResponse::success(record.request_id.clone(), json!({"value":1})),
        )
        .unwrap()
}

#[test]
fn restored_records_cannot_bypass_cached_capacity_before_explicit_restart() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let completed = complete(&mut store, 'a');
    let mut originals = Vec::new();
    for suffix in ["admission", "receipt"] {
        let path = scratch.0.join(format!("{}.{}", "a".repeat(32), suffix));
        originals.push((path.clone(), fs::read(&path).unwrap()));
        fs::remove_file(path).unwrap();
    }
    drop(store);
    let mut store = Store::open(&scratch.0).unwrap();
    assert_eq!(store.entries, 0);
    for (path, bytes) in &originals {
        restore(path, bytes);
    }
    blocked_before_reservation(&mut store);
    assert_eq!(store.entries, 0, "no silent counter reconciliation");
    assert_eq!(store.lookup(&"a".repeat(32)).unwrap(), completed);
    assert!(matches!(
        store.admit_with(&task('a'), "p", "r", || -> Result<(), String> {
            panic!("existing receipt must not reserve")
        }),
        Ok((Admission::Existing(_), None))
    ));
    blocked_before_reservation(&mut store);
    for (path, bytes) in &originals {
        assert_eq!(&fs::read(path).unwrap(), bytes);
    }
    drop(store);
    let mut store = Store::open(&scratch.0).unwrap();
    assert_eq!(store.entries, 1);
    assert_eq!(
        store.bytes,
        originals.iter().map(|(_, b)| b.len() as u64).sum::<u64>()
    );
    assert_eq!(
        store.policy()["admission_blocked_after_inventory_change"],
        false
    );
    complete(&mut store, 'b');
    complete(&mut store, 'c');
    assert_eq!(store.entries, 3);
}

#[test]
fn missing_inventory_fences_new_work_without_cascading_into_healthy_completion() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    complete(&mut store, 'a');
    let healthy = admit(&mut store, &task('b'));
    let path = scratch.0.join(format!("{}.receipt", "a".repeat(32)));
    let original = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    blocked_before_reservation(&mut store);
    let completed = store
        .finish(
            &healthy,
            RpcResponse::success(healthy.request_id.clone(), json!({"value":2})),
        )
        .unwrap();
    assert_eq!(completed["status"], "completed");
    assert_eq!(store.lookup(&healthy.execution_id).unwrap(), completed);
    restore(&path, &original);
    blocked_before_reservation(&mut store);
    drop(store);
    let mut store = Store::open(&scratch.0).unwrap();
    complete(&mut store, 'c');
    assert_eq!(store.entries, 3);
}

#[test]
fn receipt_size_drift_is_sticky_even_after_original_bytes_are_restored() {
    for grow in [false, true] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        complete(&mut store, 'a');
        let path = scratch.0.join(format!("{}.receipt", "a".repeat(32)));
        let original = fs::read(&path).unwrap();
        let mut changed = original.clone();
        if grow {
            changed.push(b' ');
        } else {
            changed.pop();
        }
        fs::write(&path, &changed).unwrap();
        blocked_before_reservation(&mut store);
        assert_eq!(fs::read(&path).unwrap(), changed);
        fs::write(&path, &original).unwrap();
        blocked_before_reservation(&mut store);
        assert_eq!(
            store.lookup(&"a".repeat(32)).unwrap()["status"],
            "completed"
        );
        drop(store);
        let mut store = Store::open(&scratch.0).unwrap();
        complete(&mut store, 'b');
    }
}

#[test]
fn foreign_and_nonregular_inventory_entries_fail_closed_without_cleanup() {
    for directory in [false, true] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let path = if directory {
            scratch.0.join(format!("{}.admission", "a".repeat(32)))
        } else {
            scratch.0.join("foreign-file")
        };
        if directory {
            fs::create_dir(&path).unwrap();
        } else {
            fs::write(&path, "foreign").unwrap();
        }
        blocked_before_reservation(&mut store);
        assert!(path.exists(), "admission must not clean foreign evidence");
    }
}

#[test]
fn inventory_gate_is_metadata_only_not_a_receipt_content_verifier() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    complete(&mut store, 'a');
    let path = scratch.0.join(format!("{}.receipt", "a".repeat(32)));
    let mut bytes = fs::read(&path).unwrap();
    bytes[0] = if bytes[0] == b'0' { b'1' } else { b'0' };
    fs::write(&path, &bytes).unwrap();
    complete(&mut store, 'b');
    assert!(store.lookup(&"a".repeat(32)).is_err());
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert_eq!(
        store.policy()["admission_blocked_after_inventory_change"],
        false
    );
}

#[test]
fn pending_directory_flood_is_bounded_without_admission_or_live_cleanup() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    for id in 0..=4 * MAX_ENTRIES {
        fs::write(scratch.0.join(format!("{id:032x}.receipt.next")), b"").unwrap();
    }
    blocked_before_reservation(&mut store);
    assert_eq!(
        fs::read_dir(&scratch.0).unwrap().count(),
        4 * MAX_ENTRIES + 2
    );
}

#[cfg(unix)]
#[test]
fn finalized_inventory_symlink_is_never_followed_or_repaired() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    complete(&mut store, 'a');
    let path = scratch.0.join(format!("{}.receipt", "a".repeat(32)));
    let outside = Scratch::new();
    fs::create_dir(&outside.0).unwrap();
    let target = outside.0.join("original");
    fs::rename(&path, &target).unwrap();
    let original = fs::read(&target).unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    blocked_before_reservation(&mut store);
    assert_eq!(fs::read_link(&path).unwrap(), target);
    assert_eq!(fs::read(target).unwrap(), original);
}
