use super::includes::load_tensor_with_includes;
use super::{
    MATRIX_PATH, RunnerResult, SCHEMA_VERSION, TOPOLOGY_PATH, build_tensor_report,
    derive_evidence_aware_gap, derive_gap, validate_tensor_config,
};
use serde_json::{Value, json};
use std::{fs, path::Path, path::PathBuf};

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
    self_test_evidence_include_loader()?;
    Ok(())
}

fn self_test_evidence_include_loader() -> RunnerResult<()> {
    let root = temp_root("kyuubiki_tensor_include_self_test")?;
    fs::write(root.join("evidence.txt"), "included_anchor\n")
        .map_err(|error| format!("failed to write include evidence: {error}"))?;
    fs::write(
        root.join("include.json"),
        serde_json::to_string_pretty(&json!({
            "schema_version": "kyuubiki.module-function-coverage-evidence/v1",
            "paradigm_contract_evidence": {
                "solver_execution": [{
                    "id": "included-contract",
                    "modules": ["engine"],
                    "files": ["evidence.txt"],
                    "required_text": ["included_anchor"]
                }]
            },
            "evidence_claims": [],
            "cell_requirements": []
        }))
        .map_err(|error| format!("failed to render include fixture: {error}"))?,
    )
    .map_err(|error| format!("failed to write include fixture: {error}"))?;
    let mut tensor = fixture_tensor();
    tensor["evidence_includes"] = json!(["include.json"]);
    let loaded = load_tensor_with_includes(&root, tensor)?;
    validate_tensor_config(&root, &loaded, &fixture_topology(), &fixture_matrix())?;
    let evidence = loaded
        .pointer("/paradigm_contract_evidence/solver_execution/0/id")
        .and_then(Value::as_str);
    let result = if evidence == Some("included-contract") {
        Ok(())
    } else {
        Err("self-test evidence include was not merged".to_string())
    };
    let _ = fs::remove_dir_all(root);
    result
}

fn temp_root(prefix: &str) -> RunnerResult<PathBuf> {
    let mut root = std::env::temp_dir();
    root.push(format!(
        "{}_{}_{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("clock error: {error}"))?
            .as_nanos()
    ));
    fs::create_dir_all(&root).map_err(|error| format!("failed to create temp root: {error}"))?;
    Ok(root)
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
        "maturity_policy": {
            "solver_execution": ["execution", "contract"]
        },
        "cell_requirements": [],
        "evidence_claims": [],
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
    use super::super::{contract_evidence_for, derive_evidence_aware_gap, derive_gap};
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

    #[test]
    fn contract_evidence_is_scoped_to_the_exact_module() {
        let tensor = json!({
            "paradigm_contract_evidence": {
                "validation": [{
                    "id": "engine-validation",
                    "modules": ["engine"],
                    "files": ["docs/example.md"]
                }]
            }
        });
        assert_eq!(
            contract_evidence_for(&tensor, "engine", "validation").len(),
            1
        );
        assert!(contract_evidence_for(&tensor, "shell", "validation").is_empty());
    }
}
