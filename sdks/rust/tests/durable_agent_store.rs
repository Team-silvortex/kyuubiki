use kyuubiki_headless_sdk::DurableAgentStoreStatus as Status;
use serde_json::{Value, json};

fn ready() -> Value {
    json!({"durable_execution":{
        "schema_version":"kyuubiki.agent-durable-execution/v1", "enabled":true,
        "available":true, "store_id":"a".repeat(32), "survives_agent_restart":true,
        "automatic_replay_authorized":false, "admission_blocked_after_write_failure":false
    }})
}

#[test]
fn durable_store_descriptor_distinguishes_readiness_without_granting_replay() {
    let descriptor = ready();
    let observed = Status::from_descriptor(&descriptor).unwrap();
    assert_eq!(
        observed,
        Status::Ready {
            store_id: "a".repeat(32)
        }
    );
    assert_eq!(observed.ready_store_id().unwrap(), "a".repeat(32));
    let mut blocked = descriptor.clone();
    blocked["durable_execution"]["admission_blocked_after_write_failure"] = json!(true);
    let observed = Status::from_descriptor(&blocked).unwrap();
    assert_eq!(
        observed,
        Status::AdmissionBlocked {
            store_id: "a".repeat(32)
        }
    );
    assert!(
        observed
            .ready_store_id()
            .unwrap_err()
            .to_string()
            .contains("write failure")
    );

    for reason in [
        "store_busy",
        "store_unavailable",
        "store_binding_unavailable",
        "future_reason",
    ] {
        for with_identity in [false, true] {
            let mut unavailable = descriptor.clone();
            let policy = &mut unavailable["durable_execution"];
            policy["available"] = json!(false);
            policy["unavailable_reason"] = json!(reason);
            if !with_identity {
                policy.as_object_mut().unwrap().remove("store_id");
                policy
                    .as_object_mut()
                    .unwrap()
                    .remove("admission_blocked_after_write_failure");
            }
            let observed = Status::from_descriptor(&unavailable).unwrap();
            let expected = if reason == "store_busy" {
                Status::Busy
            } else {
                Status::Unavailable {
                    reason: reason.into(),
                }
            };
            assert_eq!(observed, expected);
            assert!(
                observed.ready_store_id().is_err(),
                "{reason} must not admit work"
            );
        }
    }
}

#[test]
fn disabled_durable_store_is_not_a_busy_or_ready_store() {
    let disabled = json!({"durable_execution":{
        "schema_version":"kyuubiki.agent-durable-execution/v1", "enabled":false,
        "survives_agent_restart":false,"automatic_replay_authorized":false
    }});
    assert_eq!(
        Status::from_descriptor(&disabled).unwrap(),
        Status::Disabled
    );
    assert!(
        Status::Disabled
            .ready_store_id()
            .unwrap_err()
            .to_string()
            .contains("disabled")
    );
    for (key, value) in [
        ("available", json!(true)),
        ("available", json!("false")),
        ("store_id", json!("a".repeat(32))),
        ("unavailable_reason", json!("store_busy")),
        ("admission_blocked_after_write_failure", json!(true)),
        ("admission_blocked_after_inventory_change", json!(true)),
    ] {
        let mut wrong = disabled.clone();
        wrong["durable_execution"][key] = value;
        assert!(Status::from_descriptor(&wrong).is_err(), "{key}");
    }
}

#[test]
fn durable_store_descriptor_rejects_missing_malformed_and_contradictory_state() {
    for field in [
        "schema_version",
        "enabled",
        "available",
        "store_id",
        "survives_agent_restart",
        "automatic_replay_authorized",
        "admission_blocked_after_write_failure",
    ] {
        for remove in [false, true] {
            let mut wrong = ready();
            if remove {
                wrong["durable_execution"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            } else {
                wrong["durable_execution"][field] = Value::Null;
            }
            assert!(
                Status::from_descriptor(&wrong).is_err(),
                "{field} remove={remove}"
            );
        }
    }
    for (field, value) in [
        ("schema_version", json!("future")),
        ("enabled", json!("true")),
        ("available", json!("true")),
        ("available", json!(false)),
        ("store_id", json!("A".repeat(32))),
        ("store_id", json!(9)),
        ("survives_agent_restart", json!(false)),
        ("automatic_replay_authorized", json!(true)),
        ("admission_blocked_after_write_failure", json!("false")),
        ("admission_blocked_after_inventory_change", json!("false")),
        ("admission_blocked_after_inventory_change", Value::Null),
        ("unavailable_reason", json!("store_busy")),
    ] {
        let mut wrong = ready();
        wrong["durable_execution"][field] = value;
        assert!(Status::from_descriptor(&wrong).is_err(), "{field}");
    }
    for wrong in [json!(null), json!({}), json!({"durable_execution":[]})] {
        assert!(Status::from_descriptor(&wrong).is_err());
    }
    assert!(
        Status::Ready {
            store_id: "invalid".into()
        }
        .ready_store_id()
        .is_err()
    );
}

#[test]
fn inventory_fenced_descriptor_blocks_only_new_work_and_retains_original_store_identity() {
    for write_failed in [false, true] {
        let mut descriptor = ready();
        descriptor["durable_execution"]["admission_blocked_after_write_failure"] =
            json!(write_failed);
        descriptor["durable_execution"]["admission_blocked_after_inventory_change"] = json!(true);
        let status = Status::from_descriptor(&descriptor).unwrap();
        assert_eq!(
            status,
            Status::AdmissionBlocked {
                store_id: "a".repeat(32)
            }
        );
        assert!(
            status
                .ready_store_id()
                .unwrap_err()
                .to_string()
                .contains("inventory change")
        );
    }
    let mut descriptor = ready();
    descriptor["durable_execution"]["admission_blocked_after_inventory_change"] = json!(false);
    assert!(matches!(
        Status::from_descriptor(&descriptor).unwrap(),
        Status::Ready { .. }
    ));
    // Absence remains compatible with older descriptors; present null is not false.
    assert!(matches!(
        Status::from_descriptor(&ready()).unwrap(),
        Status::Ready { .. }
    ));
}

#[test]
fn unavailable_descriptor_requires_a_bounded_reason_not_an_inferred_missing_task() {
    for reason in [
        Value::Null,
        json!(true),
        json!(""),
        json!("  "),
        json!("busy\n"),
        json!("x".repeat(257)),
    ] {
        let mut wrong = ready();
        wrong["durable_execution"]["available"] = json!(false);
        wrong["durable_execution"]["unavailable_reason"] = reason;
        assert!(Status::from_descriptor(&wrong).is_err());
    }
}
