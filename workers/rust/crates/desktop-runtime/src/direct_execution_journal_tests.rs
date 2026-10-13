use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "kyuubiki-direct-journal-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
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
fn intent(id: usize) -> DirectExecutionIntent {
    DirectExecutionIntent {
        scope: "a".repeat(64),
        endpoint: format!("agent-{id}:5001"),
        study_kind: "axial_bar_1d".into(),
        execution_id: format!("{id:032x}"),
        created_at: 1,
    }
}
fn reserve(store: &Store, intent: DirectExecutionIntent) -> Result<serde_json::Value, String> {
    store.handle(DirectExecutionJournalRequest::Reserve { intent })
}
#[test]
fn reopened_journal_blocks_duplicate_dispatch_and_removal_requires_exact_identity() {
    let f = Fixture::new();
    assert_eq!(reserve(&f.store(), intent(1)).unwrap()["is_new"], true);
    let mut other = intent(1);
    other.execution_id = "b".repeat(32);
    other.study_kind = "heat_bar_1d".into();
    let response = reserve(&f.store(), other.clone()).unwrap();
    assert_eq!(response["is_new"], false);
    assert_eq!(response["intent"]["execution_id"], intent(1).execution_id);
    f.store()
        .handle(DirectExecutionJournalRequest::Remove { intent: other })
        .unwrap();
    assert_eq!(f.store().read().unwrap(), vec![intent(1)]);
    f.store()
        .handle(DirectExecutionJournalRequest::Remove { intent: intent(1) })
        .unwrap();
    assert!(f.store().read().unwrap().is_empty());
}
#[test]
fn bound_is_metadata_only_and_never_evicts_pending_work() {
    let f = Fixture::new();
    let store = f.store();
    for id in 0..LIMIT {
        reserve(&store, intent(id)).unwrap();
    }
    assert_eq!(
        reserve(&store, intent(LIMIT)).unwrap_err(),
        "direct:journal_full"
    );
    assert_eq!(store.read().unwrap().len(), LIMIT);
    assert!(fs::metadata(f.0.join(FILE)).unwrap().len() <= MAX_BYTES);
    let mut extra = serde_json::to_value(intent(99)).unwrap();
    extra["token"] = "secret".into();
    assert!(serde_json::from_value::<DirectExecutionIntent>(extra).is_err());
    assert!(
        serde_json::from_value::<DirectExecutionJournalRequest>(
            serde_json::json!({"action":"list","path":"elsewhere"})
        )
        .is_err()
    );
}
#[test]
fn malformed_records_cannot_be_silently_reset() {
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
        assert!(reserve(&store, intent(2)).is_err());
        assert_eq!(fs::read(f.0.join(FILE)).unwrap(), bytes);
    }
}
#[test]
fn process_lock_and_partial_staging_preserve_the_committed_identity() {
    let f = Fixture::new();
    let store = f.store();
    assert!(Store::open(f.0.clone()).is_err());
    reserve(&store, intent(1)).unwrap();
    fs::write(f.0.join("desktop-direct-execution.pending"), b"incomplete").unwrap();
    drop(store);
    assert_eq!(f.store().read().unwrap(), vec![intent(1)]);
    reserve(&f.store(), intent(2)).unwrap();
    assert_eq!(f.store().read().unwrap().len(), 2);
    assert!(!f.0.join("desktop-direct-execution.pending").exists());
}
#[test]
fn input_contract_rejects_ambiguous_targets_unknown_studies_and_reused_ids() {
    let f = Fixture::new();
    let store = f.store();
    reserve(&store, intent(1)).unwrap();
    for endpoint in [
        "http://agent:5001",
        "agent:05001",
        "agent:5001 ",
        "0.0.0.0:5001",
        "agent:0",
    ] {
        let mut invalid = intent(2);
        invalid.endpoint = endpoint.into();
        assert!(reserve(&store, invalid).is_err());
    }
    let mut invalid = intent(2);
    invalid.study_kind = "unknown".into();
    assert!(reserve(&store, invalid).is_err());
    let mut invalid = intent(2);
    invalid.execution_id = intent(1).execution_id;
    assert!(reserve(&store, invalid).is_err());
    assert_eq!(store.read().unwrap(), vec![intent(1)]);
}
#[cfg(unix)]
#[test]
fn private_files_and_symlink_rejection() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let f = Fixture::new();
    let store = f.store();
    reserve(&store, intent(1)).unwrap();
    assert_eq!(
        fs::metadata(f.0.join(FILE)).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let target = f.0.join("untouched");
    fs::write(&target, b"keep").unwrap();
    fs::remove_file(f.0.join(FILE)).unwrap();
    symlink(&target, f.0.join(FILE)).unwrap();
    assert!(store.read().is_err());
    assert!(reserve(&store, intent(2)).is_err());
    assert_eq!(fs::read(target).unwrap(), b"keep");
}

#[test]
fn journal_child() {
    let Some(root) = std::env::var_os("KYUUBIKI_DIRECT_JOURNAL_TEST_CHILD") else {
        return;
    };
    let root = PathBuf::from(root);
    let store = Store::open(root.clone()).unwrap();
    reserve(&store, intent(1)).unwrap();
    fs::write(root.join("ready"), b"synced").unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[test]
fn force_killed_caller_keeps_the_exact_execution_and_releases_the_process_lock() {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let f = Fixture::new();
    let mut child = Child(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "direct_execution_journal::tests::journal_child",
                "--nocapture",
            ])
            .env("KYUUBIKI_DIRECT_JOURNAL_TEST_CHILD", &f.0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !f.0.join("ready").exists() {
        assert!(
            Instant::now() < deadline && child.0.try_wait().unwrap().is_none(),
            "child did not commit its journal"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let store = f.store();
    assert_eq!(store.read().unwrap(), vec![intent(1)]);
    let mut retry = intent(1);
    retry.execution_id = "c".repeat(32);
    let reservation = reserve(&store, retry).unwrap();
    assert_eq!(reservation["is_new"], false);
    assert_eq!(
        reservation["intent"]["execution_id"],
        intent(1).execution_id
    );
}
