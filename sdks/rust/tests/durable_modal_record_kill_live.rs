#[path = "../examples/durable_modal_research/evidence.rs"]
mod evidence;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "../examples/durable_modal_research/records.rs"]
mod records;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{DurableExecutionIntent, SolverRpcClient};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};
use support::{Agent, Result};

fn read(path: &Path) -> Result<Value> {
    let root = path.parent().ok_or("missing evidence parent")?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid evidence name")?;
    Ok(evidence::read(root, name)?.value)
}

fn hold(case: &Path) -> Result<()> {
    fs::write(case.join("writer-ready"), b"test checkpoint")?;
    thread::sleep(Duration::from_secs(60));
    Err("test checkpoint was not terminated".into())
}

// Only this test executable recognizes the hook. Normal examples have no pause API.
#[test]
fn record_writer_child_fixture() -> Result<()> {
    let Some(context) = env::var_os("KYUUBIKI_TEST_RECORD_CHILD_CONTEXT") else {
        return Ok(());
    };
    let context = PathBuf::from(context);
    let case = context.parent().ok_or("case parent missing")?;
    let config = read(&context)?;
    let phase = config["phase"].as_str().ok_or("phase missing")?;
    let endpoint: SocketAddr = config["endpoint"]
        .as_str()
        .ok_or("endpoint missing")?
        .parse()?;
    let intent: DurableExecutionIntent = serde_json::from_value(config["intent"].clone())?;
    let root = case.join("submission");
    let input = model::model(false);
    records::retain(&root, "input.json", &input)?;
    let saved = json!({"endpoint":endpoint.to_string(), "execution_intent":intent,
        "input_sha256":format!("{:x}", Sha256::digest(fs::read(root.join("input.json"))?))});
    if phase == "intent-partial" {
        return records::retain_with(&root, "submission-intent.json", &saved, |file, bytes| {
            file.write_all(&bytes[..bytes.len() / 2])?;
            file.sync_all()?;
            hold(case)
        });
    }
    records::retain(&root, "submission-intent.json", &saved)?;
    if phase == "prepared" {
        return hold(case);
    }
    let client = SolverRpcClient::new(endpoint.ip().to_string(), endpoint.port());
    let admission = client.submit_prepared_durable_agent_task(&intent, input)?;
    records::retain_with(&root, "admission.json", &admission, |file, bytes| {
        let count = match phase {
            "admission-partial" => bytes.len() / 2,
            "admission-staged" => bytes.len(),
            _ => return Err("unsupported child phase".into()),
        };
        file.write_all(&bytes[..count])?;
        file.sync_all()?;
        hold(case)
    })
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn interrupt(agent: &Agent, phase: &str, id: &str) -> Result<(PathBuf, DurableExecutionIntent)> {
    let case = agent.root.join(phase);
    fs::create_dir(&case)?;
    fs::create_dir(case.join("submission"))?;
    let intent = DurableExecutionIntent::new(
        &agent.store_id()?,
        id,
        "solve_modal_frame_2d",
        &model::model(false),
    )?;
    let config = case.join("fixture.json");
    fs::write(
        &config,
        serde_json::to_vec(&json!({"phase":phase,
        "endpoint":format!("127.0.0.1:{}", agent.port), "intent":intent}))?,
    )?;
    let log = fs::File::create(case.join("child.log"))?;
    let mut child = ChildGuard(
        Command::new(env::current_exe()?)
            .args(["--exact", "record_writer_child_fixture", "--nocapture"])
            .env("KYUUBIKI_TEST_RECORD_CHILD_CONTEXT", &config)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()?,
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !case.join("writer-ready").exists() {
        if child.0.try_wait()?.is_some() || Instant::now() >= deadline {
            return Err(format!(
                "child failed before {phase}: {}",
                fs::read_to_string(case.join("child.log"))?
            )
            .into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    child.0.kill()?;
    assert!(!child.0.wait()?.success());
    Ok((case, intent))
}

fn invoke(case: &Path, agent: &Agent, verb: &str) -> Result<Output> {
    let binary = fs::canonicalize(
        env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY").ok_or("SDK example binary required")?,
    )?;
    Ok(Command::new(binary)
        .arg(verb)
        .arg(case.join("submission"))
        .arg(case.join("export"))
        .env(
            "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
            if verb == "verify" {
                "offline".into()
            } else {
                format!("127.0.0.1:{}", agent.port)
            },
        )
        .output()?)
}

fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    fs::read_dir(root)?
        .map(|entry| {
            let entry = entry?;
            Ok((PathBuf::from(entry.file_name()), fs::read(entry.path())?))
        })
        .collect()
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn process_kill_before_dispatch_never_promotes_pending_intent_or_replays_work() -> Result<()> {
    let agent = Agent::start()?;
    for (phase, id) in [("intent-partial", "a"), ("prepared", "b")] {
        let (case, intent) = interrupt(&agent, phase, &id.repeat(32))?;
        let root = case.join("submission");
        let before = snapshot(&root)?;
        assert!(!root.join("admission.json").exists());
        if phase == "intent-partial" {
            assert!(!root.join("submission-intent.json").exists());
            assert!(root.join(".submission-intent.json.pending").is_file());
        } else {
            intent.validate_input(&read(&root.join("input.json"))?)?;
            assert!(root.join("submission-intent.json").is_file());
        }
        assert_eq!(
            agent.client().inspect_durable_agent_task(&intent)?["status"],
            "not_found"
        );
        let output = invoke(&case, &agent, "inspect")?;
        assert!(!output.status.success());
        assert!(!case.join("export/verified").exists());
        if phase == "intent-partial" {
            assert!(String::from_utf8_lossy(&output.stderr).contains("submission-intent.json"));
            assert!(!case.join("export").exists());
        } else {
            assert_eq!(
                read(&case.join("export/receipt.json"))?["status"],
                "not_found"
            );
        }
        assert_eq!(snapshot(&root)?, before);
        assert_eq!(
            agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
            0
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn process_kill_during_admission_write_recovers_original_task_without_promoting_pending()
-> Result<()> {
    let mut agent = Agent::start()?;
    for (index, (phase, id)) in [("admission-partial", "c"), ("admission-staged", "d")]
        .into_iter()
        .enumerate()
    {
        let (case, intent) = interrupt(&agent, phase, &id.repeat(32))?;
        let root = case.join("submission");
        let before = snapshot(&root)?;
        assert!(!root.join("admission.json").exists());
        let pending = fs::read(root.join(".admission.json.pending"))?;
        assert_eq!(
            serde_json::from_slice::<Value>(&pending).is_ok(),
            phase == "admission-staged"
        );
        let observed = agent.client().inspect_durable_agent_task(&intent)?;
        let handle = intent.handle_from_report(&observed)?;
        let completed = agent.wait(&handle, "completed")?;
        agent.kill()?;
        agent.restart()?;
        let output = invoke(&case, &agent, "inspect")?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let verification = read(&case.join("export/verified/verification.json"))?;
        assert_eq!(verification["passed"], true);
        assert_eq!(verification["admission_response_available"], false);
        assert_eq!(read(&case.join("export/receipt.json"))?, completed);
        let verified = invoke(&case, &agent, "verify")?;
        assert!(
            verified.status.success(),
            "{}",
            String::from_utf8_lossy(&verified.stderr)
        );
        assert_eq!(snapshot(&root)?, before);
        assert_eq!(
            agent
                .client()
                .fetch_durable_agent_execution_after(&completed)?,
            completed
        );
        assert_eq!(
            agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
            index + 1
        );
    }
    Ok(())
}
