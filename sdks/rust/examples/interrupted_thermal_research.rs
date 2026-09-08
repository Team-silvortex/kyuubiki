//! Phased public-SDK acceptance; process faults are controlled outside the SDK.
#[path = "layered_thermal_research/checks.rs"]
mod checks;
#[path = "layered_thermal_research/interruption.rs"]
mod interruption;
#[allow(dead_code)]
#[path = "layered_thermal_research/model.rs"]
mod model;

use kyuubiki_headless_sdk::{ControlPlaneClient, KyuubikiAuth, SolverRpcClient};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::Write,
    path::Path,
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn retain(root: &Path, name: &str, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

fn load(root: &Path, name: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(root.join(name))?)?)
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() < 3 {
        return Err("usage: interrupted_thermal_research submit <new-dir> <idempotent|checkpoint_required> | capture|await-held <submission-dir> <new-dir> | verify <submission-dir> <new-dir> <completed|replayed|retry-blocked|recovery-blocked> <baseline-dir>".into());
    }
    let auth = env::var("KYUUBIKI_ACCESS_TOKEN")
        .ok()
        .map(KyuubikiAuth::access_token);
    // Require an explicit service; never silently use a local fallback for fault qualification.
    let client = ControlPlaneClient::new_with_auth(&env::var("KYUUBIKI_BASE_URL")?, auth)?;
    match args[0].as_str() {
        "submit" if args.len() == 3 => submit(&client, Path::new(&args[1]), &args[2]),
        "capture" | "await-held" if args.len() == 3 => {
            let root = Path::new(&args[2]);
            fs::create_dir(root)?;
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                let snapshot = capture(&client, Path::new(&args[1]))?;
                if args[0] == "capture" || interruption::held_after_heat(&snapshot) {
                    return retain(root, "snapshot.json", &snapshot);
                }
                if Instant::now() >= deadline {
                    retain(root, "timeout-snapshot.json", &snapshot)?;
                    return Err("research did not reach the job-scoped structure hold".into());
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
        "verify" if args.len() == 5 => verify(
            &client,
            Path::new(&args[1]),
            Path::new(&args[2]),
            &args[3],
            Path::new(&args[4]),
        ),
        _ => Err("invalid research command or arguments".into()),
    }
}

fn submit(client: &ControlPlaneClient, root: &Path, policy: &str) -> Result<()> {
    let request = interruption::request(policy)?;
    fs::create_dir(root)?;
    retain(root, "request.json", &request)?;
    retain(root, "health-before.json", &client.health()?)?;
    let submission =
        client.submit_workflow_graph_job(&request["graph"], &request["input_artifacts"])?;
    retain(root, "submission.json", &submission)?;
    let id = submission["job"]["job_id"]
        .as_str()
        .ok_or("missing submitted job identity")?;
    fs::write(root.join("job-id.txt"), id)?;
    retain(
        root,
        "provenance.json",
        &json!({
            "schema_version":"kyuubiki.interrupted-thermal-research-input/v1",
            "sdk_version":env!("CARGO_PKG_VERSION"), "execution":"official_rust_headless_sdk_service",
            "source_revision":env::var("KYUUBIKI_SOURCE_REVISION").unwrap_or_else(|_| "unrecorded".into()),
            "request_sha256":format!("{:x}", Sha256::digest(fs::read(root.join("request.json"))?)),
            "job_id":id
        }),
    )?;
    println!("submitted {id}");
    Ok(())
}

fn capture(client: &ControlPlaneClient, root: &Path) -> Result<Value> {
    let submitted = load(root, "submission.json")?;
    let id = submitted["job"]["job_id"]
        .as_str()
        .ok_or("missing submitted job identity")?;
    Ok(
        json!({"job":client.fetch_job(id)?, "result":client.fetch_result(id)?,
        "health":client.health()?, "agents":client.agents()?,
        "agent_observations":agent_observations()?}),
    )
}

fn agent_observations() -> Result<Value> {
    let mut observations = vec![];
    for address in env::var("KYUUBIKI_RESEARCH_AGENT_ENDPOINTS")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
    {
        let endpoint: std::net::SocketAddr = address.parse()?;
        let descriptor =
            SolverRpcClient::new(endpoint.ip().to_string(), endpoint.port()).describe_agent();
        observations.push(match descriptor {
            Ok(value) => json!({"address":address, "descriptor":value.result}),
            Err(error) => json!({"address":address, "error":error.to_string()}),
        });
    }
    Ok(json!(observations))
}

fn verify(
    client: &ControlPlaneClient,
    input: &Path,
    out: &Path,
    expected: &str,
    baseline: &Path,
) -> Result<()> {
    interruption::validate_expectation(expected)?;
    fs::create_dir(out)?;
    let submitted = load(input, "submission.json")?;
    let id = submitted["job"]["job_id"]
        .as_str()
        .ok_or("missing submitted job identity")?;
    let request = load(input, "request.json")?;
    let provenance = load(input, "provenance.json")?;
    if provenance["job_id"] != id
        || provenance["request_sha256"]
            != format!(
                "{:x}",
                Sha256::digest(fs::read(input.join("request.json"))?)
            )
    {
        return Err("research input provenance mismatch".into());
    }
    let deadline = Instant::now() + Duration::from_secs(90);
    let terminal = loop {
        let value = client.fetch_job(id)?;
        if matches!(
            value["job"]["status"].as_str(),
            Some("completed" | "failed" | "cancelled")
        ) {
            break value;
        }
        if Instant::now() >= deadline {
            retain(out, "timeout-job.json", &value)?;
            return Err("original research job did not reach terminal state within 90s".into());
        }
        thread::sleep(Duration::from_millis(100));
    };
    let result = client.fetch_result(id)?;
    retain(out, "terminal.json", &terminal)?;
    retain(out, "result.json", &result)?;
    let mut report = json!({
        "schema_version":"kyuubiki.interrupted-thermal-research/v1",
        "qualification":"synthetic_reference_not_material_certification",
        "fault_proof":"requires_separate_process_and_inflight_evidence",
        "job_id":id, "expected":expected, "passed":false
    });
    let validation = (|| -> Result<Value> {
        interruption::validate_terminal(id, &terminal, &result, expected)?;
        if expected == "completed" || expected == "replayed" {
            let gate = interruption::validate_physics(&request, &result)?;
            if baseline != out {
                let reference = load(baseline, "result.json")?;
                interruption::validate_physics(&interruption::request("idempotent")?, &reference)?;
                interruption::compare_physics(&reference, &result)?;
            }
            Ok(gate)
        } else {
            Ok(json!({"numerical_acceptance":false, "unsafe_replay_blocked":true}))
        }
    })();
    match validation {
        Ok(gates) => {
            report["gates"] = gates;
            report["passed"] = json!(true);
        }
        Err(error) => report["error"] = json!(error.to_string()),
    }
    // A late result from the previous owner must not overwrite the committed generation.
    thread::sleep(Duration::from_millis(600));
    let stable = client.fetch_result(id)?;
    report["terminal_stable"] =
        json!(stable == result && client.fetch_job(id)?["job"] == terminal["job"]);
    if report["terminal_stable"] != true {
        report["passed"] = json!(false);
    }
    retain(out, "result-later.json", &stable)?;
    retain(out, "health-after.json", &client.health()?)?;
    retain(out, "agents-after.json", &client.agents()?)?;
    retain(out, "agent-observations-after.json", &agent_observations()?)?;
    retain(out, "report.json", &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report["passed"] != true {
        return Err("interrupted research acceptance failed; evidence retained".into());
    }
    Ok(())
}
