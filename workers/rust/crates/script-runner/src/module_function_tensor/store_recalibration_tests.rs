use super::{build_tensor_report, includes, qualifications, read_json, validate_tensor_config};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const SHARD: &str = "config/architecture/module-function-coverage-evidence/current-store-native-qualification-scopes.json";

fn current() -> (PathBuf, Value, Value, Value, Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .to_path_buf();
    let tensor =
        includes::load_tensor_with_includes(&root, read_json(&root, super::TENSOR_PATH).unwrap())
            .unwrap();
    let shard = read_json(&root, SHARD).expect("current Store/native scopes must be registered");
    assert!(
        tensor["evidence_includes"]
            .as_array()
            .unwrap()
            .contains(&json!(SHARD))
    );
    let topology = read_json(&root, super::TOPOLOGY_PATH).unwrap();
    let matrix = read_json(&root, super::MATRIX_PATH).unwrap();
    validate_tensor_config(&root, &tensor, &topology, &matrix).unwrap();
    (root, tensor, topology, matrix, shard)
}

const RETAINED: &[&str] = &[
    "sdk-native-result-local-contract",
    "agent-native-result-local-recovery",
    "agent-unix-owned-process-local-recovery",
    "mac-linux-ssh-variant-deployment",
];

const OPEN: &[&str] = &[
    "workbench-real-store-account-session",
    "workbench-real-store-authorized-download",
    "installer-public-signed-store-install",
    "workbench-pwdt-store-agent-journey",
    "agent-linux-variant-native-tls",
    "agent-linux-arm64-variant-execution",
    "agent-windows-owned-process-isolation",
    "agent-unix-installed-isolation",
    "agent-isolation-whole-call-performance",
    "installer-project-operator-controlled-replacement",
    "headless-package-deployment-published-parity",
    "orchestra-store-project-package-supply",
    "installer-multiplatform-store-publication",
    "sdk-current-isolated-abi-qualification",
];

fn scope(tensor: &Value, shard: &Value, id: &str) -> Value {
    let requirement = shard["qualification_requirements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .unwrap_or_else(|| panic!("missing current obligation {id}"));
    qualifications::evaluate(
        tensor,
        requirement["module_id"].as_str().unwrap(),
        requirement["paradigm"].as_str().unwrap(),
    )
    .into_iter()
    .find(|s| s["id"] == id)
    .unwrap()
}

#[test]
fn store_recalibration_records_bounded_proof_without_promoting_whole_journeys() {
    let (_, tensor, _, _, shard) = current();
    assert_eq!(
        shard["qualification_requirements"]
            .as_array()
            .unwrap()
            .len(),
        RETAINED.len() + OPEN.len()
    );
    for id in RETAINED {
        let evidence = scope(&tensor, &shard, id);
        assert_eq!(evidence["achieved_grade"], "verified", "{id}");
        assert_eq!(evidence["target_grade"], "verified", "{id}");
        assert_eq!(evidence["met"], true, "{id}");
    }
    for id in OPEN {
        let evidence = scope(&tensor, &shard, id);
        assert_eq!(evidence["met"], false, "{id}");
        assert_eq!(evidence["achieved_grade"], "unassessed", "{id}");
    }
    for id in [
        "native-operator-result-identity-admission",
        "agent-native-operator-process-isolation",
    ] {
        let broad = tensor["evidence_claims"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        assert_eq!(broad["status"], "partial");
        assert_eq!(broad["grade"], "exercised");
    }
}

#[test]
fn store_recalibration_cannot_borrow_historical_or_other_transport_success() {
    let (_, mut tensor, _, _, shard) = current();
    let scoped_claims: Vec<_> = shard["evidence_claims"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].clone())
        .collect();
    for claim in tensor["evidence_claims"].as_array_mut().unwrap() {
        if claim["status"] == "proven" && !scoped_claims.contains(&claim["id"]) {
            claim["grade"] = json!("operational");
        }
    }
    for id in OPEN {
        assert_eq!(scope(&tensor, &shard, id)["met"], false, "{id}");
    }
    // Revoking the retained source test must reopen its own named obligation.
    for claim in tensor["evidence_claims"].as_array_mut().unwrap() {
        if scoped_claims.contains(&claim["id"]) {
            claim["status"] = json!("partial");
        }
    }
    for id in RETAINED {
        assert_eq!(scope(&tensor, &shard, id)["met"], false, "{id}");
    }
}

#[test]
fn store_recalibration_exposes_scopes_without_changing_release_policy_or_old_claims() {
    let (root, tensor, topology, matrix, shard) = current();
    let mut baseline = tensor.clone();
    for key in [
        "evidence_claims",
        "qualification_requirements",
        "cell_requirements",
    ] {
        baseline[key]
            .as_array_mut()
            .unwrap()
            .retain(|entry| !shard[key].as_array().unwrap().contains(entry));
    }
    let before = build_tensor_report(&root, &baseline, &topology, &matrix).unwrap();
    let after = build_tensor_report(&root, &tensor, &topology, &matrix).unwrap();
    assert_eq!(before["axes"], after["axes"]);
    assert_eq!(before["release_profile"], after["release_profile"]);
    assert_eq!(
        before["evidence_grade_policy"],
        after["evidence_grade_policy"]
    );
    assert_eq!(
        after["evidence_claims"].as_array().unwrap().len(),
        before["evidence_claims"].as_array().unwrap().len()
            + shard["evidence_claims"].as_array().unwrap().len()
    );
    for old in before["evidence_claims"].as_array().unwrap() {
        assert!(after["evidence_claims"].as_array().unwrap().contains(old));
    }
    for requirement in shard["qualification_requirements"].as_array().unwrap() {
        if OPEN.contains(&requirement["id"].as_str().unwrap()) {
            assert_eq!(
                after["cells"][requirement["module_id"].as_str().unwrap()]
                    [requirement["paradigm"].as_str().unwrap()]["evidence_grade"]["state"],
                "below_target"
            );
        }
    }
    assert_eq!(after["structural_ok"], true);
    assert_eq!(
        after["ok"], true,
        "advisory is not an enforced release gate"
    );
    assert_eq!(after["release_readiness"]["release_claim_allowed"], false);
    let rendered = super::markdown::render_markdown(&after);
    for id in RETAINED.iter().chain(OPEN) {
        assert!(
            rendered.contains(scope(&tensor, &shard, id)["scope"].as_str().unwrap()),
            "{id}"
        );
    }
}
