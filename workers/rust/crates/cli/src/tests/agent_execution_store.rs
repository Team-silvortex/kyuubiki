use super::*;
use kyuubiki_protocol::RpcMethod;
use std::sync::atomic::{AtomicU64, Ordering};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-durable-store-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn task(id: char) -> DurableAgentTask {
    DurableAgentTask {
        execution_id: id.to_string().repeat(32),
        method: RpcMethod::SolveBar1d,
        params: json!({"tip_force": 1.0000000000000002}),
    }
}

#[test]
fn index_is_bounded_sorted_restartable_and_never_reads_receipts() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    for id in ['c', 'a', 'b'] {
        let record = admit(&mut store, &task(id));
        store.abandon(&record.execution_id);
    }
    fs::write(
        scratch.0.join(format!("{}.receipt", "a".repeat(32))),
        b"corrupt receipt",
    )
    .unwrap();
    let query = kyuubiki_protocol::DurableAgentExecutionList {
        limit: 2,
        after: None,
    };
    let first = store.list(&query).unwrap();
    assert_eq!(first["executions"].as_array().unwrap().len(), 2);
    assert_eq!(first["executions"][0]["execution_id"], "a".repeat(32));
    assert_eq!(first["next_cursor"], "b".repeat(32));
    assert!(first["executions"][0].get("response").is_none());
    assert!(first["executions"][0].get("status").is_none());
    assert!(store.lookup(&"a".repeat(32)).is_err());
    drop(store);
    let store = Store::open(&scratch.0).unwrap();
    assert_eq!(store.list(&query).unwrap(), first);
    let second = store
        .list(&kyuubiki_protocol::DurableAgentExecutionList {
            limit: 2,
            after: Some("b".repeat(32)),
        })
        .unwrap();
    assert_eq!(second["executions"].as_array().unwrap().len(), 1);
    assert_eq!(second["executions"][0]["execution_id"], "c".repeat(32));
    assert!(second["next_cursor"].is_null());
    for limit in [0, 65, usize::MAX] {
        assert!(
            store
                .list(&kyuubiki_protocol::DurableAgentExecutionList { limit, after: None })
                .is_err()
        );
    }
    assert!(
        store
            .list(&kyuubiki_protocol::DurableAgentExecutionList {
                limit: 1,
                after: Some("../invalid".into())
            })
            .is_err()
    );
}

fn admit(store: &mut Store, task: &DurableAgentTask) -> Record {
    match store.admit(task, "process-one", "request-one").unwrap() {
        Admission::New(record) => record,
        Admission::Existing(_) => panic!("expected a new execution"),
    }
}

#[test]
fn synced_receipt_survives_reopen_and_duplicates_never_reexecute() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('a');
    let record = admit(&mut store, &task);
    assert_eq!(
        store.lookup(&task.execution_id).unwrap()["status"],
        "running"
    );
    let response = RpcResponse::success(
        record.request_id.clone(),
        json!({"value":1.0000000000000002}),
    );
    let report = store.finish(&record, response).unwrap();
    assert_eq!(report["status"], "completed");
    drop(store);
    let mut store = Store::open(&scratch.0).unwrap();
    assert_eq!(store.lookup(&task.execution_id).unwrap(), report);
    let Admission::Existing(replayed) = store.admit(&task, "process-two", "request-two").unwrap()
    else {
        panic!("duplicate would execute again");
    };
    assert_eq!(replayed, report);
    assert_eq!(store.entries, 1);
    let mut changed = task;
    changed.params["tip_force"] = json!(1.0000000000000004);
    assert!(store.admit(&changed, "process-two", "request-two").is_err());
}

#[test]
fn abandoned_admission_is_unknown_not_failed_or_replayable() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('b');
    admit(&mut store, &task);
    drop(store);
    for _ in 0..2 {
        let mut store = Store::open(&scratch.0).unwrap();
        let report = store.lookup(&task.execution_id).unwrap();
        assert_eq!(report["status"], "outcome_unknown");
        assert_eq!(report["automatic_replay_authorized"], false);
        assert!(matches!(
            store.admit(&task, "new-process", "retry").unwrap(),
            Admission::Existing(_)
        ));
    }
}

#[test]
fn store_has_exclusive_process_ownership() {
    let scratch = Scratch::new();
    let store = Store::open(&scratch.0).unwrap();
    assert!(Store::open(&scratch.0).is_err());
    drop(store);
    assert!(Store::open(&scratch.0).is_ok());
}

#[test]
fn corruption_and_orphan_receipts_never_authorize_new_work() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('c');
    let record = admit(&mut store, &task);
    store
        .finish(
            &record,
            RpcResponse::success(record.request_id.clone(), json!({})),
        )
        .unwrap();
    let receipt = scratch.0.join(format!("{}.receipt", task.execution_id));
    fs::write(&receipt, b"truncated").unwrap();
    assert!(store.lookup(&task.execution_id).is_err());
    assert!(store.admit(&task, "process", "retry").is_err());
    fs::remove_file(scratch.0.join(format!("{}.admission", task.execution_id))).unwrap();
    assert!(store.admit(&task, "process", "retry").is_err());
}

#[test]
fn limits_reject_admission_without_forgetting_prior_identities() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    store.bytes = MAX_TOTAL_BYTES - MAX_RECEIPT_BYTES as u64;
    assert!(store.admit(&task('d'), "p", "r").is_err());
    assert_eq!(store.entries, 0);
    store.bytes = 0;
    store.entries = MAX_ENTRIES;
    assert!(store.admit(&task('d'), "p", "r").is_err());
}

#[test]
fn oversized_receipt_fails_closed_without_reporting_success() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('e');
    let record = admit(&mut store, &task);
    let response = RpcResponse::success(
        record.request_id.clone(),
        json!("x".repeat(MAX_RECEIPT_BYTES)),
    );
    assert!(store.finish(&record, response).is_err());
    assert_eq!(
        store.lookup(&task.execution_id).unwrap()["status"],
        "outcome_unknown"
    );
    assert!(store.admit(&self::task('f'), "p", "r").is_err());
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
}

#[test]
fn incomplete_temporary_write_is_not_promoted_on_restart() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('f');
    admit(&mut store, &task);
    fs::write(
        scratch
            .0
            .join(format!("{}.receipt.next", task.execution_id)),
        b"partial",
    )
    .unwrap();
    drop(store);
    let store = Store::open(&scratch.0).unwrap();
    assert_eq!(
        store.lookup(&task.execution_id).unwrap()["status"],
        "outcome_unknown"
    );
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
}

#[test]
fn unreadable_deep_receipt_is_rejected_before_any_commit_file_is_created() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('6');
    let record = admit(&mut store, &task);
    let mut value = json!(0);
    for _ in 0..140 {
        value = json!([value]);
    }
    assert!(
        store
            .finish(
                &record,
                RpcResponse::success(record.request_id.clone(), value)
            )
            .is_err()
    );
    assert_eq!(
        store.lookup(&task.execution_id).unwrap()["status"],
        "outcome_unknown"
    );
    assert!(
        !scratch
            .0
            .join(format!("{}.receipt", task.execution_id))
            .exists()
    );
}

#[test]
fn failed_receipt_write_never_publishes_success_and_rejects_further_admission() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('5');
    let record = admit(&mut store, &task);
    fs::create_dir(scratch.0.join(format!("{}.receipt", task.execution_id))).unwrap();
    assert!(
        store
            .finish(
                &record,
                RpcResponse::success(record.request_id.clone(), json!({}))
            )
            .is_err()
    );
    assert_eq!(
        store.lookup(&task.execution_id).unwrap()["status"],
        "outcome_unknown"
    );
    assert!(store.admit(&self::task('4'), "p", "r").is_err());
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_redirect_receipts_or_the_store_root() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('a');
    admit(&mut store, &task);
    symlink(
        "owner.lock",
        scratch.0.join(format!("{}.receipt", task.execution_id)),
    )
    .unwrap();
    assert!(store.lookup(&task.execution_id).is_err());
    let alias = Scratch::new();
    symlink(&scratch.0, &alias.0).unwrap();
    assert!(Store::open(&alias.0).is_err());
    fs::remove_file(&alias.0).unwrap();
}
