#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{DurableExecutionHandle, DurableExecutionIntent};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs, io,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use support::{Agent, Result};

struct Probe {
    endpoint: String,
    count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Probe {
    fn new() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let endpoint = listener.local_addr()?.to_string();
        listener.set_nonblocking(true)?;
        let count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let seen = count.clone();
        let stopped = stop.clone();
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((socket, _)) => {
                        seen.fetch_add(1, Ordering::AcqRel);
                        drop(socket);
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1))
                    }
                    Err(e) => panic!("probe accept failed: {e}"),
                }
            }
        });
        Ok(Self {
            endpoint,
            count,
            stop,
            worker: Some(worker),
        })
    }

    fn assert_unused(&self) {
        assert_eq!(
            self.count.load(Ordering::Acquire),
            0,
            "local rejection must precede connection"
        );
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let result = worker.join();
            if !thread::panicking() {
                result.expect("connection probe must not fail silently");
            }
        }
    }
}

fn binary() -> Result<PathBuf> {
    Ok(fs::canonicalize(
        env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY").ok_or("SDK example binary required")?,
    )?)
}

fn invoke(binary: &Path, verb: &str, root: &Path, last: &Path, endpoint: &str) -> Result<Output> {
    Ok(Command::new(binary)
        .arg(verb)
        .arg(root)
        .arg(last)
        .env("KYUUBIKI_RESEARCH_AGENT_ENDPOINT", endpoint)
        .output()?)
}

fn write(root: &Path, name: &str, value: &Value) -> Result<()> {
    fs::write(root.join(name), serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn prepare(agent: &Agent, root: &Path, endpoint: &str) -> Result<Value> {
    let input = model::model(false);
    let intent = DurableExecutionIntent::new(
        &agent.store_id()?,
        &"8".repeat(32),
        "solve_modal_frame_2d",
        &input,
    )?;
    let admission = agent
        .client()
        .submit_prepared_durable_agent_task(&intent, input.clone())?;
    let handle = intent.handle_from_report(&admission)?;
    let completed = agent.wait(&handle, "completed")?;
    fs::create_dir(root)?;
    write(root, "input.json", &input)?;
    write(
        root,
        "submission-intent.json",
        &json!({"endpoint":endpoint,
        "execution_intent":intent,
        "input_sha256":format!("{:x}", Sha256::digest(fs::read(root.join("input.json"))?))}),
    )?;
    write(root, "admission.json", &admission)?;
    Ok(completed)
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn local_invalid_admission_precedes_network_and_export_creation() -> Result<()> {
    let agent = Agent::start()?;
    let probe = Probe::new()?;
    let binary = binary()?;
    let root = agent.root.join("preflight");
    let completed = prepare(&agent, &root, &probe.endpoint)?;
    let handle = DurableExecutionHandle::from_report(&completed)?;
    let mut foreign = completed.clone();
    foreign["store_id"] = json!("f".repeat(32));
    let mut malformed = completed.clone();
    malformed["response"]["id"] = json!("not-the-original-request");
    for (index, bytes) in [
        b"{".to_vec(),
        b"null".to_vec(),
        serde_json::to_vec(&foreign)?,
        serde_json::to_vec(&malformed)?,
        br#"{"status":"running","status":"completed"}"#.to_vec(),
    ]
    .into_iter()
    .enumerate()
    {
        fs::write(root.join("admission.json"), &bytes)?;
        for (verb, endpoint) in [
            ("inspect", probe.endpoint.as_str()),
            ("inspect", "bad-endpoint"),
            ("verify", "offline"),
        ] {
            let out = agent
                .root
                .join(format!("rejected-{index}-{verb}-{endpoint}").replace(':', "-"));
            let output = invoke(&binary, verb, &root, &out, endpoint)?;
            assert!(!output.status.success());
            probe.assert_unused();
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("admission.json"), "wrong failure: {error}");
            assert!(!out.exists());
            assert_eq!(fs::read(root.join("admission.json"))?, bytes);
        }
    }
    fs::remove_file(root.join("admission.json"))?;
    fs::create_dir(root.join("admission.json"))?;
    let out = agent.root.join("directory-admission");
    let output = invoke(&binary, "inspect", &root, &out, &probe.endpoint)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("admission.json"));
    probe.assert_unused();
    assert!(!out.exists());
    assert!(root.join("admission.json").is_dir());
    fs::remove_dir(root.join("admission.json"))?;

    // Genuine absence still proceeds to one read; the probe closes it deliberately.
    let output = invoke(&binary, "inspect", &root, &out, &probe.endpoint)?;
    assert!(!output.status.success());
    assert_eq!(probe.count.load(Ordering::Acquire), 1);
    assert!(!out.exists());
    let endpoint = format!("127.0.0.1:{}", agent.port);
    let mut intent: Value =
        serde_json::from_slice(&fs::read(root.join("submission-intent.json"))?)?;
    intent["endpoint"] = json!(endpoint);
    write(&root, "submission-intent.json", &intent)?;
    let output = invoke(&binary, "inspect", &root, &out, &endpoint)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out.join("verified/result.json").is_file());
    let relative = Command::new(&binary)
        .current_dir(&agent.root)
        .args(["inspect", "preflight", "relative-export"])
        .env("KYUUBIKI_RESEARCH_AGENT_ENDPOINT", &endpoint)
        .output()?;
    assert!(
        relative.status.success(),
        "{}",
        String::from_utf8_lossy(&relative.stderr)
    );
    assert!(
        agent
            .root
            .join("relative-export/verified/result.json")
            .is_file()
    );
    assert_eq!(
        agent.client().fetch_durable_agent_execution(&handle)?,
        completed
    );
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        1
    );
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn occupied_or_impossible_research_destination_precedes_network() -> Result<()> {
    let agent = Agent::start()?;
    let probe = Probe::new()?;
    let binary = binary()?;
    let root = agent.root.join("preflight");
    prepare(&agent, &root, &probe.endpoint)?;
    let file = agent.root.join("existing-file");
    fs::write(&file, b"keep this file")?;
    let directory = agent.root.join("existing-dir");
    fs::create_dir(&directory)?;
    fs::write(directory.join("owner"), b"keep this directory")?;
    let destinations = vec![
        file.clone(),
        directory.clone(),
        agent.root.join("missing-parent/output"),
        file.join("output"),
    ];
    #[cfg(unix)]
    let destinations = {
        let mut destinations = destinations;
        let link = agent.root.join("broken-link");
        std::os::unix::fs::symlink("missing-target", &link)?;
        destinations.push(link);
        destinations
    };
    for out in &destinations {
        for verb in ["submit", "inspect"] {
            let (source, last) = if verb == "submit" {
                (out.as_path(), PathBuf::from("7".repeat(32)))
            } else {
                (root.as_path(), out.clone())
            };
            let output = invoke(&binary, verb, source, &last, &probe.endpoint)?;
            assert!(!output.status.success());
            probe.assert_unused();
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains("new output directory"),
                "wrong failure: {error}"
            );
        }
    }
    assert_eq!(fs::read(file)?, b"keep this file");
    assert_eq!(fs::read(directory.join("owner"))?, b"keep this directory");
    assert!(!agent.root.join("missing-parent").exists());
    Ok(())
}
