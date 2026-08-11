use super::{
    FuzzProfileReport, ProtocolTestReport, QualificationContract, RpcReport, RunnerResult,
    TaskIrReport,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const RPC_REQUEST_BOUNDARY: &str = "tests::core::validates_rpc_request_envelope_boundaries";
const RPC_STATE_BOUNDARY: &str = "tests::core::validates_rpc_response_and_progress_envelope_states";
const SOLVER_ACCEPT_BOUNDARY: &str = "tests::solver_execution_capability::solver_execution_capability_accepts_agent_builtin_solver_task";
const SOLVER_DENY_BOUNDARY: &str = "tests::solver_execution_capability::solver_execution_capability_does_not_admit_operator_task_programs";

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct SecurityQualificationReport {
    boundary_count: usize,
    passed_boundary_count: usize,
    total_fuzz_cases: usize,
    digest_tamper_rejections: usize,
    structured_rejection_code_count: usize,
    fail_closed: bool,
    boundaries: Vec<SecurityBoundaryReport>,
}

#[derive(Clone, Deserialize, Serialize)]
struct SecurityBoundaryReport {
    id: String,
    passed: bool,
    evidence: Vec<String>,
}

pub(super) fn build(
    contract: &QualificationContract,
    protocol: &ProtocolTestReport,
    fuzz: &[FuzzProfileReport],
    task_ir: &TaskIrReport,
    rpc: &RpcReport,
) -> SecurityQualificationReport {
    let fuzz_passed = |id: &str| {
        fuzz.iter()
            .any(|profile| profile.id == id && profile.passed)
    };
    let test_passed = |id: &str| {
        protocol
            .required_tests
            .iter()
            .any(|test| test.id == id && test.passed)
    };
    let boundaries = vec![
        boundary(
            "rpc-envelope-state",
            test_passed(RPC_REQUEST_BOUNDARY)
                && test_passed(RPC_STATE_BOUNDARY)
                && rpc.boundary_rejection_codes.len() >= 4,
            &[RPC_REQUEST_BOUNDARY, RPC_STATE_BOUNDARY],
        ),
        boundary(
            "rpc-unknown-method-admission",
            rpc.unknown_method_rejected,
            &["unknown RPC method rejected before dispatch"],
        ),
        boundary(
            "rpc-mutated-json",
            fuzz_passed("rpc-json"),
            &["rpc-json deterministic fuzz profile"],
        ),
        boundary(
            "rpc-byte-ingress",
            fuzz_passed("rpc-bytes"),
            &["rpc-bytes deterministic fuzz profile"],
        ),
        boundary(
            "task-ir-digest-tamper",
            task_ir.task_count >= contract.minimum_task_ir_count
                && task_ir.tamper_rejection_count == task_ir.task_count,
            &["every retained TaskIR fixture rejects digest tampering"],
        ),
        boundary(
            "task-ir-structural-admission",
            task_ir.structured_rejection_codes.len() >= 4,
            &["TaskIR mirrors, ABI, program, and entrypoint reject mismatches"],
        ),
        boundary(
            "task-ir-mutated-json",
            fuzz_passed("task-ir-json"),
            &["task-ir-json deterministic fuzz profile"],
        ),
        boundary(
            "task-ir-byte-ingress",
            fuzz_passed("task-ir-bytes"),
            &["task-ir-bytes deterministic fuzz profile"],
        ),
        boundary(
            "solver-capability-admission",
            test_passed(SOLVER_ACCEPT_BOUNDARY) && test_passed(SOLVER_DENY_BOUNDARY),
            &[SOLVER_ACCEPT_BOUNDARY, SOLVER_DENY_BOUNDARY],
        ),
    ];
    let passed_boundary_count = boundaries.iter().filter(|entry| entry.passed).count();
    SecurityQualificationReport {
        boundary_count: boundaries.len(),
        passed_boundary_count,
        total_fuzz_cases: fuzz.iter().map(|profile| profile.cases).sum(),
        digest_tamper_rejections: task_ir.tamper_rejection_count,
        structured_rejection_code_count: task_ir.structured_rejection_codes.len(),
        fail_closed: passed_boundary_count == boundaries.len()
            && boundaries.len() >= contract.minimum_security_boundaries,
        boundaries,
    }
}

pub(super) fn passed(report: &SecurityQualificationReport) -> bool {
    report.fail_closed
}

pub(super) fn validate(
    contract: &QualificationContract,
    report: &SecurityQualificationReport,
) -> RunnerResult<()> {
    let expected = contract
        .required_security_boundaries
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let actual = report
        .boundaries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected_fuzz_cases = contract
        .fuzz_profiles
        .iter()
        .map(|profile| profile.cases)
        .sum::<usize>();
    if expected != actual
        || report.boundary_count != contract.required_security_boundaries.len()
        || report.boundary_count < contract.minimum_security_boundaries
        || report.passed_boundary_count != report.boundary_count
        || report.boundaries.iter().any(|entry| {
            !entry.passed
                || entry.evidence.is_empty()
                || entry.evidence.iter().any(String::is_empty)
        })
        || report.total_fuzz_cases != expected_fuzz_cases
        || report.digest_tamper_rejections < contract.minimum_task_ir_count
        || report.structured_rejection_code_count < 4
        || !report.fail_closed
    {
        return Err("protocol security qualification evidence is incomplete".to_string());
    }
    Ok(())
}

pub(super) fn run_self_test() -> RunnerResult<()> {
    let required = [
        "rpc-envelope-state",
        "rpc-unknown-method-admission",
        "rpc-mutated-json",
        "rpc-byte-ingress",
        "task-ir-digest-tamper",
        "task-ir-structural-admission",
        "task-ir-mutated-json",
        "task-ir-byte-ingress",
        "solver-capability-admission",
    ];
    let boundaries = required
        .iter()
        .map(|id| boundary(id, true, &["self-test evidence"]))
        .collect::<Vec<_>>();
    let report = SecurityQualificationReport {
        boundary_count: boundaries.len(),
        passed_boundary_count: boundaries.len(),
        total_fuzz_cases: 1_280,
        digest_tamper_rejections: 5,
        structured_rejection_code_count: 4,
        fail_closed: true,
        boundaries,
    };
    if !passed(&report)
        || report
            .boundaries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<BTreeSet<_>>()
            != required.into_iter().collect::<BTreeSet<_>>()
    {
        return Err("protocol security qualification self-test failed".to_string());
    }
    let mut tampered = report;
    tampered.boundaries[0].passed = false;
    if tampered.boundaries.iter().all(|entry| entry.passed) {
        return Err("protocol security qualification tamper self-test failed".to_string());
    }
    Ok(())
}

fn boundary(id: &str, passed: bool, evidence: &[&str]) -> SecurityBoundaryReport {
    SecurityBoundaryReport {
        id: id.to_string(),
        passed,
        evidence: evidence.iter().map(|value| (*value).to_string()).collect(),
    }
}
