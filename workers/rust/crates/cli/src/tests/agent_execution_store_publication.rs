use super::*;
use std::io::Write;

fn private_file(path: &Path, bytes: &[u8]) {
    let mut file = io::open_private(path, true).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

#[test]
fn colliding_pending_record_is_not_removed_by_a_failed_writer() {
    for stage in ["admission", "receipt"] {
        let scratch = Scratch::new();
        let _store = Store::open(&scratch.0).unwrap();
        let name = format!("{}.{stage}", "a".repeat(32));
        let pending = scratch.0.join(format!("{name}.next"));
        private_file(&pending, b"preexisting evidence");
        assert!(io::write(&scratch.0, &name, &json!({}), MAX_RECORD_BYTES).is_err());
        assert_eq!(fs::read(&pending).unwrap(), b"preexisting evidence");
        assert!(!scratch.0.join(name).exists());
    }
}

#[cfg(unix)]
#[test]
fn colliding_pending_symlink_is_not_unlinked_or_followed() {
    let scratch = Scratch::new();
    let _store = Store::open(&scratch.0).unwrap();
    let name = format!("{}.receipt", "a".repeat(32));
    let target = scratch.0.join(format!("{}.admission", "a".repeat(32)));
    let pending = scratch.0.join(format!("{name}.next"));
    private_file(&target, b"do not change");
    std::os::unix::fs::symlink(&target, &pending).unwrap();
    assert!(io::write(&scratch.0, &name, &json!({}), MAX_RECORD_BYTES).is_err());
    assert_eq!(fs::read_link(&pending).unwrap(), target);
    assert_eq!(fs::read(target).unwrap(), b"do not change");
    assert!(!scratch.0.join(name).exists());
}

#[test]
fn publication_cannot_replace_a_target_appearing_after_the_initial_check() {
    for directory in [false, true] {
        let scratch = Scratch::new();
        let _store = Store::open(&scratch.0).unwrap();
        let target = scratch.0.join(format!("{}.receipt", "a".repeat(32)));
        let pending = target.with_extension("receipt.next");
        assert!(!target.exists());
        private_file(&pending, b"new uncommitted bytes");
        if directory {
            fs::create_dir(&target).unwrap();
        } else {
            private_file(&target, b"retained original");
        }
        assert!(io::publish(&pending, &target).is_err());
        if directory {
            assert!(target.is_dir());
        } else {
            assert_eq!(fs::read(&target).unwrap(), b"retained original");
        }
        assert_eq!(fs::read(&pending).unwrap(), b"new uncommitted bytes");
    }
}

#[cfg(unix)]
#[test]
fn publication_cannot_replace_a_late_symlink_even_when_its_target_is_absent() {
    let scratch = Scratch::new();
    let _store = Store::open(&scratch.0).unwrap();
    let target = scratch.0.join(format!("{}.receipt", "a".repeat(32)));
    let pending = target.with_extension("receipt.next");
    let absent = scratch.0.join("absent");
    private_file(&pending, b"new bytes");
    std::os::unix::fs::symlink(&absent, &target).unwrap();
    assert!(io::publish(&pending, &target).is_err());
    assert_eq!(fs::read_link(target).unwrap(), absent);
    assert!(!absent.exists());
    assert_eq!(fs::read(pending).unwrap(), b"new bytes");
}

#[test]
fn published_record_is_single_link_and_reopens_without_pending_files() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let policy = store.policy();
    assert_eq!(
        policy["record_publication"]["overwrites_existing_record"],
        false
    );
    assert_eq!(
        policy["record_publication"]["hard_link_support_required"],
        true
    );
    assert_eq!(policy["record_publication"]["unsafe_fallback"], false);
    let record = admit(&mut store, &task('a'));
    let completed = store
        .finish(
            &record,
            RpcResponse::success(record.request_id.clone(), json!({"value": 2})),
        )
        .unwrap();
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 3);
    for stage in ["admission", "receipt"] {
        let path = scratch.0.join(format!("{}.{stage}", record.execution_id));
        assert!(io::open_private(&path, false).is_ok());
        assert!(!path.with_extension(format!("{stage}.next")).exists());
    }
    drop(store);
    assert_eq!(
        Store::open(&scratch.0)
            .unwrap()
            .lookup(&record.execution_id)
            .unwrap(),
        completed
    );
}

#[test]
fn interrupted_publication_aliases_are_cleaned_before_records_are_read() {
    for completed in [false, true] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let record = admit(&mut store, &task('a'));
        let expected = if completed {
            store
                .finish(
                    &record,
                    RpcResponse::success(record.request_id.clone(), json!({})),
                )
                .unwrap()
        } else {
            store.abandon(&record.execution_id);
            store.lookup(&record.execution_id).unwrap()
        };
        for stage in if completed {
            vec!["admission", "receipt"]
        } else {
            vec!["admission"]
        } {
            let target = scratch.0.join(format!("{}.{stage}", record.execution_id));
            fs::hard_link(&target, target.with_extension(format!("{stage}.next"))).unwrap();
        }
        drop(store);
        for _ in 0..2 {
            let mut reopened = Store::open(&scratch.0).unwrap();
            assert_eq!(reopened.lookup(&record.execution_id).unwrap(), expected);
            assert!(matches!(
                reopened.admit_with(&task('a'), "restart", "retry", || -> Result<(), String> {
                    panic!("interrupted publication must never reexecute")
                }),
                Ok((Admission::Existing(_), None))
            ));
            assert_eq!(
                fs::read_dir(&scratch.0).unwrap().count(),
                if completed { 3 } else { 2 }
            );
        }
    }
}
