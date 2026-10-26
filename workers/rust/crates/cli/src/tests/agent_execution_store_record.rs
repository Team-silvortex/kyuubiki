use super::*;
use sha2::{Digest, Sha256};

fn replace(path: &Path, value: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(value).unwrap();
    let mut bytes = format!("{:x}\n", Sha256::digest(&body)).into_bytes();
    bytes.extend(body);
    fs::write(path, &bytes).unwrap();
    bytes
}

fn invalid_fields() -> Vec<(&'static str, String)> {
    let mut fields = vec![];
    for field in ["request_id", "process_instance_id", "method"] {
        for value in ["".into(), " ".into(), "a\nb".into(), "a".repeat(257)] {
            fields.push((field, value));
        }
    }
    for value in [
        "".into(),
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
    ] {
        fields.push(("request_digest", value));
    }
    for value in ["ping", "submit_durable_agent_task", "solve_"] {
        fields.push(("method", value.into()));
    }
    fields
}

#[test]
fn malformed_admission_identity_never_becomes_a_recovery_observation_or_index_entry() {
    for terminal in [None, Some(true), Some(false)] {
        for (field, value) in invalid_fields() {
            let scratch = Scratch::new();
            let mut store = Store::open(&scratch.0).unwrap();
            let task = task('a');
            let record = admit(&mut store, &task);
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
            let admission_path = scratch.0.join(format!("{}.admission", record.execution_id));
            let admission_bytes = fs::read(&admission_path).unwrap();
            let mut wire = serde_json::to_value(&record).unwrap();
            wire[field] = json!(value);
            let bad_admission = replace(&admission_path, &wire);
            let receipt_path = scratch.0.join(format!("{}.receipt", record.execution_id));
            let receipts = terminal.map(|_| {
                let bytes = fs::read(&receipt_path).unwrap();
                let mut receipt: Value = serde_json::from_slice(&bytes[65..]).unwrap();
                // Keep the two records consistent so equality alone cannot detect this fault.
                receipt["record"] = wire;
                if field == "request_id" {
                    receipt["response"]["id"] = json!(value);
                }
                let changed = replace(&receipt_path, &receipt);
                (bytes, changed)
            });
            for _ in 0..2 {
                assert!(
                    store.lookup(&record.execution_id).is_err(),
                    "accepted {field}={value:?}"
                );
                assert!(
                    store
                        .list(&kyuubiki_protocol::DurableAgentExecutionList {
                            expected_store_id: None,
                            after: None,
                            limit: 1,
                        })
                        .is_err(),
                    "indexed {field}={value:?}"
                );
                assert!(
                    store
                        .admit_with(&task, "p", "r", || -> Result<(), String> {
                            panic!("invalid admission must not reserve execution");
                        })
                        .is_err()
                );
                assert_eq!(fs::read(&admission_path).unwrap(), bad_admission);
                if let Some((_, changed)) = &receipts {
                    assert_eq!(fs::read(&receipt_path).unwrap(), *changed);
                }
                assert_eq!(
                    store.lookup(&"f".repeat(32)).unwrap()["status"],
                    "not_found"
                );
                drop(store);
                store = Store::open(&scratch.0).unwrap();
            }
            drop(store);
            fs::write(&admission_path, admission_bytes).unwrap();
            if let Some((bytes, _)) = receipts {
                fs::write(&receipt_path, bytes).unwrap();
            }
            let mut store = Store::open(&scratch.0).unwrap();
            assert_eq!(store.lookup(&record.execution_id).unwrap(), original);
            assert!(matches!(
                store.admit(&task, "p", "r"),
                Ok(Admission::Existing(_))
            ));
            admit(&mut store, &super::task('b'));
        }
    }
}

#[test]
fn invalid_new_admission_identity_is_rejected_before_reservation_or_disk_write() {
    for (field, value) in invalid_fields()
        .into_iter()
        .filter(|(field, _)| matches!(*field, "request_id" | "process_instance_id"))
    {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let (process, request) = if field == "request_id" {
            ("process", value.as_str())
        } else {
            (value.as_str(), "request")
        };
        assert!(
            store
                .admit_with(&task('a'), process, request, || -> Result<(), String> {
                    panic!("invalid {field} reached execution reservation");
                })
                .is_err()
        );
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
        assert_eq!(store.entries, 0);
        assert!(store.active.is_empty() && !store.write_failed);
        admit(&mut store, &task('b'));
    }
}

#[test]
fn duplicate_admission_checks_method_even_when_retained_digest_matches() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let task = task('a');
    let record = admit(&mut store, &task);
    store.abandon(&record.execution_id);
    let path = scratch.0.join(format!("{}.admission", record.execution_id));
    let original = fs::read(&path).unwrap();
    let mut wire = serde_json::to_value(&record).unwrap();
    wire["method"] = json!("solve_modal_frame_2d");
    let changed = replace(&path, &wire);
    for _ in 0..2 {
        assert!(
            store
                .admit_with(&task, "p", "r", || -> Result<(), String> {
                    panic!("conflicting method must not reserve execution");
                })
                .is_err(),
            "digest alone accepted a conflicting retained method"
        );
        assert_eq!(fs::read(&path).unwrap(), changed);
        drop(store);
        store = Store::open(&scratch.0).unwrap();
    }
    drop(store);
    fs::write(path, original).unwrap();
    let mut store = Store::open(&scratch.0).unwrap();
    assert!(matches!(
        store.admit(&task, "p", "r"),
        Ok(Admission::Existing(_))
    ));
}

#[test]
fn valid_admission_identity_boundaries_keep_unicode_and_task_ir_without_normalization() {
    for (process, request) in [
        ("p".repeat(256), "r".repeat(256)),
        (
            "\u{7814}\u{7a76}\u{8fdb}\u{7a0b}".into(),
            " \u{8bf7}\u{6c42} ".into(),
        ),
    ] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let mut task = task('a');
        task.method = RpcMethod::RunOperatorTaskIr;
        task.params = json!({"mode":"execute"});
        let Admission::New(record) = store.admit(&task, &process, &request).unwrap() else {
            panic!("expected admission");
        };
        let report = store
            .finish(&record, RpcResponse::success(&request, json!({"value":2})))
            .unwrap();
        drop(store);
        let mut store = Store::open(&scratch.0).unwrap();
        assert_eq!(store.lookup(&record.execution_id).unwrap(), report);
        assert_eq!(report["request_id"], request);
        assert_eq!(report["process_instance_id"], process);
        assert!(matches!(
            store.admit(&task, "new-process", "new-request"),
            Ok(Admission::Existing(_))
        ));
    }
}
