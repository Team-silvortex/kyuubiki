use super::*;
use sha2::{Digest, Sha256};
use std::io::Write;

fn replace_receipt(path: &Path, value: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(value).unwrap();
    let mut bytes = format!("{:x}\n", Sha256::digest(&body)).into_bytes();
    bytes.extend(body);
    let mut file = io::open_private(path, false).unwrap();
    file.set_len(0).unwrap();
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
    bytes
}

fn original(store: &mut Store, failed: bool) -> (Record, Value) {
    let record = admit(store, &task('a'));
    let response = if failed {
        RpcResponse::error_with_details(
            record.request_id.clone(),
            "invalid_input",
            "test rejection",
            json!({"event":"diagnostic", "progress":{"ok":true}, "fraction":-0.0}),
        )
    } else {
        RpcResponse::success(
            record.request_id.clone(),
            json!({
                "event":"physical_data", "progress":{"ok":false},
                "negative_zero":-0.0, "tiny":f64::from_bits(1),
                "precise":1.0000000000000002, "max":u64::MAX,
            }),
        )
    };
    let report = store.finish(&record, response).unwrap();
    (record, report)
}

#[test]
fn terminal_response_cannot_drop_unknown_envelope_fields_during_native_readback() {
    for failed in [false, true] {
        for (field, value) in [
            ("event", json!("progress")),
            ("progress", json!({"fraction":0.5})),
            ("status", json!("running")),
            ("automatic_replay_authorized", json!(true)),
            ("future_extension", Value::Null),
        ] {
            let scratch = Scratch::new();
            let mut store = Store::open(&scratch.0).unwrap();
            let (record, _) = original(&mut store, failed);
            let path = scratch.0.join(format!("{}.receipt", record.execution_id));
            let mut wire: Value = serde_json::from_slice(&fs::read(&path).unwrap()[65..]).unwrap();
            wire["response"][field] = value;
            let changed = replace_receipt(&path, &wire);
            for _ in 0..2 {
                assert!(
                    store.lookup(&record.execution_id).is_err(),
                    "silently discarded {field}"
                );
                assert!(
                    store
                        .admit_with(&task('a'), "p", "duplicate", || -> Result<(), String> {
                            panic!("invalid original receipt must not reexecute");
                        })
                        .is_err()
                );
                assert_eq!(fs::read(&path).unwrap(), changed);
                drop(store);
                store = Store::open(&scratch.0).unwrap();
            }
        }
    }
}

#[test]
fn terminal_failure_cannot_hide_unknown_error_envelope_fields() {
    for (field, value) in [
        ("result", json!({"verified":true})),
        ("execution_started", json!(false)),
        ("automatic_replay_authorized", json!(true)),
        ("future_extension", Value::Null),
    ] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let (record, _) = original(&mut store, true);
        let path = scratch.0.join(format!("{}.receipt", record.execution_id));
        let mut wire: Value = serde_json::from_slice(&fs::read(&path).unwrap()[65..]).unwrap();
        wire["response"]["error"][field] = value;
        let changed = replace_receipt(&path, &wire);
        for _ in 0..2 {
            assert!(
                store.lookup(&record.execution_id).is_err(),
                "silently discarded error.{field}"
            );
            assert_eq!(fs::read(&path).unwrap(), changed);
            drop(store);
            store = Store::open(&scratch.0).unwrap();
        }
    }
}

#[test]
fn terminal_payloads_keep_reserved_names_and_exact_values_in_their_own_scope() {
    for failed in [false, true] {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let (record, report) = original(&mut store, failed);
        let path = scratch.0.join(format!("{}.receipt", record.execution_id));
        let original = fs::read(&path).unwrap();
        drop(store);
        let store = Store::open(&scratch.0).unwrap();
        let restored = store.lookup(&record.execution_id).unwrap();
        assert_eq!(restored, report);
        assert_eq!(fs::read(&path).unwrap(), original);
        let payload = if failed {
            &restored["response"]["error"]["details"]
        } else {
            &restored["response"]["result"]
        };
        assert!(payload["event"].is_string() && payload["progress"].is_object());
        let zero = if failed { "fraction" } else { "negative_zero" };
        assert_eq!(
            payload[zero].as_f64().unwrap().to_bits(),
            (-0.0_f64).to_bits()
        );
        if !failed {
            assert_eq!(payload["tiny"].as_f64().unwrap().to_bits(), 1);
            assert_eq!(
                payload["precise"].as_f64().unwrap().to_bits(),
                1.0000000000000002_f64.to_bits()
            );
            assert_eq!(payload["max"].as_u64(), Some(u64::MAX));
        }
    }
}
