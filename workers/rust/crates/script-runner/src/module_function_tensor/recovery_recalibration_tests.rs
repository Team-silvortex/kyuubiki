use super::{build_tensor_report, includes, qualifications, read_json, validate_tensor_config};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn current() -> (PathBuf, Value, Value, Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .to_path_buf();
    let tensor =
        includes::load_tensor_with_includes(&root, read_json(&root, super::TENSOR_PATH).unwrap())
            .unwrap();
    let topology = read_json(&root, super::TOPOLOGY_PATH).unwrap();
    let matrix = read_json(&root, super::MATRIX_PATH).unwrap();
    validate_tensor_config(&root, &tensor, &topology, &matrix).unwrap();
    (root, tensor, topology, matrix)
}

const OPEN_SCOPES: &[(&str, &str, &str)] = &[
    (
        "runtime-agent-cli",
        "security",
        "native-agent-lan-ingress-authorization",
    ),
    (
        "runtime-agent-cli",
        "runtime_api",
        "mesh-authenticated-multihost-recovery",
    ),
    (
        "workbench-shell",
        "persistence_provenance",
        "workbench-installed-direct-owner-recovery",
    ),
    (
        "runtime-agent-cli",
        "solver_execution",
        "agent-durable-ledger-host-power-loss",
    ),
    (
        "orchestra-control-plane",
        "workflow_composition",
        "orchestra-installed-typed-workflow-recovery",
    ),
    (
        "runtime-agent-cli",
        "benchmark",
        "mesh-sustained-ingress-and-recovery-budget",
    ),
];

#[test]
fn recovery_recalibration_keeps_each_control_mode_and_local_surface_scoped() {
    let (_, tensor, _, _) = current();
    for (module, paradigm, id, expected) in [
        (
            "runtime-agent-cli",
            "runtime_api",
            "orchestra-retained-agent-control-rejoin",
            "operational",
        ),
        (
            "runtime-agent-cli",
            "solver_execution",
            "standalone-agent-local-process-crash-recovery",
            "verified",
        ),
        (
            "runtime-agent-cli",
            "runtime_api",
            "mesh-local-discovery-owner-isolation",
            "verified",
        ),
        (
            "workbench-shell",
            "persistence_provenance",
            "workbench-local-direct-caller-recovery",
            "verified",
        ),
        (
            "workbench-shell",
            "persistence_provenance",
            "workbench-local-checkpoint-reservation-boundaries",
            "verified",
        ),
        (
            "workbench-shell",
            "product_surface",
            "desktop-local-journal-session-fencing",
            "verified",
        ),
        (
            "orchestra-control-plane",
            "validation",
            "orchestra-source-typed-postgres-continuation",
            "verified",
        ),
    ] {
        let scoped = qualifications::evaluate(&tensor, module, paradigm);
        let scope = scoped.iter().find(|scope| scope["id"] == id).unwrap();
        assert_eq!(scope["achieved_grade"], expected, "{id}");
        assert_eq!(scope["met"], true, "{id}");
    }
}

#[test]
fn unrelated_operational_claims_cannot_close_current_recovery_obligations() {
    let (_, mut tensor, _, _) = current();
    // Even optimistic unrelated claims cannot fill an empty exact-scope binding.
    for claim in tensor["evidence_claims"].as_array_mut().unwrap() {
        claim["grade"] = json!("operational");
    }
    for (module, paradigm, id) in OPEN_SCOPES {
        let scoped = qualifications::evaluate(&tensor, module, paradigm);
        let scope = scoped.iter().find(|scope| scope["id"] == *id).unwrap();
        assert_eq!(scope["target_grade"], "qualified", "{id}");
        assert_eq!(scope["claims"], json!([]), "{id}");
        assert_eq!(scope["achieved_grade"], "unassessed", "{id}");
        assert_eq!(scope["met"], false, "{id}");
    }
}

#[test]
fn local_desktop_evidence_repairs_mapping_without_promoting_installed_release() {
    let (root, tensor, topology, matrix) = current();
    let claim = tensor["evidence_claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|claim| claim["id"] == "desktop-local-call-chain-and-layout-verification")
        .unwrap();
    assert_eq!(claim["grade"], "verified");
    assert_eq!(claim["paradigms"], json!(["product_surface"]));
    let report = build_tensor_report(&root, &tensor, &topology, &matrix).unwrap();
    for module in [
        "hub-shell",
        "installer-shell",
        "workbench-shell",
        "desktop-shared-ui",
    ] {
        let cell = &report["cells"][module]["product_surface"];
        assert_eq!(
            cell["evidence_grade"]["achieved_grade"], "verified",
            "{module}"
        );
        if module == "desktop-shared-ui" {
            assert_eq!(cell["evidence_grade"]["state"], "target_met");
        } else {
            assert_eq!(cell["evidence_grade"]["state"], "below_target");
            assert!(
                cell["evidence_grade"]["qualification_requirements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(
                        |scope| scope["scope"].as_str().unwrap().starts_with("windows/")
                            && scope["met"] == false
                    )
            );
        }
    }
    for (module, paradigm, _) in OPEN_SCOPES {
        assert_eq!(
            report["cells"][*module][*paradigm]["evidence_grade"]["state"],
            "below_target"
        );
    }
    assert_eq!(report["structural_ok"], true);
    assert_eq!(report["release_readiness"]["release_claim_allowed"], false);
    assert_eq!(report["release_readiness"]["gate_mode"], "advisory");
    assert_eq!(report["ok"], true);
}
