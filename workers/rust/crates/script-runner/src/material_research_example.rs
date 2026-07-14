use crate::native_time::utc_iso_timestamp;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

mod check;

const DEFAULT_OUT: &str = "tmp/material-research-example.json";
const DEFAULT_STUDY: &str = "heat-spreader";

type RunnerResult<T> = Result<T, String>;

pub(crate) use check::run_check_material_research_example;

pub(crate) fn run_capture_material_research_example(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(args)?;
    let (absolute, relative) = repo_local_path(root, &options.out, "--out")?;
    let evidence = build_evidence(root, &options.study)?;
    write_json(&absolute, &evidence)?;
    println!("material research example wrote {relative}");
    Ok(0)
}

struct Options {
    out: String,
    study: String,
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        out: DEFAULT_OUT.to_string(),
        study: DEFAULT_STUDY.to_string(),
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner capture-material-research-example [--out tmp/material-research-example.json] [--study heat-spreader]"
                );
                return Ok(options);
            }
            "--out" => options.out = required_value(&mut iter, "--out")?,
            "--study" => options.study = required_value(&mut iter, "--study")?,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if options.study != DEFAULT_STUDY {
        return Err(
            "the first automated research example is intentionally fixed to heat-spreader"
                .to_string(),
        );
    }
    Ok(options)
}

pub(super) fn required_value(
    iter: &mut impl Iterator<Item = OsString>,
    name: &str,
) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{name} requires a repo-local path"))
}

fn build_evidence(root: &Path, study: &str) -> RunnerResult<Value> {
    let (mut command, stdout) = run_exploration(root, study)?;
    let exploration: Value = serde_json::from_str(&stdout)
        .map_err(|error| format!("material exploration did not emit JSON: {error}"))?;
    let report = exploration.get("report").unwrap_or(&Value::Null);
    if let Some(object) = command.as_object_mut() {
        object.remove("stdout");
        object.remove("stderr");
    }
    Ok(json!({
        "schema_version": "kyuubiki.automated-material-research-example/v1",
        "example_id": "material.heat_spreader_screening.automated_research.v1",
        "generated_at_utc": utc_iso_timestamp(),
        "posture": "screening_research_example",
        "study": study,
        "command": command,
        "exploration_sha256": sha256_compact_json(&stdout),
        "summary": {
            "exploration_schema_version": field(&exploration, "schema_version"),
            "report_schema_version": field(report, "schema_version"),
            "template_id": field(&exploration, "template_id"),
            "mode": field(&exploration, "mode"),
            "iteration": exploration.get("iteration").cloned().unwrap_or(Value::Null),
            "next_round_iteration": exploration.pointer("/next_round/iteration").cloned().unwrap_or(Value::Null),
            "next_round_decision": exploration.pointer("/next_round/decision").cloned().unwrap_or(Value::Null),
            "candidate_count": exploration.get("candidate_count").cloned().unwrap_or(Value::Null),
            "result_payload_count": exploration.get("result_payloads").and_then(Value::as_array).map_or(0, Vec::len),
            "winner_candidate_id": report.get("winner_candidate_id").cloned().unwrap_or(Value::Null),
            "reliability_posture": report.pointer("/reliability/posture").cloned().unwrap_or(Value::Null),
            "optimization_id": report.pointer("/optimization/id").cloned().unwrap_or(Value::Null),
            "candidates": candidate_summary(report),
            "quality_gates": report.pointer("/reliability/quality_gates").cloned().unwrap_or_else(|| json!([])),
            "limitations": report.pointer("/reliability/limitations").cloned().unwrap_or_else(|| json!([])),
        },
        "exploration": exploration,
    }))
}

fn run_exploration(root: &Path, study: &str) -> RunnerResult<(Value, String)> {
    let argv = vec![
        "cargo",
        "run",
        "-q",
        "-p",
        "kyuubiki-cli",
        "--bin",
        "kyuubiki-material-explore",
        "--",
        study,
        "--json",
    ];
    let started = Instant::now();
    let output = Command::new(argv[0])
        .args(&argv[1..])
        .current_dir(root.join("workers/rust"))
        .output()
        .map_err(|error| format!("failed to run material exploration: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "material exploration command failed".to_string()
        } else {
            stderr
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok((
        json!({
            "id": "material_explore_heat_spreader",
            "cwd": "workers/rust",
            "argv": argv,
            "status": output.status.code().unwrap_or(0),
            "signal": Value::Null,
            "duration_ms": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            "stdout": stdout,
            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
            "ok": true,
        }),
        stdout,
    ))
}

fn candidate_summary(report: &Value) -> Vec<Value> {
    report
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|candidate| {
            json!({
                "rank": candidate.get("rank").cloned().unwrap_or(Value::Null),
                "candidate_id": candidate.get("candidate_id").cloned().unwrap_or(Value::Null),
                "score": candidate.get("score").cloned().unwrap_or(Value::Null),
                "peak_temperature_c": candidate.get("peak_temperature_c").cloned().unwrap_or(Value::Null),
                "areal_mass_kg_m2": candidate.get("areal_mass_kg_m2").cloned().unwrap_or(Value::Null),
                "conductivity_density_ratio": candidate.get("conductivity_density_ratio").cloned().unwrap_or(Value::Null),
                "material_card_confidence": candidate.get("material_card_confidence").cloned().unwrap_or(Value::Null),
            })
        })
        .collect()
}

fn sha256_compact_json(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.trim().as_bytes());
    hasher.update(b"\n");
    format!("{:x}", hasher.finalize())
}

pub(super) fn repo_local_path(
    root: &Path,
    path: &str,
    label: &str,
) -> RunnerResult<(PathBuf, String)> {
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
        .map_err(|error| format!("failed to encode material research example: {error}"))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::sha256_compact_json;

    #[test]
    fn compact_hash_uses_trimmed_stdout_plus_newline() {
        assert_eq!(
            sha256_compact_json("{\"a\":1}\n"),
            "e346432021b04179518d9614f3560ccd71354a4ee101ddcb893d6959a9d6301c"
        );
    }
}
