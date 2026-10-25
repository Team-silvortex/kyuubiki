#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::DurableExecutionHandle;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    env, fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use support::{Agent, Result};

fn write(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn read(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    fn walk(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            let relative = path.strip_prefix(root)?.to_owned();
            if fs::symlink_metadata(&path)?.is_dir() {
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

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

struct Study {
    binary: PathBuf,
    root: PathBuf,
    receipt: Value,
}

impl Study {
    fn prepare(agent: &Agent, name: &str, id: &str) -> Result<Self> {
        let binary = fs::canonicalize(
            env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY").ok_or("example binary required")?,
        )?;
        let root = agent.root.join(name);
        fs::create_dir(&root)?;
        let expected_store = agent.store_id()?;
        success(
            &Command::new(&binary)
                .arg("submit")
                .arg(root.join("submission"))
                .arg(id)
                .env(
                    "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
                    format!("127.0.0.1:{}", agent.port),
                )
                .output()?,
        );
        let handle =
            DurableExecutionHandle::from_report(&read(&root.join("submission/admission.json"))?)?;
        assert_eq!(handle.store_id, expected_store);
        let receipt = agent.wait(&handle, "completed")?;
        let source = root.join("interrupted");
        fs::create_dir(&source)?;
        write(&source.join("receipt.json"), &receipt)?;
        fs::create_dir(source.join(".verification-pending"))?;
        // Deliberately unusable staging: recovery must derive from the receipt only.
        fs::write(
            source.join(".verification-pending/result.json"),
            b"not a result",
        )?;
        Ok(Self {
            binary,
            root,
            receipt,
        })
    }

    fn copy(&self, root: PathBuf) -> Result<Self> {
        fs::create_dir(&root)?;
        for (relative, bytes) in snapshot(&self.root)? {
            if self.root.join(&relative).is_dir() {
                fs::create_dir(root.join(relative))?;
            } else {
                fs::write(root.join(relative), bytes)?;
            }
        }
        Ok(Self {
            binary: self.binary.clone(),
            root,
            receipt: self.receipt.clone(),
        })
    }

    fn recover(&self, destination: &Path) -> Result<Output> {
        Ok(Command::new(&self.binary)
            .arg("recover-export")
            .arg(self.root.join("submission"))
            .arg(self.root.join("interrupted"))
            .arg(destination)
            .env(
                "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
                "offline-must-not-connect",
            )
            .output()?)
    }

    fn reject(&self, destination: &Path, cause: &str) -> Result<()> {
        let before = snapshot(&self.root)?;
        let output = self.recover(destination)?;
        assert!(
            !output.status.success(),
            "recovery unexpectedly accepted {cause}"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(cause), "expected {cause}: {error}");
        assert_eq!(
            snapshot(&self.root)?,
            before,
            "rejected recovery changed evidence"
        );
        Ok(())
    }
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn offline_export_recovery_uses_only_completed_receipt_without_network_or_source_writes()
-> Result<()> {
    let mut agent = Agent::start()?;
    let original = Study::prepare(&agent, "original", &"a".repeat(32))?;
    let missing_ack = original.copy(agent.root.join("missing-ack"))?;
    fs::remove_file(missing_ack.root.join("submission/admission.json"))?;
    agent.kill()?;
    let probe = TcpListener::bind(("127.0.0.1", agent.port))?;
    probe.set_nonblocking(true)?;
    for study in [&original, &missing_ack] {
        let before = snapshot(&study.root)?;
        let destination = agent.root.join(format!(
            "{}-recovered",
            study.root.file_name().unwrap().to_str().unwrap()
        ));
        let output = study.recover(&destination)?;
        success(&output);
        assert!(String::from_utf8_lossy(&output.stdout).contains("offline export recovered"));
        assert_eq!(snapshot(&study.root)?, before);
        assert_eq!(read(&destination.join("receipt.json"))?, study.receipt);
        let handle = DurableExecutionHandle::from_report(&study.receipt)?;
        handle.validate_completed_result(
            &study.receipt,
            &read(&destination.join("verified/result.json"))?,
        )?;
        let report = read(&destination.join("verified/verification.json"))?;
        assert_eq!(report["passed"], true);
        assert_eq!(
            report["admission_response_available"],
            study.root.join("submission/admission.json").exists()
        );
        success(
            &Command::new(&study.binary)
                .arg("verify")
                .arg(study.root.join("submission"))
                .arg(&destination)
                .env(
                    "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
                    "offline-must-not-connect",
                )
                .output()?,
        );
        let saved = snapshot(&destination)?;
        study.reject(&destination, "already exists")?;
        assert_eq!(snapshot(&destination)?, saved);
        assert!(!destination.join(".verification-pending").exists());
    }
    assert_eq!(
        probe.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(probe);
    agent.restart()?;
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        1
    );
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution(&DurableExecutionHandle::from_report(
                &original.receipt
            )?)?,
        original.receipt
    );
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn offline_export_recovery_rejects_invalid_or_conflicting_receipts_before_creating_output()
-> Result<()> {
    let mut agent = Agent::start()?;
    let study = Study::prepare(&agent, "original", &"b".repeat(32))?;
    let other = Study::prepare(&agent, "other", &"c".repeat(32))?;
    agent.kill()?;
    for (fault, cause) in [
        ("failed", "retained failure"),
        ("running", "not completed"),
        ("outcome_unknown", "not completed"),
        ("not_found", "unsupported durable status"),
        ("foreign", "prepared input"),
        ("bad-physics", "angular frequency"),
        ("missing", "required file is missing"),
        ("partial", "receipt.json"),
        ("duplicate", "duplicate JSON object key"),
        ("oversized", "8 MiB file limit"),
        ("corrupt-admission", "admission.json"),
        ("terminal-conflict", "terminal observation changed"),
        ("changed-input", "submission input changed"),
    ] {
        let case = study.copy(agent.root.join(fault))?;
        let path = case.root.join("interrupted/receipt.json");
        let mut receipt = case.receipt.clone();
        // Exercise the intended receipt gate even if this fast solve happened
        // to return a terminal admission. Admission conflicts have their own case.
        if !matches!(fault, "corrupt-admission" | "terminal-conflict") {
            fs::remove_file(case.root.join("submission/admission.json"))?;
        }
        match fault {
            "failed" => {
                receipt["status"] = json!("failed");
                receipt["response"] = json!({"rpc_version":1,"id":receipt["request_id"],"ok":false,
                    "result":null,"error":{"code":"cancelled","message":"retained failure"}});
                write(&path, &receipt)?;
            }
            "running" | "outcome_unknown" | "not_found" => {
                receipt["status"] = json!(fault);
                receipt["response"] = Value::Null;
                write(&path, &receipt)?;
            }
            "foreign" => write(&path, &other.receipt)?,
            "bad-physics" => {
                receipt["response"]["result"]["modes"][0]["eigenvalue_rad_s_squared"] =
                    json!(999.0);
                write(&path, &receipt)?;
            }
            "missing" => fs::rename(&path, case.root.join("interrupted/.receipt.json.pending"))?,
            "partial" => fs::write(&path, b"{")?,
            "duplicate" => {
                let bytes = fs::read_to_string(&path)?;
                fs::write(
                    &path,
                    format!(
                        "{{\"status\":\"failed\",{}",
                        bytes.strip_prefix('{').unwrap()
                    ),
                )?;
            }
            "oversized" => fs::OpenOptions::new()
                .write(true)
                .open(&path)?
                .set_len(8 * 1024 * 1024 + 1)?,
            "corrupt-admission" => fs::write(case.root.join("submission/admission.json"), b"{")?,
            "terminal-conflict" => {
                write(&case.root.join("submission/admission.json"), &receipt)?;
                for value in receipt["response"]["result"]["modes"][0]["shape"]
                    .as_array_mut()
                    .unwrap()
                {
                    *value = json!(-value.as_f64().unwrap());
                }
                write(&path, &receipt)?;
            }
            "changed-input" => fs::write(case.root.join("submission/input.json"), b"{}")?,
            _ => unreachable!(),
        }
        let destination = case.root.join("recovered");
        case.reject(&destination, cause)?;
        assert!(!destination.exists());
    }
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn offline_export_recovery_cannot_bypass_published_evidence_or_write_inside_sources() -> Result<()>
{
    let mut agent = Agent::start()?;
    let study = Study::prepare(&agent, "original", &"d".repeat(32))?;
    agent.kill()?;
    for kind in ["empty", "corrupt", "file"] {
        let case = study.copy(agent.root.join(kind))?;
        let published = case.root.join("interrupted/verified");
        if kind == "file" {
            fs::write(&published, b"blocked evidence")?;
        } else {
            fs::create_dir(&published)?;
            if kind == "corrupt" {
                fs::write(published.join("result.json"), b"{")?;
            }
        }
        let destination = case.root.join("recovered");
        case.reject(&destination, "use verify, not recover-export")?;
        assert!(!destination.exists());
    }
    for source in ["submission", "interrupted"] {
        let destination = study.root.join(source).join("new-export");
        study.reject(&destination, "outside retained evidence")?;
        assert!(!destination.exists());
        study.reject(&study.root.join(source), "already exists")?;
    }
    #[cfg(unix)]
    {
        let alias = agent.root.join("alias");
        std::os::unix::fs::symlink(study.root.join("interrupted"), &alias)?;
        study.reject(&alias.join("new-export"), "outside retained evidence")?;
        assert!(!alias.join("new-export").exists());
        let published = study.root.join("interrupted/verified");
        std::os::unix::fs::symlink("missing-published", &published)?;
        let output = study.recover(&agent.root.join("blocked"))?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("use verify, not recover-export"));
        assert!(fs::symlink_metadata(published)?.file_type().is_symlink());
        assert!(!agent.root.join("blocked").exists());
    }
    Ok(())
}
