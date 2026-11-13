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
    "agent-bounded-pending-cancellation-local-recovery",
    "agent-package-cleanup-contention-local-recovery",
    "agent-supervised-package-fetch-local-recovery",
    "agent-large-download-restart-local-recovery",
    "agent-orphan-download-coordination-local-recovery",
    "agent-session-initialization-local-recovery",
    "agent-initialization-staging-local-recovery",
    "agent-initialization-io-failure-local-recovery",
    "agent-session-retirement-local-recovery",
    "agent-session-lifecycle-admission-local-recovery",
    "agent-generation-partial-cleanup-local-recovery",
    "agent-abandoned-generation-retry-local-recovery",
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

#[test]
fn abandoned_retry_proof_does_not_promote_or_replace_partial_cleanup_proof() {
    let (_, mut tensor, _, _, shard) = current();
    for claim in tensor["evidence_claims"].as_array_mut().unwrap() {
        if claim["id"] == "agent-abandoned-generation-retry-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &tensor,
            &shard,
            "agent-abandoned-generation-retry-local-recovery"
        )["met"],
        false
    );
    for independent in [
        "agent-generation-partial-cleanup-local-recovery",
        "agent-session-retirement-local-recovery",
        "agent-session-lifecycle-admission-local-recovery",
    ] {
        assert_eq!(
            scope(&tensor, &shard, independent)["met"],
            true,
            "{independent}"
        );
    }
}

#[test]
fn generation_cleanup_proof_is_independent_of_session_retirement_and_admission() {
    let (_, mut tensor, _, _, shard) = current();
    for claim in tensor["evidence_claims"].as_array_mut().unwrap() {
        if claim["id"] == "agent-generation-partial-cleanup-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &tensor,
            &shard,
            "agent-generation-partial-cleanup-local-recovery"
        )["met"],
        false
    );
    for independent in [
        "agent-session-retirement-local-recovery",
        "agent-session-lifecycle-admission-local-recovery",
    ] {
        assert_eq!(
            scope(&tensor, &shard, independent)["met"],
            true,
            "{independent}"
        );
    }
}

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
        let grade = if matches!(
            *id,
            "agent-linux-variant-native-tls"
                | "agent-isolation-whole-call-performance"
                | "agent-unix-installed-isolation"
        ) {
            "verified"
        } else {
            "unassessed"
        };
        assert_eq!(evidence["achieved_grade"], grade, "{id}");
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
    let mut without_cleanup_contention = tensor.clone();
    for claim in without_cleanup_contention["evidence_claims"]
        .as_array_mut()
        .unwrap()
    {
        if claim["id"] == "agent-package-cleanup-contention-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_cleanup_contention,
            &shard,
            "agent-package-cleanup-contention-local-recovery"
        )["met"],
        false,
        "TLS and pending cancellation evidence cannot replace stalled package cleanup proof"
    );
    assert_eq!(
        scope(
            &without_cleanup_contention,
            &shard,
            "agent-bounded-pending-cancellation-local-recovery"
        )["met"],
        true
    );
    let mut without_isolated_calls = tensor.clone();
    let mut without_orphan_coordination = tensor.clone();
    let mut without_initialization = tensor.clone();
    let mut without_staging = tensor.clone();
    let mut without_io_failure = tensor.clone();
    let mut without_retirement = tensor.clone();
    let mut without_admission = tensor.clone();
    for claim in without_admission["evidence_claims"].as_array_mut().unwrap() {
        if claim["id"] == "agent-session-lifecycle-admission-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_admission,
            &shard,
            "agent-session-lifecycle-admission-local-recovery"
        )["met"],
        false,
        "startup retirement proof cannot replace normal-close and admission protection"
    );
    assert_eq!(
        scope(
            &without_admission,
            &shard,
            "agent-session-retirement-local-recovery"
        )["met"],
        true
    );
    for claim in without_retirement["evidence_claims"]
        .as_array_mut()
        .unwrap()
    {
        if claim["id"] == "agent-session-retirement-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_retirement,
            &shard,
            "agent-session-retirement-local-recovery"
        )["met"],
        false,
        "incomplete initialization and download recovery cannot prove partial completed-session removal"
    );
    assert_eq!(
        scope(
            &without_retirement,
            &shard,
            "agent-initialization-io-failure-local-recovery"
        )["met"],
        true
    );
    for claim in without_io_failure["evidence_claims"]
        .as_array_mut()
        .unwrap()
    {
        if claim["id"] == "agent-initialization-io-failure-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_io_failure,
            &shard,
            "agent-initialization-io-failure-local-recovery"
        )["met"],
        false,
        "process termination proof cannot replace short-write and permission failure qualification"
    );
    assert_eq!(
        scope(
            &without_io_failure,
            &shard,
            "agent-initialization-staging-local-recovery"
        )["met"],
        true
    );
    for claim in without_staging["evidence_claims"].as_array_mut().unwrap() {
        if claim["id"] == "agent-initialization-staging-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_staging,
            &shard,
            "agent-initialization-staging-local-recovery"
        )["met"],
        false,
        "a committed initialization record cannot prove torn staging cleanup or no-replace publication"
    );
    assert_eq!(
        scope(
            &without_staging,
            &shard,
            "agent-session-initialization-local-recovery"
        )["met"],
        true
    );
    for claim in without_initialization["evidence_claims"]
        .as_array_mut()
        .unwrap()
    {
        if claim["id"] == "agent-session-initialization-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_initialization,
            &shard,
            "agent-session-initialization-local-recovery"
        )["met"],
        false,
        "completed-session orphan recovery cannot prove partial initialization cleanup"
    );
    assert_eq!(
        scope(
            &without_initialization,
            &shard,
            "agent-orphan-download-coordination-local-recovery"
        )["met"],
        true
    );
    for claim in without_orphan_coordination["evidence_claims"]
        .as_array_mut()
        .unwrap()
    {
        if claim["id"] == "agent-orphan-download-coordination-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_orphan_coordination,
            &shard,
            "agent-orphan-download-coordination-local-recovery"
        )["met"],
        false,
        "waiting for child exit before restart cannot prove suspended-orphan protection"
    );
    assert_eq!(
        scope(
            &without_orphan_coordination,
            &shard,
            "agent-large-download-restart-local-recovery"
        )["met"],
        true
    );
    let mut without_large_download = tensor.clone();
    for claim in without_large_download["evidence_claims"]
        .as_array_mut()
        .unwrap()
    {
        if claim["id"] == "agent-large-download-restart-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_large_download,
            &shard,
            "agent-large-download-restart-local-recovery"
        )["met"],
        false,
        "partial metadata and injected files cannot prove real artifact restart recovery"
    );
    assert_eq!(
        scope(
            &without_large_download,
            &shard,
            "agent-supervised-package-fetch-local-recovery"
        )["met"],
        true
    );
    let mut without_supervised_fetch = tensor.clone();
    for claim in without_supervised_fetch["evidence_claims"]
        .as_array_mut()
        .unwrap()
    {
        if claim["id"] == "agent-supervised-package-fetch-source-regression" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_supervised_fetch,
            &shard,
            "agent-supervised-package-fetch-local-recovery"
        )["met"],
        false,
        "prompt cancellation acknowledgment alone cannot prove that download stopped"
    );
    assert_eq!(
        scope(
            &without_supervised_fetch,
            &shard,
            "agent-package-cleanup-contention-local-recovery"
        )["met"],
        true
    );
    for claim in without_isolated_calls["evidence_claims"]
        .as_array_mut()
        .unwrap()
    {
        if claim["id"] == "linux-native-isolation-whole-call-benchmark" {
            claim["status"] = json!("partial");
        }
    }
    assert_eq!(
        scope(
            &without_isolated_calls,
            &shard,
            "agent-isolation-whole-call-performance"
        )["achieved_grade"],
        "unassessed",
        "cold TLS frames cannot substitute for whole native isolated-call performance"
    );
    // Revoking the retained source test must reopen its own named obligation.
    for claim in tensor["evidence_claims"].as_array_mut().unwrap() {
        if scoped_claims.contains(&claim["id"]) {
            claim["status"] = json!("partial");
        }
    }
    for id in RETAINED {
        assert_eq!(scope(&tensor, &shard, id)["met"], false, "{id}");
    }
    assert_eq!(
        scope(&tensor, &shard, "agent-linux-variant-native-tls")["achieved_grade"],
        "unassessed",
        "TLS deployment cannot borrow verified SSH or local gateway evidence"
    );
    assert_eq!(
        scope(&tensor, &shard, "agent-isolation-whole-call-performance")["achieved_grade"],
        "unassessed",
        "whole-call isolation cannot borrow old resident-host or solver-node timing"
    );
    assert_eq!(
        scope(&tensor, &shard, "agent-unix-installed-isolation")["achieved_grade"],
        "unassessed",
        "installed recovery cannot borrow source tests or successful deployment"
    );
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
