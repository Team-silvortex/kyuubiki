#[path = "../examples/durable_modal_research/checks.rs"]
mod checks;
#[path = "../examples/durable_modal_research/evidence.rs"]
mod evidence;
#[path = "support/modal_metadata_faults.rs"]
mod metadata_faults;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "../examples/durable_modal_research/publication.rs"]
mod publication;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{DurableExecutionHandle, DurableExecutionIntent};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use support::{Agent, Result};

fn retain(root: &Path, name: &str, value: &Value) -> Result<()> {
    let mut file = fs::File::create_new(root.join(name))?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    Ok(())
}

fn completed(agent: &Agent) -> Result<(Value, DurableExecutionHandle, Value)> {
    let input = model::model(false);
    let admission = agent.client().submit_durable_agent_task(
        &agent.store_id()?,
        &"a".repeat(32),
        "solve_modal_frame_2d",
        input.clone(),
    )?;
    let handle = DurableExecutionHandle::from_report(&admission)?;
    let report = agent.wait(&handle, "completed")?;
    Ok((input, handle, report))
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn invalid_physical_results_never_publish_formal_export_files() -> Result<()> {
    let agent = Agent::start()?;
    let (input, handle, completed) = completed(&agent)?;
    checks::verify(&input, handle.completed_result(&completed)?)?;
    for (index, (pointer, value)) in [
        (
            "/response/result/modes/0/eigenvalue_rad_s_squared",
            json!(999.0),
        ),
        ("/response/result/modes", json!([])),
        ("/response/result/total_mass", json!(1.0)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut corrupt = completed.clone();
        *corrupt.pointer_mut(pointer).unwrap() = value;
        let out = agent.root.join(format!("invalid-{index}"));
        fs::create_dir(&out)?;
        retain(&out, "receipt.json", &corrupt)?;
        assert!(publication::publish(&out, &input, &handle, &corrupt, true).is_err());
        assert!(
            !out.join("result.json").exists(),
            "unverified result was published: {pointer}"
        );
        assert!(!out.join("verification.json").exists());
        assert!(!out.join("verified").exists());
        assert!(out.join("receipt.json").exists());
    }
    assert_eq!(
        agent.client().fetch_durable_agent_execution(&handle)?,
        completed
    );
    Ok(())
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn inconsistent_modal_metadata_never_publishes_verified_results() -> Result<()> {
    let agent = Agent::start()?;
    let (input, handle, completed) = completed(&agent)?;
    let result = handle.completed_result(&completed)?;
    checks::verify(&input, result)?;
    for (name, result, cause) in metadata_faults::cases(result) {
        let mut report = completed.clone();
        report["response"]["result"] = result;
        let out = agent.root.join(name);
        fs::create_dir(&out)?;
        retain(&out, "receipt.json", &report)?;
        let before = fs::read(out.join("receipt.json"))?;
        let verdict = publication::publish(&out, &input, &handle, &report, true);
        assert!(
            verdict.is_err(),
            "inconsistent metadata accepted: {}",
            out.display()
        );
        assert!(verdict.unwrap_err().to_string().contains(cause), "{cause}");
        assert!(!out.join("verified").exists());
        assert!(!out.join(".verification-pending").exists());
        assert_eq!(fs::read(out.join("receipt.json"))?, before);
    }
    assert_eq!(
        agent.client().fetch_durable_agent_execution(&handle)?,
        completed
    );
    Ok(())
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn staged_readback_and_publication_failures_never_replace_verified_evidence() -> Result<()> {
    let agent = Agent::start()?;
    let (input, handle, report) = completed(&agent)?;
    for fault in [
        "corrupt",
        "missing",
        "verification-blocked",
        "commit-blocked",
    ] {
        let out = agent.root.join(fault);
        fs::create_dir(&out)?;
        retain(&out, "receipt.json", &report)?;
        let result = publication::publish_with(&out, &input, &handle, &report, true, |stage| {
            match fault {
                "corrupt" => fs::write(stage.join("result.json"), b"{}")?,
                "missing" => fs::remove_file(stage.join("result.json"))?,
                "verification-blocked" => fs::create_dir(stage.join("verification.json"))?,
                "commit-blocked" => {
                    fs::create_dir(out.join("verified"))?;
                    fs::write(out.join("verified/owner.txt"), b"existing evidence")?;
                }
                _ => unreachable!(),
            }
            Ok(())
        });
        assert!(result.is_err(), "{fault}");
        assert!(!out.join(".verification-pending").exists(), "{fault}");
        assert!(!out.join("verified/result.json").exists(), "{fault}");
        assert!(!out.join("verified/verification.json").exists(), "{fault}");
        assert_eq!(read(&out, "receipt.json")?, report);
        if fault == "commit-blocked" {
            assert_eq!(
                fs::read(out.join("verified/owner.txt"))?,
                b"existing evidence"
            );
        }
    }
    let pending = agent.root.join("preexisting-stage");
    fs::create_dir(&pending)?;
    fs::create_dir(pending.join(".verification-pending"))?;
    fs::write(
        pending.join(".verification-pending/owner.txt"),
        b"interrupted evidence",
    )?;
    assert!(publication::publish(&pending, &input, &handle, &report, true).is_err());
    assert_eq!(
        fs::read(pending.join(".verification-pending/owner.txt"))?,
        b"interrupted evidence"
    );

    let out = agent.root.join("healthy");
    fs::create_dir(&out)?;
    publication::publish(&out, &input, &handle, &report, true)?;
    let verification = fs::read(out.join("verified/verification.json"))?;
    let result = fs::read(out.join("verified/result.json"))?;
    assert!(publication::publish(&out, &input, &handle, &report, false).is_err());
    assert_eq!(
        fs::read(out.join("verified/verification.json"))?,
        verification
    );
    assert_eq!(fs::read(out.join("verified/result.json"))?, result);
    assert_eq!(
        agent.client().fetch_durable_agent_execution(&handle)?,
        report
    );
    Ok(())
}

fn read(root: &Path, name: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(root.join(name))?)?)
}

// Only the test executable accepts this hook. The shipped example has no hold env var.
#[test]
fn publication_child_fixture() -> Result<()> {
    let Some(out) = env::var_os("KYUUBIKI_TEST_MODAL_PUBLICATION_CHILD") else {
        return Ok(());
    };
    let out = Path::new(&out);
    let input = read(out, "input.json")?;
    let report = read(out, "receipt.json")?;
    let handle = DurableExecutionHandle::from_report(&report)?;
    publication::publish_with(out, &input, &handle, &report, true, |_| {
        fs::write(out.join("stage-ready"), b"unverified")?;
        thread::sleep(Duration::from_secs(60));
        Ok(())
    })
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn killed_export_process_leaves_only_pending_data_and_fresh_inspection_recovers() -> Result<()> {
    let binary = fs::canonicalize(
        env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY").ok_or("SDK example binary required")?,
    )?;
    let mut agent = Agent::start()?;
    let (input, handle, report) = completed(&agent)?;
    let interrupted = agent.root.join("interrupted-export");
    fs::create_dir(&interrupted)?;
    retain(&interrupted, "input.json", &input)?;
    retain(&interrupted, "receipt.json", &report)?;
    let log = fs::File::create(interrupted.join("child.log"))?;
    let mut child = ChildGuard(
        Command::new(env::current_exe()?)
            .args(["--exact", "publication_child_fixture", "--nocapture"])
            .env("KYUUBIKI_TEST_MODAL_PUBLICATION_CHILD", &interrupted)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()?,
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !interrupted.join("stage-ready").exists() {
        if child.0.try_wait()?.is_some() || Instant::now() >= deadline {
            return Err(format!(
                "publication child never staged: {}",
                fs::read_to_string(interrupted.join("child.log"))?
            )
            .into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    child.0.kill()?;
    assert!(!child.0.wait()?.success());
    assert!(
        interrupted
            .join(".verification-pending/result.json")
            .exists()
    );
    assert!(!interrupted.join("verified").exists());
    assert!(!interrupted.join("result.json").exists());
    assert_eq!(read(&interrupted, "receipt.json")?, report);
    agent.kill()?;

    let submission = agent.root.join("submission");
    fs::create_dir(&submission)?;
    retain(&submission, "input.json", &input)?;
    let intent = DurableExecutionIntent::new(
        &handle.store_id,
        &handle.execution_id,
        &handle.method,
        &input,
    )?;
    retain(
        &submission,
        "submission-intent.json",
        &json!({
            "endpoint":format!("127.0.0.1:{}", agent.port), "execution_intent":intent,
            "input_sha256":format!("{:x}", Sha256::digest(fs::read(submission.join("input.json"))?))
        }),
    )?;
    let pending_before = fs::read(interrupted.join(".verification-pending/result.json"))?;
    let receipt_before = fs::read(interrupted.join("receipt.json"))?;
    let offline = agent.root.join("offline-export");
    let recovered = Command::new(&binary)
        .arg("recover-export")
        .arg(&submission)
        .arg(&interrupted)
        .arg(&offline)
        .env(
            "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
            "offline-must-not-connect",
        )
        .output()?;
    assert!(
        recovered.status.success(),
        "{}",
        String::from_utf8_lossy(&recovered.stderr)
    );
    let verified_offline = Command::new(&binary)
        .arg("verify")
        .arg(&submission)
        .arg(&offline)
        .env(
            "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
            "offline-must-not-connect",
        )
        .output()?;
    assert!(verified_offline.status.success());
    assert_eq!(read(&offline, "receipt.json")?, report);
    assert_eq!(fs::read(interrupted.join("receipt.json"))?, receipt_before);
    assert_eq!(
        fs::read(interrupted.join(".verification-pending/result.json"))?,
        pending_before
    );
    assert!(!interrupted.join("verified").exists());
    agent.restart()?;
    assert_eq!(
        agent.client().fetch_durable_agent_execution(&handle)?,
        report
    );
    let fresh = agent.root.join("fresh-export");
    let inspected = Command::new(binary)
        .arg("inspect")
        .arg(&submission)
        .arg(&fresh)
        .env(
            "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
            format!("127.0.0.1:{}", agent.port),
        )
        .output()?;
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    let verified = fresh.join("verified");
    let verification = read(&verified, "verification.json")?;
    assert_eq!(verification["passed"], true);
    assert_eq!(verification["execution"], serde_json::to_value(&handle)?);
    assert_eq!(
        verification["result_sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(verified.join("result.json"))?)
        )
    );
    let result = read(&verified, "result.json")?;
    assert_eq!(&result, handle.completed_result(&report)?);
    checks::verify(&input, &result)?;
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        1
    );
    assert!(!interrupted.join("verified").exists());
    Ok(())
}
