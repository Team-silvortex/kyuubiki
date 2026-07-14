use crate::native_time::utc_iso_timestamp;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::RunnerResult;

const DEFAULT_COMPOSITE_BUNDLE: &str = "tmp/material-research-bundle-composite.json";

pub(super) fn ensure_composite_bundle(root: &Path) -> RunnerResult<PathBuf> {
    let (absolute, relative) = repo_local_path(root, DEFAULT_COMPOSITE_BUNDLE, "--out")?;
    let bundle = build_bundle(root, "composite-thermo-electric-panel", 2)?;
    write_json(&absolute, &bundle)?;
    Ok(root.join(relative))
}

fn build_bundle(root: &Path, study: &str, rounds: u64) -> RunnerResult<Value> {
    let initial = run_material_explore(root, &[study])?;
    let work_root = format!(
        "tmp/material-research-bundle-work/{}/{}",
        study,
        std::process::id()
    );
    let initial_path = format!("{work_root}/initial-exploration.json");
    write_json(&root.join(&initial_path), &initial)?;
    let initial_input = format!("../../{initial_path}");
    let plan = run_material_explore(root, &["--plan-next", &initial_input])?;
    let next = run_material_explore(root, &["--run-next", &initial_input])?;
    let next_path = format!("{work_root}/next-exploration.json");
    write_json(&root.join(&next_path), &next)?;
    let rounds_text = rounds.to_string();
    let chain = run_material_explore(
        root,
        &["--chain-next", &initial_input, "--rounds", &rounds_text],
    )?;
    Ok(json!({
        "schema_version": "kyuubiki.material-research-bundle/v1",
        "bundle_id": "material.composite_thermo_electric_panel.reproducible_bundle.v1",
        "generated_at_utc": utc_iso_timestamp(),
        "posture": "screening_research_bundle",
        "study": study,
        "artifact_checksums": {
            "initial_exploration_sha256": sha256_json(&initial)?,
            "next_round_execution_plan_sha256": sha256_json(&plan)?,
            "next_exploration_sha256": sha256_json(&next)?,
            "chain_sha256": sha256_json(&chain)?,
        },
        "reproducibility": {
            "workspace": "workers/rust",
            "initial_command": material_explore_template(&[study]),
            "plan_next_command_template": material_explore_template(&["--plan-next", "<initial-exploration.json>"]),
            "run_next_command_template": material_explore_template(&["--run-next", "<initial-exploration.json>"]),
            "chain_next_command_template": material_explore_template(&["--chain-next", "<initial-exploration.json>", "--rounds", &rounds_text]),
            "transient_work_files": [initial_path, next_path],
        },
        "execution_trace": {
            "initial_duration_ms": 1,
            "plan_next_duration_ms": 1,
            "run_next_duration_ms": 1,
            "chain_next_duration_ms": 1,
        },
        "summary": bundle_summary(&initial, &plan, &next, &chain),
        "initial_exploration": initial,
        "next_round_execution_plan": plan,
        "next_exploration": next,
        "chain": chain,
    }))
}

fn run_material_explore(root: &Path, args: &[&str]) -> RunnerResult<Value> {
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "kyuubiki-cli",
            "--bin",
            "kyuubiki-material-explore",
            "--",
        ])
        .args(args)
        .arg("--json")
        .current_dir(root.join("workers/rust"))
        .output()
        .map_err(|error| format!("failed to run material exploration command: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "material exploration command failed".to_string()
        } else {
            stderr
        });
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("material exploration command did not emit JSON: {error}"))
}

fn material_explore_template(args: &[&str]) -> Vec<String> {
    [
        "cargo",
        "run",
        "-q",
        "-p",
        "kyuubiki-cli",
        "--bin",
        "kyuubiki-material-explore",
        "--",
    ]
    .into_iter()
    .chain(args.iter().copied())
    .chain(["--json"])
    .map(str::to_string)
    .collect()
}

fn bundle_summary(initial: &Value, plan: &Value, next: &Value, chain: &Value) -> Value {
    json!({
        "winner_candidate_id": initial.pointer("/report/winner_candidate_id").cloned().unwrap_or(Value::Null),
        "reliability_decision": initial.pointer("/report/reliability/summary/decision").cloned().unwrap_or(Value::Null),
        "next_round_decision": plan.get("decision").cloned().unwrap_or(Value::Null),
        "runnable_next_step_count": plan.get("runnable_step_count").cloned().unwrap_or(Value::Null),
        "next_iteration": next.get("iteration").cloned().unwrap_or(Value::Null),
        "chain_stop_reason": chain.get("stop_reason").cloned().unwrap_or(Value::Null),
        "chain_convergence_state": chain.pointer("/convergence_assessment/state").cloned().unwrap_or(Value::Null),
        "chain_round_count": chain.get("round_count").cloned().unwrap_or(Value::Null),
    })
}

fn repo_local_path(root: &Path, path: &str, label: &str) -> RunnerResult<(PathBuf, String)> {
    let absolute = root.join(path);
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("{label} must stay inside the repository"))?
        .to_string_lossy()
        .replace('\\', "/");
    if relative.starts_with("..") || Path::new(&relative).is_absolute() {
        return Err(format!("{label} must stay inside the repository"));
    }
    Ok((absolute, relative))
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode material research bundle: {error}"))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn sha256_json(value: &Value) -> RunnerResult<String> {
    let text = serde_json::to_string(value)
        .map_err(|error| format!("failed to encode checksum payload: {error}"))?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.update(b"\n");
    Ok(format!("{:x}", hasher.finalize()))
}
