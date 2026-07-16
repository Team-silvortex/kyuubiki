use super::{
    EVIDENCE_KITS_SCHEMA_VERSION, ROADMAP_SCHEMA_VERSION, is_below_minimum_coverage_level,
    level_rank, qualification_evidence_errors, qualification_evidence_kit_errors,
    qualification_roadmap_errors,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(super) fn run_self_test() -> Vec<String> {
    let mut failures = Vec::new();
    assert_rule(
        is_below_minimum_coverage_level("baseline", "review"),
        "baseline must fail review gate",
        &mut failures,
    );
    assert_rule(
        is_below_minimum_coverage_level("smoke", "review"),
        "smoke must fail review gate",
        &mut failures,
    );
    assert_rule(
        !is_below_minimum_coverage_level("review", "review"),
        "review must satisfy review gate",
        &mut failures,
    );
    assert_rule(
        !is_below_minimum_coverage_level("qualification", "review"),
        "qualification must satisfy review gate",
        &mut failures,
    );
    assert_rule(
        level_rank("smoke") < level_rank("baseline"),
        "level order must keep smoke before baseline",
        &mut failures,
    );
    assert_rule(
        level_rank("review") < level_rank("qualification"),
        "level order must keep review before qualification",
        &mut failures,
    );

    let missing = serde_json::json!({ "evidence": {} });
    assert_contains(
        &qualification_evidence_errors(&missing),
        "qualification-level operators must declare evidence.qualification",
        "missing qualification evidence must fail",
        &mut failures,
    );
    let partial = serde_json::json!({
        "evidence": { "qualification": { "tests": ["virtual.rs"] } }
    });
    assert_contains(
        &qualification_evidence_errors(&partial),
        "evidence.qualification.validation_sources must be non-empty",
        "partial qualification evidence must fail",
        &mut failures,
    );
    let full = serde_json::json!({
        "evidence": {
            "qualification": {
                "validation_sources": ["analytic"],
                "convergence_checks": ["mesh_refinement"],
                "provenance": ["baseline_id"],
                "release_gates": ["release_blocking_regression"],
                "tests": ["virtual.rs"]
            }
        }
    });
    assert_rule(
        qualification_evidence_errors(&full).is_empty(),
        "complete qualification evidence must validate",
        &mut failures,
    );

    let manifest = serde_json::json!({ "version_line": "moxi self-test" });
    let operators = ["solve.ok", "solve.low"]
        .into_iter()
        .collect::<HashSet<_>>();
    let levels = HashMap::from([("solve.ok", "review"), ("solve.low", "baseline")]);
    let roadmap = roadmap_fixture();
    assert_rule(
        qualification_roadmap_errors(&roadmap, &manifest, &operators, &levels).is_empty(),
        "valid qualification roadmap must pass",
        &mut failures,
    );
    assert_contains(
        &qualification_roadmap_errors(
            &with_field(&roadmap, "version_line", Value::from("moxi wrong")),
            &manifest,
            &operators,
            &levels,
        ),
        "version_line must match reliability manifest",
        "roadmap version mismatch must fail",
        &mut failures,
    );
    assert_any_contains(
        &qualification_roadmap_errors(
            &with_candidate_operator_ids(&roadmap, &["solve.missing"]),
            &manifest,
            &operators,
            &levels,
        ),
        "is not in reliability manifest",
        "roadmap unknown operator must fail",
        &mut failures,
    );
    assert_any_contains(
        &qualification_roadmap_errors(
            &with_candidate_operator_ids(&roadmap, &["solve.low"]),
            &manifest,
            &operators,
            &levels,
        ),
        "is below roadmap minimum review",
        "roadmap candidate below minimum must fail",
        &mut failures,
    );
    let qualification_manifest = serde_json::json!({
        "version_line": "moxi self-test",
        "minimum_coverage_level": "qualification"
    });
    let qualified_levels =
        HashMap::from([("solve.ok", "qualification"), ("solve.low", "baseline")]);
    assert_contains(
        &qualification_roadmap_errors(
            &roadmap,
            &qualification_manifest,
            &operators,
            &qualified_levels,
        ),
        "minimum_candidate_level review is below manifest minimum qualification",
        "roadmap minimum below manifest minimum must fail",
        &mut failures,
    );

    let kits = kits_fixture();
    assert_rule(
        qualification_evidence_kit_errors(&kits, &roadmap, &manifest).is_empty(),
        "valid qualification evidence kit must pass",
        &mut failures,
    );
    assert_any_contains(
        &qualification_evidence_kit_errors(
            &with_kit_candidate(&kits, "missing"),
            &roadmap,
            &manifest,
        ),
        "is not in qualification roadmap",
        "qualification evidence kit without roadmap candidate must fail",
        &mut failures,
    );
    assert_any_contains(
        &qualification_evidence_kit_errors(
            &collecting_without_artifact_entry(&kits),
            &roadmap,
            &manifest,
        ),
        "collecting kits must declare artifact_path or artifact_command",
        "collecting qualification evidence kit without artifact entry must fail",
        &mut failures,
    );
    assert_any_contains(
        &qualification_evidence_kit_errors(
            &with_kit_operator_ids(&kits, &["solve.low"]),
            &roadmap,
            &manifest,
        ),
        "is not in the roadmap candidate",
        "qualification evidence kit with out-of-candidate operator must fail",
        &mut failures,
    );
    assert_any_contains(
        &qualification_evidence_kit_errors(&with_kit_operator_ids(&kits, &[]), &roadmap, &manifest),
        "missing roadmap operator_id solve.ok",
        "qualification evidence kit missing roadmap operator must fail",
        &mut failures,
    );
    assert_any_contains(
        &qualification_evidence_kit_errors(
            &with_kit_operator_ids(&kits, &["solve.ok", "solve.ok"]),
            &roadmap,
            &manifest,
        ),
        "duplicate operator_id solve.ok",
        "qualification evidence kit duplicate operator must fail",
        &mut failures,
    );
    assert_any_contains(
        &qualification_evidence_kit_errors(
            &duplicate_artifact_requirement(&kits),
            &roadmap,
            &manifest,
        ),
        "duplicate artifact_id reference",
        "qualification evidence kit duplicate artifact must fail",
        &mut failures,
    );
    assert_any_contains(
        &qualification_evidence_kit_errors(&absolute_artifact_path(&kits), &roadmap, &manifest),
        "artifact_path must be repo-relative",
        "qualification evidence kit absolute artifact path must fail",
        &mut failures,
    );
    assert_contains(
        &qualification_evidence_kit_errors(
            &with_field(&kits, "kits", Value::Array(Vec::new())),
            &roadmap,
            &manifest,
        ),
        "kits must be non-empty",
        "empty qualification evidence kits must fail",
        &mut failures,
    );
    failures
}

fn roadmap_fixture() -> Value {
    serde_json::json!({
        "schema_version": ROADMAP_SCHEMA_VERSION,
        "version_line": "moxi self-test",
        "minimum_candidate_level": "review",
        "candidates": [{
            "candidate_id": "self-test",
            "priority": "p0",
            "domain": "self_test",
            "operator_ids": ["solve.ok"],
            "rationale": "self-test candidate",
            "evidence_gaps": ["gap"],
            "required_artifacts": ["artifact"],
            "graduation_gate": "gate"
        }]
    })
}

fn kits_fixture() -> Value {
    serde_json::json!({
        "schema_version": EVIDENCE_KITS_SCHEMA_VERSION,
        "version_line": "moxi self-test",
        "kits": [{
            "candidate_id": "self-test",
            "status": "planned",
            "artifact_profile": "analytic",
            "operator_ids": ["solve.ok"],
            "artifact_requirements": [{
                "artifact_id": "reference",
                "kind": "note",
                "gate": "gate",
                "artifact_path": "docs/reference.md",
                "path_policy": "repo-relative-required-before-qualification"
            }]
        }]
    })
}

fn with_field(value: &Value, field_name: &str, field_value: Value) -> Value {
    let mut updated = value.clone();
    updated[field_name] = field_value;
    updated
}

fn with_candidate_operator_ids(roadmap: &Value, operator_ids: &[&str]) -> Value {
    let mut updated = roadmap.clone();
    updated["candidates"][0]["operator_ids"] = strings(operator_ids);
    updated
}

fn with_kit_candidate(kits: &Value, candidate_id: &str) -> Value {
    let mut updated = kits.clone();
    updated["kits"][0]["candidate_id"] = Value::from(candidate_id);
    updated
}

fn with_kit_operator_ids(kits: &Value, operator_ids: &[&str]) -> Value {
    let mut updated = kits.clone();
    updated["kits"][0]["operator_ids"] = strings(operator_ids);
    updated
}

fn collecting_without_artifact_entry(kits: &Value) -> Value {
    let mut updated = kits.clone();
    updated["kits"][0]["status"] = Value::from("collecting");
    updated["kits"][0]["artifact_requirements"][0]
        .as_object_mut()
        .map(|object| {
            object.remove("artifact_path");
            object.remove("artifact_command");
        });
    updated
}

fn duplicate_artifact_requirement(kits: &Value) -> Value {
    let mut updated = kits.clone();
    let item = updated["kits"][0]["artifact_requirements"][0].clone();
    updated["kits"][0]["artifact_requirements"] = Value::Array(vec![item.clone(), item]);
    updated
}

fn absolute_artifact_path(kits: &Value) -> Value {
    let mut updated = kits.clone();
    updated["kits"][0]["artifact_requirements"][0]["artifact_path"] =
        Value::from("/tmp/reference.md");
    updated
}

fn strings(values: &[&str]) -> Value {
    Value::Array(values.iter().map(|value| Value::from(*value)).collect())
}

fn assert_rule(condition: bool, message: &str, failures: &mut Vec<String>) {
    if !condition {
        failures.push(message.to_string());
    }
}

fn assert_contains(errors: &[String], expected: &str, message: &str, failures: &mut Vec<String>) {
    if !errors.iter().any(|error| error == expected) {
        failures.push(message.to_string());
    }
}

fn assert_any_contains(
    errors: &[String],
    expected: &str,
    message: &str,
    failures: &mut Vec<String>,
) {
    if !errors.iter().any(|error| error.contains(expected)) {
        failures.push(message.to_string());
    }
}
