use super::{
    MATRIX_PATH, RunnerResult, SCHEMA_VERSION, TOPOLOGY_PATH, build_tensor_report,
    derive_evidence_aware_gap, derive_gap, validate_tensor_config,
};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn run_self_test() -> RunnerResult<()> {
    if derive_gap("covered", true) != "ok"
        || derive_gap("partial", true) != "weak"
        || derive_gap("planned", true) != "required_gap"
    {
        return Err("self-test gap derivation failed".to_string());
    }
    let weak = derive_evidence_aware_gap(
        "covered",
        true,
        &json!({ "test_command_count": 0, "contract_evidence_count": 0 }),
    );
    let ok = derive_evidence_aware_gap(
        "covered",
        true,
        &json!({ "test_command_count": 1, "contract_evidence_count": 0 }),
    );
    if weak != "weak_evidence" || ok != "ok" {
        return Err("self-test evidence-aware gap derivation failed".to_string());
    }

    let topology = fixture_topology();
    let matrix = fixture_matrix();
    let tensor = fixture_tensor();
    validate_tensor_config(Path::new("."), &tensor, &topology, &matrix)?;
    let report = build_tensor_report(&tensor, &topology, &matrix);
    if report.get("blocking_gap_count").and_then(Value::as_u64) != Some(0)
        || report
            .pointer("/module_summary/engine/counts/weak")
            .and_then(Value::as_u64)
            != Some(1)
        || report
            .pointer("/cells/engine/solver_execution/evidence_depth/test_command_count")
            .and_then(Value::as_u64)
            != Some(2)
    {
        return Err("self-test report derivation failed".to_string());
    }
    Ok(())
}

fn fixture_topology() -> Value {
    json!({
        "benchmark_lanes": { "runtime_solver": "r" },
        "security_lanes": { "data_contract": "d" },
        "lane_test_plan": {
            "benchmark": { "runtime_solver": [{ "id": "rt", "command": "make test-rust", "scope": "local" }] },
            "security": { "data_contract": [{ "id": "schema", "command": "make architecture-check", "scope": "release" }] }
        },
        "modules": [{
            "id": "engine",
            "layer": "runtime_data_plane",
            "benchmark_lanes": ["runtime_solver"],
            "security_lanes": ["data_contract"]
        }]
    })
}

fn fixture_matrix() -> Value {
    json!({
        "paradigms": { "solver_execution": "s" },
        "required_by_module": { "engine": ["solver_execution"] },
        "cells": { "engine": { "solver_execution": "partial" } }
    })
}

fn fixture_tensor() -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "topology": TOPOLOGY_PATH,
        "matrix": MATRIX_PATH,
        "depth_axes": { "required": "r", "status": "s" },
        "paradigm_lanes": {
            "solver_execution": {
                "benchmark": ["runtime_solver"],
                "security": ["data_contract"]
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::super::{derive_evidence_aware_gap, derive_gap};
    use serde_json::json;

    #[test]
    fn derives_required_and_optional_gaps() {
        assert_eq!(derive_gap("partial", true), "weak");
        assert_eq!(derive_gap("partial", false), "watch");
    }

    #[test]
    fn covered_required_without_evidence_is_weak() {
        assert_eq!(
            derive_evidence_aware_gap(
                "covered",
                true,
                &json!({"test_command_count": 0, "contract_evidence_count": 0})
            ),
            "weak_evidence"
        );
    }
}
