use crate::native_time::utc_iso_timestamp;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const DEFAULT_OUT: &str = "tmp/material-research-bundle.json";
const DEFAULT_STUDY: &str = "heat-spreader";
const DEFAULT_ROUNDS: u64 = 2;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_build_material_research_bundle(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(args)?;
    let relative =
        build_material_research_bundle_file(root, &options.study, &options.out, options.rounds)?;
    println!("material research bundle wrote {relative}");
    Ok(0)
}

pub(crate) fn build_material_research_bundle_file(
    root: &Path,
    study: &str,
    out: &str,
    rounds: u64,
) -> RunnerResult<String> {
    let (absolute, relative) = repo_local_path(root, out, "--out")?;
    let options = Options {
        out: out.to_string(),
        study: study.to_string(),
        rounds,
    };
    let bundle = build_bundle(root, &options)?;
    write_json(&absolute, &bundle)?;
    Ok(relative)
}

struct Options {
    out: String,
    study: String,
    rounds: u64,
}

struct StudyProfile {
    bundle_id: &'static str,
    work_slug: &'static str,
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        out: DEFAULT_OUT.to_string(),
        study: DEFAULT_STUDY.to_string(),
        rounds: DEFAULT_ROUNDS,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner build-material-research-bundle [--study heat-spreader] [--out tmp/material-research-bundle.json] [--rounds 2]"
                );
                return Ok(options);
            }
            "--out" => options.out = required_value(&mut iter, "--out")?,
            "--study" => options.study = required_value(&mut iter, "--study")?,
            "--rounds" => {
                let value = required_value(&mut iter, "--rounds")?;
                options.rounds = value
                    .parse::<u64>()
                    .map_err(|_| "--rounds must be a positive integer".to_string())?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if options.rounds == 0 {
        return Err("--rounds must be a positive integer".to_string());
    }
    study_profile(&options.study)?;
    Ok(options)
}

fn required_value(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn study_profile(study: &str) -> RunnerResult<StudyProfile> {
    match study {
        "heat-spreader" => Ok(StudyProfile {
            bundle_id: "material.heat_spreader_screening.reproducible_bundle.v1",
            work_slug: "heat-spreader",
        }),
        "composite-thermo-electric-panel" => Ok(StudyProfile {
            bundle_id: "material.composite_thermo_electric_panel.reproducible_bundle.v1",
            work_slug: "composite-thermo-electric-panel",
        }),
        other => Err(format!(
            "unsupported retained research bundle study: {other}"
        )),
    }
}

fn build_bundle(root: &Path, options: &Options) -> RunnerResult<Value> {
    let profile = study_profile(&options.study)?;
    let work_root = format!(
        "tmp/material-research-bundle-work/{}/{}",
        profile.work_slug,
        std::process::id()
    );
    let initial = run_material_explore(root, &[&options.study])?;
    let initial_path = format!("{work_root}/initial-exploration.json");
    write_json(&root.join(&initial_path), &initial.payload)?;
    let initial_input = format!("../../{initial_path}");
    let plan = run_material_explore(root, &["--plan-next", &initial_input])?;
    let next = run_material_explore(root, &["--run-next", &initial_input])?;
    let next_path = format!("{work_root}/next-exploration.json");
    write_json(&root.join(&next_path), &next.payload)?;
    let rounds = options.rounds.to_string();
    let chain = run_material_explore(root, &["--chain-next", &initial_input, "--rounds", &rounds])?;
    Ok(json!({
        "schema_version": "kyuubiki.material-research-bundle/v1",
        "bundle_id": profile.bundle_id,
        "generated_at_utc": utc_iso_timestamp(),
        "posture": "screening_research_bundle",
        "study": options.study,
        "artifact_checksums": {
            "initial_exploration_sha256": sha256_json(&initial.payload)?,
            "next_round_execution_plan_sha256": sha256_json(&plan.payload)?,
            "next_exploration_sha256": sha256_json(&next.payload)?,
            "chain_sha256": sha256_json(&chain.payload)?,
        },
        "reproducibility": {
            "workspace": "workers/rust",
            "initial_command": initial.command,
            "plan_next_command_template": material_explore_template(&["--plan-next", "<initial-exploration.json>"]),
            "run_next_command_template": material_explore_template(&["--run-next", "<initial-exploration.json>"]),
            "chain_next_command_template": material_explore_template(&["--chain-next", "<initial-exploration.json>", "--rounds", &rounds]),
            "transient_work_files": [initial_path, next_path],
        },
        "execution_trace": {
            "initial_duration_ms": initial.duration_ms,
            "plan_next_duration_ms": plan.duration_ms,
            "run_next_duration_ms": next.duration_ms,
            "chain_next_duration_ms": chain.duration_ms,
        },
        "summary": bundle_summary(&initial.payload, &plan.payload, &next.payload, &chain.payload),
        "initial_exploration": initial.payload,
        "next_round_execution_plan": plan.payload,
        "next_exploration": next.payload,
        "chain": chain.payload,
    }))
}

struct MaterialExploreRun {
    command: Vec<String>,
    duration_ms: u64,
    payload: Value,
}

fn run_material_explore(root: &Path, args: &[&str]) -> RunnerResult<MaterialExploreRun> {
    let argv = material_explore_template(args);
    let started = Instant::now();
    let output = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(root.join("workers/rust"))
        .output()
        .map_err(|error| format!("failed to run material exploration command: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("command failed: {}", argv.join(" "))
        } else {
            stderr
        });
    }
    let payload = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("{} did not emit JSON: {error}", argv.join(" ")))?;
    Ok(MaterialExploreRun {
        command: argv,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        payload,
    })
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

#[cfg(test)]
mod tests {
    use super::{material_explore_template, study_profile};

    #[test]
    fn supports_retained_bundle_studies() {
        assert_eq!(
            study_profile("heat-spreader").unwrap().bundle_id,
            "material.heat_spreader_screening.reproducible_bundle.v1"
        );
        assert!(study_profile("unknown").is_err());
    }

    #[test]
    fn material_explore_template_keeps_json_flag() {
        let argv = material_explore_template(&["heat-spreader"]);
        assert_eq!(argv.last().map(String::as_str), Some("--json"));
    }
}
