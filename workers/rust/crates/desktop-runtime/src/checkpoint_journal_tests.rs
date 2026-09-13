use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "kyuubiki-checkpoint-test-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        )))
    }
    fn store(&self) -> Store {
        Store::open(self.0.clone()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn intent(index: usize) -> CheckpointIntent {
    CheckpointIntent {
        key: format!("{index:064x}"),
        scope: "a".repeat(64),
        operation: "model".into(),
        parent_id: "project".into(),
        request_id: format!("native-request-key-{index:04}"),
        created_at: 1_800_000_000_000,
    }
}

#[test]
fn identity_survives_store_reopen_and_cleanup_requires_exact_request() {
    let f = Fixture::new();
    let original = f.store().reserve(intent(1)).unwrap();
    assert!(original.is_new);
    let mut retry = intent(1);
    retry.request_id = "another-random-request-0001".into();
    let recovered = f.store().reserve(retry).unwrap();
    assert!(!recovered.is_new);
    assert_eq!(original.intent, recovered.intent);
    f.store()
        .remove(&original.intent.key, "wrong-request-id-0001")
        .unwrap();
    assert_eq!(f.store().read().unwrap().len(), 1);
    f.store()
        .remove(&original.intent.key, &original.intent.request_id)
        .unwrap();
    assert!(f.store().read().unwrap().is_empty());
}

#[test]
fn capacity_rejects_new_keys_without_evicting_an_uncertain_write() {
    let f = Fixture::new();
    let store = f.store();
    for index in 0..MAX_ENTRIES {
        store.reserve(intent(index)).unwrap();
    }
    assert_eq!(
        store.reserve(intent(65)).err().unwrap(),
        "checkpoint:recovery_capacity_reached"
    );
    assert!(!store.reserve(intent(0)).unwrap().is_new);
    assert_eq!(store.read().unwrap().len(), MAX_ENTRIES);
    assert!(fs::metadata(f.0.join(FILE)).unwrap().len() <= MAX_BYTES);
}

#[test]
fn rejects_unknown_fields_and_invalid_records_without_overwriting() {
    let f = Fixture::new();
    let store = f.store();
    store.reserve(intent(1)).unwrap();
    let before = fs::read(f.0.join(FILE)).unwrap();
    for bad in [
        serde_json::json!({"key":"abc","payload":{"secret":true}}),
        {
            let mut value = serde_json::to_value(intent(1)).unwrap();
            value["token"] = "secret".into();
            value
        },
    ] {
        assert!(serde_json::from_value::<CheckpointIntent>(bad).is_err());
    }
    let mut invalid = intent(2);
    invalid.created_at = u64::MAX;
    assert!(store.reserve(invalid).is_err());
    let mut invalid = intent(1);
    invalid.parent_id = "wrong-project".into();
    assert!(store.reserve(invalid).is_err());
    assert_eq!(fs::read(f.0.join(FILE)).unwrap(), before);
}

#[test]
fn corrupt_or_oversized_journal_is_not_silently_reset() {
    let f = Fixture::new();
    let store = f.store();
    for bytes in [
        b"{truncated".to_vec(),
        vec![b'x'; MAX_BYTES as usize + 1],
        serde_json::to_vec(&Journal {
            version: 1,
            pending: vec![intent(1), intent(1)],
        })
        .unwrap(),
    ] {
        fs::write(f.0.join(FILE), &bytes).unwrap();
        assert!(store.read().is_err());
        assert!(store.reserve(intent(2)).is_err());
        assert_eq!(fs::read(f.0.join(FILE)).unwrap(), bytes);
    }
}

#[test]
fn store_lock_prevents_concurrent_lost_updates_and_retains_its_inode() {
    let f = Fixture::new();
    let first = f.store();
    assert!(Store::open(f.0.clone()).is_err());
    first.reserve(intent(1)).unwrap();
    drop(first);
    assert_eq!(f.store().read().unwrap().len(), 1);
    assert!(f.0.join("desktop-checkpoint-recovery.lock").is_file());
}

#[test]
fn interrupted_staging_file_does_not_replace_the_last_committed_record() {
    let f = Fixture::new();
    let store = f.store();
    store.reserve(intent(1)).unwrap();
    fs::write(f.0.join("desktop-checkpoint-recovery.pending"), b"partial").unwrap();
    assert_eq!(store.read().unwrap(), vec![intent(1)]);
    store.reserve(intent(2)).unwrap();
    assert_eq!(store.read().unwrap().len(), 2);
    assert!(!f.0.join("desktop-checkpoint-recovery.pending").exists());
}

#[cfg(unix)]
#[test]
fn journal_is_private_and_rejects_symlinked_storage() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let f = Fixture::new();
    let store = f.store();
    store.reserve(intent(1)).unwrap();
    assert_eq!(
        fs::metadata(f.0.join(FILE)).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(f.0.join("desktop-checkpoint-recovery.lock"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let target = f.0.join("untouched");
    fs::write(&target, b"keep").unwrap();
    fs::remove_file(f.0.join(FILE)).unwrap();
    symlink(&target, f.0.join(FILE)).unwrap();
    assert!(store.read().is_err());
    assert!(store.reserve(intent(2)).is_err());
    assert_eq!(fs::read(target).unwrap(), b"keep");
}
