//! A small direct-Agent research example. Fault injection belongs to the test harness.
#[path = "durable_modal_research/checks.rs"]
mod checks;
#[path = "durable_modal_research/evidence.rs"]
mod evidence;
#[path = "durable_modal_research/model.rs"]
mod model;
#[path = "durable_modal_research/publication.rs"]
mod publication;
#[path = "durable_modal_research/readback.rs"]
mod readback;
#[path = "durable_modal_research/records.rs"]
mod records;

use kyuubiki_headless_sdk::{DurableExecutionIntent, SolverRpcClient};
use records::retain;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{env, fs, net::SocketAddr, path::Path};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn load(root: &Path, name: &str) -> Result<Value> {
    Ok(evidence::read(root, name)?.value)
}

fn require_new_directory(path: &Path) -> Result<()> {
    let context = format!("new output directory {}", path.display());
    if path.file_name().is_none() {
        return Err(format!("{context}: expected a new named directory").into());
    }
    match fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(format!("{context}: already exists; refusing to replace evidence").into());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("{context}: {error}").into()),
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !fs::metadata(parent)
        .map_err(|error| format!("{context}: {error}"))?
        .is_dir()
    {
        return Err(format!("{context}: parent is not a directory").into());
    }
    // This read-only preflight is not a reservation. create_dir still owns the
    // final no-overwrite decision after the remote read, including concurrent changes.
    Ok(())
}

fn require_unpublished_directory(path: &Path) -> Result<()> {
    if !fs::symlink_metadata(path)?.is_dir() {
        return Err("recovery source must be a non-symlink directory".into());
    }
    match fs::symlink_metadata(path.join("verified")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => Err(
            "recovery source already contains verified evidence; use verify, not recover-export"
                .into(),
        ),
    }
}

fn require_external_destination(path: &Path, sources: &[&Path]) -> Result<()> {
    require_new_directory(path)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent)?;
    for source in sources {
        if parent.starts_with(fs::canonicalize(source)?) {
            return Err(
                "recovery destination must be outside retained evidence directories".into(),
            );
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let recover = args.first().is_some_and(|arg| arg == "recover-export");
    if args.len() != if recover { 4 } else { 3 }
        || !matches!(
            args[0].as_str(),
            "submit" | "inspect" | "verify" | "recover-export"
        )
    {
        return Err("usage: durable_modal_research submit <new-dir> <32-hex-execution-id> | inspect <submission-dir> <new-output-dir> | verify <submission-dir> <export-dir> | recover-export <submission-dir> <interrupted-export-dir> <new-output-dir>; submit/inspect require KYUUBIKI_RESEARCH_AGENT_ENDPOINT; verify is offline and read-only; recover-export is offline and writes only a new directory".into());
    }
    let root = Path::new(&args[1]);
    if args[0] == "submit" {
        require_new_directory(root)?;
        let endpoint: SocketAddr = env::var("KYUUBIKI_RESEARCH_AGENT_ENDPOINT")?.parse()?;
        let client = SolverRpcClient::new(endpoint.ip().to_string(), endpoint.port());
        let descriptor = client.describe_agent()?.result;
        let store = descriptor["durable_execution"]["store_id"]
            .as_str()
            .ok_or("durable store is required")?;
        let input = model::model(false);
        let intent = DurableExecutionIntent::new(store, &args[2], "solve_modal_frame_2d", &input)?;
        fs::create_dir(root)?;
        // Retain input and recovery identity before the first mutation leaves the SDK.
        retain(root, "input.json", &input)?;
        retain(
            root,
            "submission-intent.json",
            &json!({"endpoint":endpoint.to_string(), "execution_intent":intent,
            "input_sha256":format!("{:x}", Sha256::digest(evidence::read(root, "input.json")?.bytes))}),
        )?;
        let admission = client.submit_prepared_durable_agent_task(&intent, input)?;
        retain(root, "admission.json", &admission)?;
        println!(
            "admitted {}; inspect the original execution without resubmitting",
            args[2]
        );
    } else {
        let retained = load(root, "submission-intent.json")?;
        let input_document = evidence::read(root, "input.json")?;
        let original_endpoint: SocketAddr = retained["endpoint"]
            .as_str()
            .ok_or("missing original endpoint")?
            .parse()?;
        if retained["input_sha256"] != format!("{:x}", Sha256::digest(&input_document.bytes)) {
            return Err("submission input changed; inspect the original research directory".into());
        }
        let input = input_document.value;
        let intent: DurableExecutionIntent =
            serde_json::from_value(retained["execution_intent"].clone())?;
        intent.validate_input(&input)?;
        // A corrupt local acknowledgement must not be masked by a remote outage
        // or leave a fresh diagnostic export before it is even inspected.
        let admission =
            evidence::read_optional(root, "admission.json")?.map(|document| document.value);
        let admitted = admission
            .as_ref()
            .map(|value| {
                intent
                    .handle_from_report(value)
                    .map_err(|error| format!("admission.json: {error}"))
            })
            .transpose()?;
        let out = Path::new(&args[2]);
        if recover {
            require_unpublished_directory(out)?;
        }
        let receipt = if args[0] == "inspect" {
            require_new_directory(out)?;
            let endpoint: SocketAddr = env::var("KYUUBIKI_RESEARCH_AGENT_ENDPOINT")?.parse()?;
            if endpoint != original_endpoint {
                return Err("submission endpoint changed; inspect the original endpoint".into());
            }
            let client = SolverRpcClient::new(endpoint.ip().to_string(), endpoint.port());
            let receipt = client.inspect_durable_agent_task(&intent)?;
            fs::create_dir(out)?;
            retain(out, "receipt.json", &receipt)?;
            receipt
        } else {
            load(out, "receipt.json")?
        };
        let recovered = intent.handle_from_report(&receipt)?;
        // Only a missing acknowledgement permits intent-based reconciliation.
        // A present but corrupt or contradictory acknowledgement never gets ignored.
        let handle = admitted.unwrap_or(recovered);
        if let Some(value) = &admission {
            handle
                .validate_followup(value, &receipt)
                .map_err(|error| format!("admission.json / receipt.json consistency: {error}"))?;
        }
        if recover {
            // A retained completed receipt is the only source; never promote
            // staged files or fetch/replay computation to fill missing evidence.
            checks::verify(&input, handle.completed_result(&receipt)?)?;
            let destination = Path::new(&args[3]);
            require_external_destination(destination, &[root, out])?;
            fs::create_dir(destination)?;
            retain(destination, "receipt.json", &receipt)?;
            publication::publish(destination, &input, &handle, &receipt, admission.is_some())?;
            println!(
                "offline export recovered from retained receipt; no Agent contacted or task resubmitted"
            );
        } else if args[0] == "inspect" {
            publication::publish(out, &input, &handle, &receipt, admission.is_some())?;
            println!("four physical modes independently verified after JSON readback");
        } else {
            let report = readback::verify(out, &input, &handle, &receipt)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}
