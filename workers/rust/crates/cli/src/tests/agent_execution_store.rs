use super::*;
#[path = "agent_execution_store_binding.rs"]
mod binding_tests;
#[path = "agent_execution_store_identity.rs"]
mod identity_tests;
#[path = "agent_execution_store_missing.rs"]
mod missing_tests;
#[path = "agent_execution_store_publication.rs"]
mod publication_tests;
use kyuubiki_protocol::RpcMethod;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "agent_execution_store_response.rs"]
mod response_contract;

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
        expected_store_id: None,
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
        expected_store_id: None,
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
            expected_store_id: None,
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
                .list(&kyuubiki_protocol::DurableAgentExecutionList {
                    expected_store_id: None,
                    limit,
                    after: None
                })
                .is_err()
        );
    }
    assert!(
        store
            .list(&kyuubiki_protocol::DurableAgentExecutionList {
                expected_store_id: None,
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
fn store_capacity_rejection_does_not_write_or_forget_existing_identities() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    for capacity in [false, true] {
        store.entries = if capacity { MAX_ENTRIES } else { 0 };
        store.bytes = if capacity { 0 } else { MAX_TOTAL_BYTES };
        let error = store.admit_with(&task('a'), "process", "submit", || -> Result<(), String> {
            panic!("full store must not reserve an execution");
        });
        let Err(AdmissionFailure::Rejected(report)) = error else {
            panic!("expected a definitive rejection")
        };
        assert_eq!(report["execution_id"], "a".repeat(32));
        assert_eq!(
            report["request_digest"],
            task('a').request_digest().unwrap()
        );
        assert_eq!(report["request_id"], "submit");
        assert_eq!(report["process_instance_id"], "process");
        assert_eq!(report["execution_started"], false);
        assert_eq!(report["automatic_replay_authorized"], false);
        assert_eq!(report["reason"], "store_capacity");
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
        assert!(store.active.is_empty());
    }
    store.entries = 0;
    store.bytes = 0;
    let record = admit(&mut store, &task('a'));
    store.abandon(&record.execution_id);
    store.entries = MAX_ENTRIES;
    assert!(matches!(
        store.admit_with(&task('a'), "new", "retry", || -> Result<(), String> {
            panic!("duplicate")
        }),
        Ok((Admission::Existing(_), None))
    ));
}

fn reject_reservation(store: &mut Store, task: &DurableAgentTask) -> Value {
    match store.admit_with(task, "original-process", "original-submit", || {
        Err::<(), _>("agent_at_capacity: execution slot unavailable".into())
    }) {
        Ok((Admission::Existing(report), None)) => report,
        _ => panic!("expected a retained terminal rejection"),
    }
}

#[test]
fn reservation_rejection_survives_reopen_without_reserving_or_reexecuting() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('a');
    let report = reject_reservation(&mut store, &task);
    assert_eq!(report["status"], "failed");
    assert_eq!(report["request_digest"], task.request_digest().unwrap());
    assert_eq!(report["response"]["id"], "original-submit");
    assert_eq!(
        report["response"]["error"]["code"],
        "durable_admission_rejected"
    );
    assert_eq!(
        report["response"]["error"]["details"]["execution_started"],
        false
    );
    assert_eq!(report["automatic_replay_authorized"], false);
    assert!(store.active.is_empty());
    assert_eq!(store.entries, 1);
    assert!(store.bytes < 2 * MAX_RECORD_BYTES as u64);
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 3);
    drop(store);
    for _ in 0..2 {
        let mut store = Store::open(&scratch.0).unwrap();
        assert_eq!(store.lookup(&task.execution_id).unwrap(), report);
        match store.admit_with(&task, "other-process", "retry", || -> Result<(), String> {
            panic!("must not reserve on duplicate")
        }) {
            Ok((Admission::Existing(duplicate), None)) => assert_eq!(duplicate, report),
            _ => panic!("rejected identity must remain terminal"),
        }
        let index = store
            .list(&kyuubiki_protocol::DurableAgentExecutionList {
                expected_store_id: None,
                limit: 24,
                after: None,
            })
            .unwrap();
        assert_eq!(index["executions"][0]["execution_id"], task.execution_id);
        assert!(index["executions"][0].get("response").is_none());
        let mut changed = task.clone();
        changed.params = json!({"different":true});
        assert!(store.admit_with(&changed, "p", "r", || Ok(())).is_err());
    }
}

#[test]
fn failed_rejection_commit_remains_unknown_and_never_returns_a_transient_safe_rejection() {
    for stage in ["admission", "receipt"] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let task = task('a');
        let obstruction = scratch
            .0
            .join(format!("{}.{stage}.next", task.execution_id));
        fs::create_dir(&obstruction).unwrap();
        assert!(matches!(
            store.admit_with(&task, "p", "r", || Err::<(), _>("agent_draining".into())),
            Err(AdmissionFailure::Uncertain(_))
        ));
        assert!(store.write_failed);
        assert!(store.active.is_empty());
        assert_eq!(
            store.lookup(&task.execution_id).unwrap()["status"],
            if stage == "admission" {
                "not_found"
            } else {
                "outcome_unknown"
            }
        );
        assert!(matches!(
            store.admit_with(&self::task('b'), "p", "r", || Ok(())),
            Err(AdmissionFailure::Uncertain(_))
        ));
        fs::remove_dir(obstruction).unwrap();
        drop(store);
        let mut store = Store::open(&scratch.0).unwrap();
        if stage == "receipt" {
            match store.admit_with(&task, "p2", "r2", || -> Result<(), String> {
                panic!("must not resume a partial commit")
            }) {
                Ok((Admission::Existing(report), None)) => {
                    assert_eq!(report["status"], "outcome_unknown")
                }
                _ => panic!("expected inspect-only state"),
            }
        }
    }
}

#[test]
fn write_failure_or_identity_conflict_never_returns_a_safe_rejection() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    admit(&mut store, &task('a'));
    let mut changed = task('a');
    changed.params = json!({"different":true});
    assert!(matches!(
        store.admit_with(&changed, "p", "r", || Ok(())),
        Err(AdmissionFailure::Uncertain(_))
    ));
    fs::create_dir(scratch.0.join(format!("{}.admission.next", "b".repeat(32)))).unwrap();
    assert!(matches!(
        store.admit_with(&task('b'), "p", "r", || Ok(())),
        Err(AdmissionFailure::Uncertain(_))
    ));
    assert!(store.write_failed);
    assert!(matches!(
        store.admit_with(&task('c'), "p", "r", || Ok(())),
        Err(AdmissionFailure::Uncertain(_))
    ));
    assert!(store.active.contains(&"a".repeat(32)));
}

#[test]
fn oversized_response_becomes_a_small_failed_receipt_without_reporting_success() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('e');
    let record = admit(&mut store, &task);
    let response = RpcResponse::success(
        record.request_id.clone(),
        json!("x".repeat(MAX_RECEIPT_BYTES)),
    );
    let report = store.finish(&record, response).unwrap();
    assert_response_rejected(&report, "byte_limit");
    assert_eq!(store.lookup(&task.execution_id).unwrap(), report);
    assert!(!store.write_failed);
    assert!(store.active.is_empty());
    assert!(store.bytes < 4096);
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 3);
    drop(store);
    let mut store = Store::open(&scratch.0).unwrap();
    assert_eq!(store.lookup(&task.execution_id).unwrap(), report);
    let Admission::Existing(duplicate) = store.admit(&task, "restarted", "retry").unwrap() else {
        panic!("rejected response must not re-execute");
    };
    assert_eq!(duplicate, report);
    assert!(store.admit(&self::task('f'), "p", "r").is_ok());
}

#[test]
fn response_size_limit_does_not_poison_unrelated_admission() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let record = admit(&mut store, &task('a'));
    let _ = store.finish(
        &record,
        RpcResponse::success(
            record.request_id.clone(),
            json!("x".repeat(MAX_RECEIPT_BYTES)),
        ),
    );
    assert!(
        store.admit(&task('b'), "process", "next").is_ok(),
        "a deterministic response size rejection is not a filesystem failure"
    );
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
fn unreadable_deep_response_is_replaced_before_any_commit_file_is_created() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('6');
    let record = admit(&mut store, &task);
    let mut value = json!(0);
    for _ in 0..140 {
        value = json!([value]);
    }
    let report = store
        .finish(
            &record,
            RpcResponse::success(record.request_id.clone(), value),
        )
        .unwrap();
    assert_response_rejected(&report, "restart_decode");
    assert!(store.bytes < 4096);
    drop(store);
    let store = Store::open(&scratch.0).unwrap();
    assert_eq!(store.lookup(&task.execution_id).unwrap(), report);
}

fn assert_response_rejected(report: &Value, reason: &str) {
    assert_eq!(report["status"], "failed");
    assert_eq!(report["automatic_replay_authorized"], false);
    assert_eq!(report["response"]["ok"], false);
    assert!(report["response"]["result"].is_null());
    assert_eq!(
        report["response"]["error"]["code"],
        "durable_response_unrecordable"
    );
    assert_eq!(
        report["response"]["error"]["details"],
        json!({
            "phase":"receipt_validation", "reason":reason, "max_receipt_bytes":MAX_RECEIPT_BYTES,
            "execution_may_have_started":true, "original_response_discarded":true,
            "external_effects_rolled_back":false, "automatic_replay_authorized":false
        })
    );
}

#[test]
fn compact_rejection_write_failure_remains_unknown_and_fences_admission() {
    for deeply_nested in [false, true] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let record = admit(&mut store, &task('a'));
        let obstruction = scratch
            .0
            .join(format!("{}.receipt.next", record.execution_id));
        fs::create_dir(&obstruction).unwrap();
        let mut value = if deeply_nested {
            json!(0)
        } else {
            json!("x".repeat(MAX_RECEIPT_BYTES))
        };
        if deeply_nested {
            for _ in 0..140 {
                value = json!([value]);
            }
        }
        assert!(
            store
                .finish(
                    &record,
                    RpcResponse::success(record.request_id.clone(), value)
                )
                .is_err()
        );
        assert!(store.write_failed);
        assert!(store.active.is_empty());
        assert!(store.admit(&task('b'), "p", "r").is_err());
        let unknown = store.lookup(&record.execution_id).unwrap();
        assert_eq!(unknown["status"], "outcome_unknown");
        assert!(unknown["response"].is_null());
        fs::remove_dir(obstruction).unwrap();
        drop(store);
        let mut store = Store::open(&scratch.0).unwrap();
        assert_eq!(store.lookup(&record.execution_id).unwrap(), unknown);
        assert!(matches!(
            store.admit(&task('a'), "new", "retry"),
            Ok(Admission::Existing(_))
        ));
    }
}

#[test]
fn rejected_error_payload_does_not_affect_a_siblings_reserved_receipt() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let bad = admit(&mut store, &task('a'));
    let sibling = admit(&mut store, &task('b'));
    let report = store
        .finish(
            &bad,
            RpcResponse::error_with_details(
                bad.request_id.clone(),
                "operator_error",
                "original error",
                json!("x".repeat(MAX_RECEIPT_BYTES)),
            ),
        )
        .unwrap();
    assert_response_rejected(&report, "byte_limit");
    assert!(!report.to_string().contains("operator_error"));
    assert!(store.active.contains(&sibling.execution_id));
    let success = store
        .finish(
            &sibling,
            RpcResponse::success(sibling.request_id.clone(), json!({"value":3})),
        )
        .unwrap();
    assert_eq!(success["status"], "completed");
    assert!(!store.write_failed);
    let mut changed = task('a');
    changed.params["tip_force"] = json!(2);
    assert!(store.admit(&changed, "p", "r").is_err());
}

#[test]
fn preparation_has_typed_size_and_decode_errors_without_a_disk_write() {
    let value = json!("bounded");
    let size = serde_json::to_vec(&value).unwrap().len();
    assert!(io::prepare(&value, size).is_ok());
    assert!(matches!(
        io::prepare(&value, size - 1),
        Err(io::PreparationError::ByteLimit)
    ));
    let mut nested = json!(0);
    for _ in 0..140 {
        nested = json!([nested]);
    }
    assert!(matches!(
        io::prepare(&nested, MAX_RECEIPT_BYTES),
        Err(io::PreparationError::RestartDecode(_))
    ));
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

#[test]
fn serializer_failure_is_not_misclassified_by_matching_its_message() {
    #[derive(Deserialize)]
    struct BrokenResponse;
    impl Serialize for BrokenResponse {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom(
                "execution receipt exceeds byte limit",
            ))
        }
    }
    let Err(error) = io::prepare(&BrokenResponse, MAX_RECEIPT_BYTES) else {
        panic!("serializer must fail");
    };
    assert_eq!(error.reason(), "serialization");
    assert!(matches!(error, io::PreparationError::Encoding(_)));
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
