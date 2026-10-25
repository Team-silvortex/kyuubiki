#[path = "../examples/durable_modal_research/checks.rs"]
mod checks;
#[path = "support/modal_metadata_faults.rs"]
mod metadata_faults;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::DurableExecutionHandle;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use support::{Agent, Result};

fn read(root: &Path, name: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(root.join(name))?)?)
}

fn write(root: &Path, name: &str, value: &Value) -> Result<()> {
    fs::write(root.join(name), serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let relative = path.strip_prefix(root)?.to_owned();
            if path.is_dir() {
                files.insert(relative, vec![]);
                walk(root, &path, files)?;
            } else {
                files.insert(relative, fs::read(path)?);
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files)?;
    Ok(files)
}

struct Study {
    binary: PathBuf,
    root: PathBuf,
}

impl Study {
    fn prepare(agent: &Agent, name: &str, id: &str) -> Result<Self> {
        let binary = fs::canonicalize(
            env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY")
                .ok_or("SDK example binary required")?,
        )?;
        let root = agent.root.join(name);
        fs::create_dir(&root)?;
        let study = Self { binary, root };
        let expected_store = agent.store_id()?;
        let invoke = |verb: &str, last: &Path| {
            Command::new(&study.binary)
                .arg(verb)
                .arg(study.root.join("submission"))
                .arg(last)
                .env(
                    "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
                    format!("127.0.0.1:{}", agent.port),
                )
                .output()
        };
        let submitted = invoke("submit", Path::new(id))?;
        assert_success(&submitted);
        let admission = read(&study.root, "submission/admission.json")?;
        let handle = DurableExecutionHandle::from_report(&admission)?;
        assert_eq!(handle.store_id, expected_store);
        agent.wait(&handle, "completed")?;
        assert_success(&invoke("inspect", &study.root.join("export"))?);
        Ok(study)
    }

    fn copy(&self, target: PathBuf) -> Result<Self> {
        fs::create_dir(&target)?;
        for (relative, bytes) in snapshot(&self.root)? {
            if self.root.join(&relative).is_dir() {
                fs::create_dir(target.join(relative))?;
            } else {
                fs::write(target.join(relative), bytes)?;
            }
        }
        Ok(Self {
            binary: self.binary.clone(),
            root: target,
        })
    }

    fn verify(&self) -> Result<Output> {
        Ok(Command::new(&self.binary)
            .arg("verify")
            .arg(self.root.join("submission"))
            .arg(self.root.join("export"))
            // An offline reader must not even parse runtime connection configuration.
            .env(
                "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
                "offline-must-not-connect",
            )
            .output()?)
    }

    fn verify_unchanged(&self, succeeds: bool) -> Result<Output> {
        let before = snapshot(&self.root)?;
        let output = self.verify()?;
        assert_eq!(
            output.status.success(),
            succeeds,
            "{}: {}",
            self.root.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            snapshot(&self.root)?,
            before,
            "verification must be read-only"
        );
        Ok(output)
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn offline_verification_rechecks_original_export_without_agent_or_writes() -> Result<()> {
    let mut agent = Agent::start()?;
    let study = Study::prepare(&agent, "original", &"a".repeat(32))?;
    agent.kill()?;
    let output = study.verify_unchanged(true)?;
    let fresh: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(fresh["passed"], true);
    assert_eq!(fresh["verification_source"], "offline_recomputed");
    assert_eq!(fresh["historical_diagnostics_match"], true);
    let saved = read(&study.root, "export/verified/verification.json")?;
    assert_eq!(fresh["execution"], saved["execution"]);
    assert_eq!(fresh["result_sha256"], saved["result_sha256"]);

    let missing_ack = study.copy(agent.root.join("missing-ack"))?;
    fs::remove_file(missing_ack.root.join("submission/admission.json"))?;
    missing_ack.verify_unchanged(true)?;

    let historical = study.copy(agent.root.join("historical-diagnostics"))?;
    let mut old = saved;
    old["independent_roots"] = json!(vec![999.0; 4]);
    old["physical_residuals"] = json!(vec![0.0; 4]);
    write(&historical.root, "export/verified/verification.json", &old)?;
    let output = historical.verify_unchanged(true)?;
    let recomputed: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(recomputed["independent_roots"], fresh["independent_roots"]);
    assert_eq!(
        recomputed["physical_residuals"],
        fresh["physical_residuals"]
    );
    assert_ne!(recomputed["independent_roots"], old["independent_roots"]);
    assert_eq!(recomputed["historical_diagnostics_match"], false);

    agent.restart()?;
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        1
    );
    assert_eq!(
        agent.client().describe_agent()?.result["lifecycle"]["active_execution_count"],
        0
    );
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn conflicting_terminal_admission_blocks_online_and_offline_research() -> Result<()> {
    let agent = Agent::start()?;
    let study = Study::prepare(&agent, "terminal-evidence", &"e".repeat(32))?;
    let completed = read(&study.root, "export/receipt.json")?;
    let handle = DurableExecutionHandle::from_report(&completed)?;
    let input = read(&study.root, "submission/input.json")?;
    for fault in ["failed", "changed-mode-sign"] {
        let case = study.copy(agent.root.join(fault))?;
        let mut retained = completed.clone();
        if fault == "failed" {
            retained["status"] = json!("failed");
            retained["response"] = json!({"rpc_version":1, "id":handle.request_id,
                "ok":false, "result":null, "error":{"code":"cancelled", "message":"retained failure"}});
        } else {
            for entry in retained["response"]["result"]["modes"][0]["shape"]
                .as_array_mut()
                .unwrap()
            {
                *entry = json!(-entry.as_f64().unwrap());
            }
            checks::verify(&input, &retained["response"]["result"])?;
        }
        assert_eq!(DurableExecutionHandle::from_report(&retained)?, handle);
        write(&case.root, "submission/admission.json", &retained)?;
        let rejected = case.verify_unchanged(false)?;
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("terminal observation"));
        assert!(
            String::from_utf8_lossy(&rejected.stderr).contains("admission.json / receipt.json")
        );
        let output = case.root.join("new-export");
        let inspected = Command::new(&case.binary)
            .arg("inspect")
            .arg(case.root.join("submission"))
            .arg(&output)
            .env(
                "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
                format!("127.0.0.1:{}", agent.port),
            )
            .output()?;
        assert!(
            !inspected.status.success(),
            "conflicting admission must block publication"
        );
        assert!(String::from_utf8_lossy(&inspected.stderr).contains("terminal observation"));
        assert!(!output.join("verified").exists());
        assert_eq!(read(&output, "receipt.json")?, completed);
        assert_eq!(read(&case.root, "submission/admission.json")?, retained);
    }
    for state in ["completed", "outcome_unknown"] {
        let case = study.copy(agent.root.join(format!("consistent-{state}")))?;
        let mut retained = completed.clone();
        if state == "outcome_unknown" {
            retained["status"] = json!(state);
            retained["response"] = Value::Null;
        }
        write(&case.root, "submission/admission.json", &retained)?;
        case.verify_unchanged(true)?;
    }
    assert_eq!(
        agent.client().fetch_durable_agent_execution(&handle)?,
        completed
    );
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        1
    );
    study.verify_unchanged(true)?;
    Ok(())
}

#[cfg(unix)]
#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn offline_verification_never_downgrades_a_dangling_admission_to_missing() -> Result<()> {
    let mut agent = Agent::start()?;
    let study = Study::prepare(&agent, "dangling-admission", &"c".repeat(32))?;
    agent.kill()?;
    let before = fs::read(study.root.join("export/verified/result.json"))?;
    let admission = study.root.join("submission/admission.json");
    fs::remove_file(&admission)?;
    std::os::unix::fs::symlink("absent-original-ack.json", &admission)?;
    let output = study.verify()?;
    assert!(
        !output.status.success(),
        "a present broken admission must block verification"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("admission.json"));
    assert!(fs::symlink_metadata(&admission)?.file_type().is_symlink());
    assert!(
        !study
            .root
            .join("submission/absent-original-ack.json")
            .exists()
    );
    assert_eq!(
        fs::read(study.root.join("export/verified/result.json"))?,
        before
    );
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn exported_result_must_preserve_original_numeric_values_not_only_physics() -> Result<()> {
    let mut agent = Agent::start()?;
    let study = Study::prepare(&agent, "numeric-evidence", &"f".repeat(32))?;
    agent.kill()?;
    let original = read(&study.root, "export/verified/result.json")?;
    let input = read(&study.root, "submission/input.json")?;
    for fault in ["signed-zero", "one-ulp", "integer-for-float"] {
        let case = study.copy(agent.root.join(fault))?;
        let mut result = original.clone();
        match fault {
            "signed-zero" => {
                let entry = &mut result["modes"][0]["shape"][0];
                let zero = entry.as_f64().unwrap();
                assert_eq!(zero, 0.0, "the fixed DOF must be zero");
                *entry = json!(-zero);
                assert_eq!(result, original, "ordinary equality loses zero's sign");
            }
            "one-ulp" => {
                let entry = &mut result["modes"][0]["natural_frequency_hz"];
                *entry = json!(f64::from_bits(entry.as_f64().unwrap().to_bits() + 1));
            }
            "integer-for-float" => {
                assert!(result["total_mass"].is_f64());
                result["total_mass"] = json!(21);
            }
            _ => unreachable!(),
        }
        // Physical tolerance is not evidence identity. All copies still satisfy
        // the independent reference, and their byte hashes are internally valid.
        checks::verify(&input, &result)?;
        write(&case.root, "export/verified/result.json", &result)?;
        let mut saved = read(&case.root, "export/verified/verification.json")?;
        saved["result_sha256"] = json!(format!(
            "{:x}",
            Sha256::digest(fs::read(case.root.join("export/verified/result.json"))?)
        ));
        write(&case.root, "export/verified/verification.json", &saved)?;
        let output = case.verify_unchanged(false)?;
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("original completed receipt"),
            "{fault}: {error}"
        );
        assert_eq!(
            fs::read(case.root.join("export/receipt.json"))?,
            fs::read(study.root.join("export/receipt.json"))?
        );
    }
    let reformatted = study.copy(agent.root.join("reformatted-result"))?;
    let bytes = serde_json::to_vec(&original)?;
    assert_ne!(
        bytes,
        fs::read(study.root.join("export/verified/result.json"))?
    );
    fs::write(reformatted.root.join("export/verified/result.json"), &bytes)?;
    let mut saved = read(&reformatted.root, "export/verified/verification.json")?;
    saved["result_sha256"] = json!(format!("{:x}", Sha256::digest(bytes)));
    write(
        &reformatted.root,
        "export/verified/verification.json",
        &saved,
    )?;
    reformatted.verify_unchanged(true)?;
    study.verify_unchanged(true)?;
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn offline_revalidation_rejects_consistently_rehashed_wrong_modal_metadata() -> Result<()> {
    let mut agent = Agent::start()?;
    let study = Study::prepare(&agent, "metadata-evidence", &"9".repeat(32))?;
    agent.kill()?;
    let original = read(&study.root, "export/receipt.json")?;
    for (name, result, cause) in metadata_faults::cases(&original["response"]["result"]) {
        let case = study.copy(agent.root.join(name))?;
        let mut receipt = original.clone();
        receipt["response"]["result"] = result.clone();
        write(&case.root, "export/receipt.json", &receipt)?;
        // Keep all copied identity/content records consistent, so the physical
        // result contract, not a byte hash or terminal comparison, must reject it.
        write(&case.root, "submission/admission.json", &receipt)?;
        write(&case.root, "export/verified/result.json", &result)?;
        let mut report = read(&case.root, "export/verified/verification.json")?;
        report["result_sha256"] = json!(format!(
            "{:x}",
            Sha256::digest(fs::read(case.root.join("export/verified/result.json"))?)
        ));
        write(&case.root, "export/verified/verification.json", &report)?;
        let output = case.verify_unchanged(false)?;
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(cause), "expected {cause}: {error}");
    }
    study.verify_unchanged(true)?;
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn offline_verification_rejects_ambiguous_and_oversized_evidence() -> Result<()> {
    let mut agent = Agent::start()?;
    let study = Study::prepare(&agent, "bounded-evidence", &"d".repeat(32))?;
    agent.kill()?;
    for (index, (file, key, wrong)) in [
        ("submission/input.json", "mode_count", json!(0)),
        (
            "submission/submission-intent.json",
            "input_sha256",
            json!("wrong"),
        ),
        ("submission/admission.json", "store_id", json!("wrong")),
        ("export/receipt.json", "status", json!("failed")),
        ("export/verified/result.json", "total_mass", json!(1.0)),
        ("export/verified/verification.json", "passed", json!(false)),
    ]
    .into_iter()
    .enumerate()
    {
        let duplicate = study.copy(agent.root.join(format!("duplicate-{index}")))?;
        let bytes = fs::read_to_string(duplicate.root.join(file))?;
        fs::write(
            duplicate.root.join(file),
            format!(
                "{{{}:{},{}",
                serde_json::to_string(key)?,
                wrong,
                bytes.strip_prefix('{').unwrap()
            ),
        )?;
        // Keep dependent byte hashes consistent, so the duplicate-key gate itself
        // must reject a document whose last-key-wins interpretation looked valid.
        if index == 0 || index == 4 {
            let (report, field) = if index == 0 {
                ("submission/submission-intent.json", "input_sha256")
            } else {
                ("export/verified/verification.json", "result_sha256")
            };
            let mut value = read(&duplicate.root, report)?;
            value[field] = json!(format!(
                "{:x}",
                Sha256::digest(fs::read(duplicate.root.join(file))?)
            ));
            write(&duplicate.root, report, &value)?;
        }
        let output = duplicate.verify_unchanged(false)?;
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("duplicate JSON object key"), "{error}");
        assert!(
            error.contains(Path::new(file).file_name().unwrap().to_str().unwrap()),
            "{error}"
        );

        let oversized = study.copy(agent.root.join(format!("oversized-{index}")))?;
        fs::OpenOptions::new()
            .write(true)
            .open(oversized.root.join(file))?
            .set_len(8 * 1024 * 1024 + 1)?;
        let output = oversized.verify_unchanged(false)?;
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("8 MiB file limit"), "{error}");
        assert!(
            error.contains(Path::new(file).file_name().unwrap().to_str().unwrap()),
            "{error}"
        );
    }
    study.verify_unchanged(true)?;
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn offline_verification_rejects_damaged_mixed_and_unpublished_evidence() -> Result<()> {
    let mut agent = Agent::start()?;
    let study = Study::prepare(&agent, "original", &"a".repeat(32))?;
    let other = Study::prepare(&agent, "other-execution", &"b".repeat(32))?;
    agent.kill()?;
    for (index, file) in [
        "submission/input.json",
        "submission/submission-intent.json",
        "export/receipt.json",
        "export/verified/result.json",
        "export/verified/verification.json",
    ]
    .into_iter()
    .enumerate()
    {
        for corrupt in [false, true] {
            let case = study.copy(agent.root.join(format!("file-{index}-{corrupt}")))?;
            if corrupt {
                fs::write(case.root.join(file), b"{")?;
            } else {
                fs::remove_file(case.root.join(file))?;
            }
            case.verify_unchanged(false)?;
        }
    }

    for fault in [
        "input",
        "input-rehashed",
        "intent",
        "result",
        "result-rehashed",
        "result-and-receipt-rehashed",
        "receipt-state",
        "receipt-identity",
        "execution",
        "passed",
        "scope",
        "certification",
        "orthogonality",
        "basis",
        "admission-flag",
        "diagnostic-shape",
        "diagnostic-type",
        "corrupt-ack",
        "foreign-ack",
        "mixed-execution",
    ] {
        let case = study.copy(agent.root.join(fault))?;
        let mut saved = read(&case.root, "export/verified/verification.json")?;
        let mut receipt = read(&case.root, "export/receipt.json")?;
        match fault {
            "input" | "input-rehashed" => {
                let mut input = read(&case.root, "submission/input.json")?;
                input["elements"][0]["density"] = json!(99.0);
                write(&case.root, "submission/input.json", &input)?;
                if fault == "input-rehashed" {
                    let mut intent = read(&case.root, "submission/submission-intent.json")?;
                    intent["input_sha256"] = json!(format!(
                        "{:x}",
                        Sha256::digest(fs::read(case.root.join("submission/input.json"))?)
                    ));
                    write(&case.root, "submission/submission-intent.json", &intent)?;
                }
            }
            "intent" => {
                let mut intent = read(&case.root, "submission/submission-intent.json")?;
                intent["execution_intent"]["request_digest"] = json!("f".repeat(64));
                write(&case.root, "submission/submission-intent.json", &intent)?;
            }
            "result" | "result-rehashed" | "result-and-receipt-rehashed" => {
                let mut result = read(&case.root, "export/verified/result.json")?;
                result["total_mass"] = json!(1.0);
                write(&case.root, "export/verified/result.json", &result)?;
                if fault != "result" {
                    saved["result_sha256"] = json!(format!(
                        "{:x}",
                        Sha256::digest(fs::read(case.root.join("export/verified/result.json"))?)
                    ));
                }
                if fault == "result-and-receipt-rehashed" {
                    receipt["response"]["result"] = result;
                }
            }
            "receipt-state" => {
                receipt["status"] = json!("outcome_unknown");
                receipt["response"] = Value::Null;
            }
            "receipt-identity" => receipt["store_id"] = json!("f".repeat(32)),
            "execution" => saved["execution"]["request_id"] = json!("different-request"),
            "passed" => saved["passed"] = json!(false),
            "scope" => saved["scope"] = json!("different-reference"),
            "certification" => saved["material_certification"] = json!(true),
            "orthogonality" => saved["mass_orthogonality_checked"] = json!(false),
            "basis" => saved["recovery_basis"] = json!("untrusted"),
            "admission-flag" => saved["admission_response_available"] = json!("true"),
            "diagnostic-shape" => saved["independent_roots"] = json!([]),
            "diagnostic-type" => saved["physical_residuals"][0] = json!("not-a-number"),
            "corrupt-ack" => fs::write(case.root.join("submission/admission.json"), b"{")?,
            "foreign-ack" => {
                write(
                    &case.root,
                    "submission/admission.json",
                    &read(&other.root, "submission/admission.json")?,
                )?;
            }
            "mixed-execution" => {
                receipt = read(&other.root, "export/receipt.json")?;
                saved = read(&other.root, "export/verified/verification.json")?;
                write(
                    &case.root,
                    "export/verified/result.json",
                    &read(&other.root, "export/verified/result.json")?,
                )?;
            }
            _ => unreachable!(),
        }
        write(&case.root, "export/receipt.json", &receipt)?;
        write(&case.root, "export/verified/verification.json", &saved)?;
        let rejected = case.verify_unchanged(false)?;
        let expected_cause = match fault {
            "input-rehashed" => Some("durable input changed since preparation"),
            "result" => Some("export result bytes do not match the verification hash"),
            "result-rehashed" => Some("result does not match the original completed receipt"),
            "result-and-receipt-rehashed" => Some("wrong total mass"),
            "mixed-execution" => {
                Some("durable receipt does not match the prepared input or identity")
            }
            _ => None,
        };
        if let Some(cause) = expected_cause {
            let error = String::from_utf8_lossy(&rejected.stderr);
            assert!(
                error.contains(cause),
                "{fault} failed at the wrong boundary: {error}"
            );
        }
    }

    let pending = study.copy(agent.root.join("pending-only"))?;
    fs::rename(
        pending.root.join("export/verified"),
        pending.root.join("export/.verification-pending"),
    )?;
    pending.verify_unchanged(false)?;
    let missing = study.copy(agent.root.join("missing-export"))?;
    fs::remove_dir_all(missing.root.join("export"))?;
    missing.verify_unchanged(false)?;
    assert!(!missing.root.join("export").exists());
    study.verify_unchanged(true)?;
    agent.restart()?;
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        2
    );
    Ok(())
}
