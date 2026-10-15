use super::*;

fn descriptor() -> Value {
    let mut value =
        serde_json::to_value(kyuubiki_protocol::AgentDescriptor::solver_agent_default()).unwrap();
    value["execution_admission"] = json!({
        "schema_version":"kyuubiki.agent-execution-admission/v1", "configured":true,
        "max_active_executions":2, "active_execution_count":0
    });
    value
}

#[test]
fn selection_requires_explicit_metadata_instead_of_compatibility_defaults() {
    let valid = descriptor();
    assert_eq!(eligible_score(&valid, "solve_bar_1d"), Ok(100));
    for field in [
        "role",
        "protocol",
        "lifecycle",
        "execution_admission",
        "runtime",
    ] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(eligible_score(&missing, "solve_bar_1d").is_err(), "{field}");
        missing[field] = Value::Null;
        assert!(eligible_score(&missing, "solve_bar_1d").is_err(), "{field}");
    }
}

#[test]
fn selection_checks_protocol_and_method_before_health_ranking() {
    for (pointer, value) in [
        ("/role", json!("observer")),
        ("/protocol/name", json!("foreign-rpc")),
        ("/protocol/rpc_version", json!(2)),
        ("/protocol/transport/kind", json!("http")),
        ("/protocol/transport/encoding", json!("binary")),
        ("/protocol/transport/framing", json!("unframed")),
        ("/protocol/methods", json!(["ping", "describe_agent"])),
    ] {
        let mut invalid = descriptor();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(
            eligible_score(&invalid, "solve_bar_1d").is_err(),
            "{pointer}"
        );
    }
    for method in ["ping", "run_durable_agent_task", "solve_future_unsupported"] {
        assert_eq!(
            eligible_score(&descriptor(), method),
            Err("unsupported_method")
        );
    }
}

#[test]
fn selection_rejects_inconsistent_lifecycle_claims() {
    for (field, value) in [
        ("accepting_new_work", json!(false)),
        ("quiescent", json!(true)),
        ("safe_to_replace", json!(true)),
        ("process_instance_id", json!("")),
        ("state", json!("unavailable")),
        ("schema_version", json!("future")),
        ("mutation_control_scope", json!("anywhere")),
    ] {
        let mut invalid = descriptor();
        invalid["lifecycle"][field] = value;
        assert_eq!(
            eligible_score(&invalid, "solve_bar_1d"),
            Err("invalid_lifecycle"),
            "{field}"
        );
    }
}

#[test]
fn selection_rejects_missing_or_invalid_admission_capacity() {
    for (field, value) in [
        ("schema_version", json!("future")),
        ("configured", json!(false)),
        ("max_active_executions", json!(0)),
        ("max_active_executions", json!(1025)),
        ("max_active_executions", Value::Null),
        ("active_execution_count", json!(-1)),
        ("active_execution_count", json!("0")),
        ("active_execution_count", Value::Null),
    ] {
        let mut invalid = descriptor();
        invalid["execution_admission"][field] = value;
        assert_eq!(
            eligible_score(&invalid, "solve_bar_1d"),
            Err("invalid_execution_admission"),
            "{field}"
        );
    }
}

#[test]
fn independently_observed_active_counts_are_combined_conservatively() {
    for (lifecycle, admission, eligible) in [
        (0, 0, true),
        (1, 0, true),
        (0, 1, true),
        (2, 0, false),
        (0, 2, false),
        (3, 0, false),
    ] {
        let mut value = descriptor();
        value["lifecycle"]["active_execution_count"] = json!(lifecycle);
        value["execution_admission"]["active_execution_count"] = json!(admission);
        assert_eq!(eligible_score(&value, "solve_bar_1d").is_ok(), eligible);
    }
}

#[test]
fn health_is_bounded_ranking_data_not_execution_permission() {
    for score in [0, 100] {
        let mut value = descriptor();
        value["runtime"]["health_score"] = json!(score);
        assert_eq!(eligible_score(&value, "solve_bar_1d"), Ok(score));
        value["execution_admission"]["active_execution_count"] = json!(2);
        assert_eq!(
            eligible_score(&value, "solve_bar_1d"),
            Err("execution_at_capacity")
        );
    }
    for score in [json!(-1), json!(1.5), json!(101), json!("100"), Value::Null] {
        let mut value = descriptor();
        value["runtime"]["health_score"] = score;
        assert_eq!(
            eligible_score(&value, "solve_bar_1d"),
            Err("invalid_health_score")
        );
    }
}
