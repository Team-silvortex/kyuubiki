use super::self_test::{fixture_matrix, fixture_tensor, fixture_topology};
use super::{CellEvaluationInput, build_tensor_report, evidence_grade, maturity, qualifications};
use serde_json::{Value, json};
use std::path::Path;

fn claim(id: &str, grade: &str, dimensions: &[&str]) -> Value {
    json!({"id": id, "grade": grade, "status": "proven", "modules": ["engine"],
        "paradigms": ["solver_execution"], "dimensions": dimensions})
}

fn requirement() -> Value {
    json!({"id": "windows-recovery", "module_id": "engine", "paradigm": "solver_execution",
        "dimension": "execution", "scope": "windows/installed/recovery", "target": "qualified",
        "claims": [], "basis": ["Cargo.toml"], "acceptance": "Retain the actual installed Windows journey"})
}

fn grade(tensor: &Value, status: &str) -> Value {
    evidence_grade::evaluate(&CellEvaluationInput {
        tensor,
        module_id: "engine",
        paradigm: "solver_execution",
        status,
        required: true,
        benchmark_tests: &[],
        security_tests: &[],
        contract_evidence: &[],
    })
}

fn report(tensor: &Value, status: &str) -> Value {
    let mut matrix = fixture_matrix();
    matrix["cells"]["engine"]["solver_execution"] = json!(status);
    build_tensor_report(Path::new("."), tensor, &fixture_topology(), &matrix).unwrap()
}

#[test]
fn registered_lanes_cannot_prove_execution_security_or_benchmarks() {
    let mut tensor = fixture_tensor();
    tensor["maturity_policy"]["solver_execution"] = json!(["execution", "security", "benchmark"]);
    let commands = [json!({"id": "only-registered"})];
    let input = CellEvaluationInput {
        tensor: &tensor,
        module_id: "engine",
        paradigm: "solver_execution",
        status: "covered",
        required: true,
        benchmark_tests: &commands,
        security_tests: &commands,
        contract_evidence: &[],
    };
    assert_eq!(maturity::evaluate(&input)["present_dimensions"], json!([]));
    assert_eq!(
        evidence_grade::evaluate(&input)["achieved_grade"],
        "declared"
    );
    assert_eq!(
        evidence_grade::evaluate(&input)["registered_test_command_count"],
        2
    );
}

#[test]
fn minimum_required_dimension_wins_over_one_operational_claim() {
    let mut tensor = fixture_tensor();
    tensor["evidence_claims"] = json!([
        claim("installed-run", "operational", &["execution"]),
        claim("verified-contract", "verified", &["contract"]),
    ]);
    let evaluated = grade(&tensor, "covered");
    assert_eq!(evaluated["best_available_grade"], "operational");
    assert_eq!(evaluated["achieved_grade"], "verified");
    assert_eq!(evaluated["gap_steps"], 1);
}

#[test]
fn unrelated_dimensions_and_modules_cannot_promote_a_required_dimension() {
    let mut tensor = fixture_tensor();
    let mut other = claim("other-module", "operational", &["execution", "contract"]);
    other["modules"] = json!(["different-engine"]);
    tensor["evidence_claims"] =
        json!([other, claim("only-recovery", "operational", &["recovery"])]);
    assert_eq!(grade(&tensor, "covered")["achieved_grade"], "declared");
}

#[test]
fn open_scope_blocks_even_when_all_dimensions_exceed_target() {
    let mut tensor = fixture_tensor();
    tensor["evidence_claims"] = json!([claim(
        "linux-installed",
        "operational",
        &["execution", "contract"]
    )]);
    tensor["qualification_requirements"] = json!([requirement()]);
    let evaluated = grade(&tensor, "covered");
    assert_eq!(evaluated["achieved_grade"], "operational");
    assert_eq!(evaluated["gap_steps"], 0);
    assert_eq!(evaluated["state"], "below_target");
    let report = report(&tensor, "covered");
    assert_eq!(report["evidence_grade_calibration"]["target_met_count"], 0);
    assert_eq!(
        report["evidence_grade_calibration"]["scope_requirement_gap_count"],
        1
    );
    assert_eq!(report["release_readiness"]["p0_coordinate_gap_count"], 1);
    assert_eq!(
        report["release_readiness"]["planning_queue"][0]["qualification_gaps"][0]["scope"],
        "windows/installed/recovery"
    );
    assert_eq!(report["release_readiness"]["release_claim_allowed"], false);
    assert_eq!(
        report["ok"], true,
        "advisory structure checks remain usable while evidence is missing"
    );
    tensor["release_profile"]["gate_mode"] = json!("enforced");
    assert_eq!(self::report(&tensor, "covered")["ok"], false);
}

#[test]
fn partial_capability_cannot_be_counted_as_target_met_with_perfect_evidence() {
    let mut tensor = fixture_tensor();
    tensor["evidence_claims"] = json!([claim(
        "deep-proof",
        "operational",
        &["execution", "contract"]
    )]);
    let report = report(&tensor, "partial");
    assert_eq!(
        report["cells"]["engine"]["solver_execution"]["evidence_grade"]["gap_steps"],
        0
    );
    assert_eq!(report["evidence_grade_calibration"]["target_met_count"], 0);
    assert_eq!(
        report["release_readiness"]["criticality_summary"]["p0"]["target_met_count"],
        0
    );
    assert_eq!(report["release_readiness"]["coordinate_gap_count"], 1);
}

#[test]
fn rounded_percent_cannot_hide_an_open_noncritical_coordinate() {
    let mut tensor = fixture_tensor();
    tensor["release_profile"]["paradigm_criticality"]["solver_execution"] = json!("p1");
    let cells = serde_json::Map::from_iter((0..2500).map(|index| {
        (
            format!("module-{index}"),
            json!({"solver_execution": {"required": true, "evidence_grade": {
            "state": if index == 0 { "below_target" } else { "target_met" }, "gap_steps": 0}}}),
        )
    }));
    let readiness = super::release_readiness::evaluate(
        Path::new("."),
        &tensor,
        &cells,
        true,
        true,
        &json!({"target_met_percent": 100.0, "gaps": []}),
    )
    .unwrap();
    assert_eq!(readiness["p0_coordinate_gap_count"], 0);
    assert_eq!(readiness["criticality_summary"]["p1"]["gap_count"], 1);
    assert_eq!(readiness["release_claim_allowed"], false);
}

#[test]
fn readable_report_keeps_open_scope_and_assessment_limits_visible() {
    let mut tensor = fixture_tensor();
    tensor["evidence_claims"] = json!([claim(
        "deep-proof",
        "operational",
        &["execution", "contract"]
    )]);
    tensor["qualification_requirements"] = json!([requirement()]);
    let rendered = super::markdown::render_markdown(&report(&tensor, "covered"));
    for text in [
        "not a test or product coverage percentage",
        "not fresh test execution",
        "windows/installed/recovery",
        "unassessed",
        "Retain the actual installed Windows journey",
    ] {
        assert!(rendered.contains(text), "missing {text}");
    }
}

#[test]
fn named_proven_scope_can_close_without_broadcasting_to_another_scope() {
    let mut tensor = fixture_tensor();
    tensor["evidence_claims"] = json!([claim(
        "actual-windows",
        "qualified",
        &["execution", "contract"]
    )]);
    let mut named = requirement();
    named["claims"] = json!(["actual-windows"]);
    let mut another = requirement();
    another["id"] = json!("different-backend");
    another["scope"] = json!("windows/postgresql/revision-2");
    tensor["qualification_requirements"] = json!([named, another]);
    let requirements = qualifications::evaluate(&tensor, "engine", "solver_execution");
    assert_eq!(requirements[0]["met"], true);
    assert_eq!(requirements[1]["met"], false);
    tensor["qualification_requirements"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_eq!(grade(&tensor, "covered")["state"], "target_met");
    tensor["evidence_claims"][0]["status"] = json!("partial");
    assert_eq!(
        grade(&tensor, "covered")["qualification_requirements"][0]["met"],
        false
    );
}

#[test]
fn malformed_or_cross_coordinate_scope_bindings_fail_closed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut tensor = fixture_tensor();
    tensor["evidence_claims"] = json!([claim(
        "retained-run",
        "qualified",
        &["execution", "contract"]
    )]);
    tensor["qualification_requirements"] = json!([requirement()]);
    qualifications::validate_config(root, &tensor, &fixture_matrix()).unwrap();
    for (key, value) in [
        ("module_id", json!("unknown")),
        ("paradigm", json!("unknown")),
        ("dimension", json!("recovery")),
        ("target", json!("unassessed")),
        ("claims", json!(["missing-proof"])),
        ("claims", json!("not-an-array")),
        ("claims", json!(["retained-run", "retained-run"])),
        ("basis", json!([])),
        ("basis", json!(["../Cargo.toml"])),
        ("acceptance", json!(" ")),
    ] {
        let mut invalid = tensor.clone();
        invalid["qualification_requirements"][0][key] = value;
        assert!(
            qualifications::validate_config(root, &invalid, &fixture_matrix()).is_err(),
            "{key}"
        );
    }
    tensor["qualification_requirements"][0]["claims"] = json!(["retained-run"]);
    tensor["evidence_claims"][0]["dimensions"] = json!(["contract"]);
    assert!(qualifications::validate_config(root, &tensor, &fixture_matrix()).is_err());
    tensor["qualification_requirements"] = json!([requirement(), requirement()]);
    assert!(qualifications::validate_config(root, &tensor, &fixture_matrix()).is_err());
}
