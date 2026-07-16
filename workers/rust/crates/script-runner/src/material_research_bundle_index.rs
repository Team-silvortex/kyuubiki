use crate::{
    material_research_bundle::validate_material_research_bundle_value,
    material_research_bundle_build::build_material_research_bundle_file,
    native_time::utc_iso_timestamp,
};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_OUT_DIR: &str = "tmp/material-research-bundles";
const BUNDLE_PROFILES: &[BundleProfile] = &[
    BundleProfile {
        study: "heat-spreader",
        file: "heat-spreader.json",
    },
    BundleProfile {
        study: "composite-thermo-electric-panel",
        file: "composite-thermo-electric-panel.json",
    },
];

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_build_material_research_bundle_index(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(args)?;
    let result = if options.self_test {
        run_self_test()
    } else {
        build_index_files(root, &options)
    };
    match result {
        Ok(message) => {
            println!("{message}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("material research bundle index failed: {issue}");
            Ok(1)
        }
    }
}

struct Options {
    out_dir: String,
    ensure_bundles: bool,
    self_test: bool,
}

#[derive(Clone, Copy)]
struct BundleProfile {
    study: &'static str,
    file: &'static str,
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        out_dir: DEFAULT_OUT_DIR.to_string(),
        ensure_bundles: false,
        self_test: false,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner build-material-research-bundle-index [--self-test] [--ensure-bundles] [--out-dir tmp/material-research-bundles]"
                );
                return Ok(options);
            }
            "--self-test" => options.self_test = true,
            "--ensure-bundles" => options.ensure_bundles = true,
            "--out-dir" => {
                options.out_dir = iter
                    .next()
                    .map(|value| value.to_string_lossy().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "--out-dir requires a repo-local path".to_string())?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

fn run_self_test() -> RunnerResult<String> {
    let index = build_index(vec![IndexEntry {
        profile: BundleProfile {
            study: "heat-spreader",
            file: "a.json",
        },
        path: "tmp/a.json".to_string(),
        bundle: json!({
            "study": "heat-spreader",
            "bundle_id": "bundle.a",
            "posture": "screening_research_bundle",
            "summary": {
                "winner_candidate_id": "candidate-a",
                "reliability_decision": "blocked_by_quality_gates",
                "next_round_decision": "mitigate_design_risk",
                "runnable_next_step_count": 3,
                "next_iteration": 2,
                "chain_stop_reason": "risk_mitigation_required",
                "chain_convergence_state": "blocked_by_quality_gates",
                "chain_round_count": 2,
            },
            "research_evidence": {
                "candidate_count": 2,
                "ranked_candidate_ids": ["candidate-a", "candidate-b"],
                "winner_candidate_id": "candidate-a",
                "primary_metric_ids": ["peak_temperature_c"],
                "metric_objective_count": 1,
                "violated_quality_gate_ids": ["gate.temperature"],
                "focus_candidate_ids": ["candidate-a"],
                "quality_gate_decision": "blocked_by_quality_gates",
                "plan_decision": "mitigate_design_risk",
                "plan_step_count": 3,
                "chain_round_count": 2,
                "chain_trace_round_count": 2,
                "final_winner_candidate_id": "candidate-b",
            },
        }),
    }]);
    if index.get("bundle_count").and_then(Value::as_u64) != Some(1)
        || index.pointer("/reliability_decision_counts/blocked_by_quality_gates")
            != Some(&Value::from(1))
    {
        return Err("self-test did not build expected index counts".to_string());
    }
    if index.pointer("/bundles/0/runnable_next_step_count") != Some(&Value::from(3))
        || index.pointer("/bundles/0/next_iteration") != Some(&Value::from(2))
    {
        return Err("self-test did not retain next-round execution summary".to_string());
    }
    if index.pointer("/bundles/0/final_winner_candidate_id") != Some(&Value::from("candidate-b"))
        || index.pointer("/bundles/0/winner_changed_in_chain") != Some(&Value::from(true))
        || index.pointer("/winner_changed_in_chain_count") != Some(&Value::from(1))
    {
        return Err("self-test did not retain compact research evidence".to_string());
    }
    Ok("material research bundle index self-test passed".to_string())
}

fn build_index_files(root: &Path, options: &Options) -> RunnerResult<String> {
    let (out_dir, out_dir_relative) = repo_local_path(root, &options.out_dir, "--out-dir")?;
    fs::create_dir_all(&out_dir)
        .map_err(|error| format!("failed to create {}: {error}", out_dir.display()))?;
    if options.ensure_bundles {
        for profile in BUNDLE_PROFILES {
            let bundle_path = format!("{out_dir_relative}/{}", profile.file);
            build_material_research_bundle_file(root, profile.study, &bundle_path, 2)?;
        }
    }
    let mut entries = Vec::new();
    for profile in BUNDLE_PROFILES {
        let relative_path = format!("{out_dir_relative}/{}", profile.file);
        let (absolute, _) = repo_local_path(root, &relative_path, "bundle path")?;
        if !absolute.exists() {
            return Err(format!(
                "bundle does not exist: {relative_path}; pass --ensure-bundles to build it"
            ));
        }
        let (bundle, raw_text) = read_bundle(&absolute, &relative_path)?;
        validate_material_research_bundle_value(root, &bundle, Some(&raw_text))?;
        entries.push(IndexEntry {
            profile: *profile,
            path: relative_path,
            bundle,
        });
    }
    let index = build_index(entries);
    write_json(&out_dir.join("index.json"), &index)?;
    write_readme(&index, &out_dir.join("README.md"))?;
    Ok(format!(
        "material research bundle index wrote {out_dir_relative}/index.json"
    ))
}

struct IndexEntry {
    profile: BundleProfile,
    path: String,
    bundle: Value,
}

fn build_index(entries: Vec<IndexEntry>) -> Value {
    let bundles: Vec<Value> = entries
        .into_iter()
        .map(|entry| bundle_index_entry(&entry.profile, &entry.path, &entry.bundle))
        .collect();
    json!({
        "schema_version": "kyuubiki.material-research-bundle-index/v1",
        "generated_at_utc": utc_iso_timestamp(),
        "bundle_count": bundles.len(),
        "studies": bundles.iter().filter_map(|bundle| bundle.get("study").cloned()).collect::<Vec<_>>(),
        "winner_changed_in_chain_count": bundles.iter().filter(|bundle| bundle.get("winner_changed_in_chain").and_then(Value::as_bool) == Some(true)).count(),
        "reliability_decision_counts": counts_by(&bundles, "reliability_decision"),
        "next_round_decision_counts": counts_by(&bundles, "next_round_decision"),
        "bundles": bundles,
    })
}

fn bundle_index_entry(profile: &BundleProfile, path: &str, bundle: &Value) -> Value {
    let initial_winner = pointer_or_null(bundle, "/summary/winner_candidate_id");
    let final_winner = pointer_or_null(bundle, "/research_evidence/final_winner_candidate_id");
    let winner_changed = initial_winner != Value::Null
        && final_winner != Value::Null
        && initial_winner != final_winner;
    json!({
        "study": bundle.get("study").cloned().unwrap_or(Value::Null),
        "bundle_id": bundle.get("bundle_id").cloned().unwrap_or(Value::Null),
        "path": path,
        "posture": bundle.get("posture").cloned().unwrap_or(Value::Null),
        "winner_candidate_id": initial_winner,
        "final_winner_candidate_id": final_winner,
        "winner_changed_in_chain": winner_changed,
        "reliability_decision": pointer_or_null(bundle, "/summary/reliability_decision"),
        "next_round_decision": pointer_or_null(bundle, "/summary/next_round_decision"),
        "runnable_next_step_count": pointer_or_null(bundle, "/summary/runnable_next_step_count"),
        "next_iteration": pointer_or_null(bundle, "/summary/next_iteration"),
        "chain_stop_reason": pointer_or_null(bundle, "/summary/chain_stop_reason"),
        "chain_convergence_state": pointer_or_null(bundle, "/summary/chain_convergence_state"),
        "chain_round_count": pointer_or_null(bundle, "/summary/chain_round_count"),
        "chain_trace_round_count": pointer_or_null(bundle, "/research_evidence/chain_trace_round_count"),
        "research_candidate_count": pointer_or_null(bundle, "/research_evidence/candidate_count"),
        "primary_metric_ids": pointer_or_null(bundle, "/research_evidence/primary_metric_ids"),
        "metric_objective_count": pointer_or_null(bundle, "/research_evidence/metric_objective_count"),
        "violated_quality_gate_ids": pointer_or_null(bundle, "/research_evidence/violated_quality_gate_ids"),
        "focus_candidate_ids": pointer_or_null(bundle, "/research_evidence/focus_candidate_ids"),
        "profile_study": profile.study,
    })
}

fn counts_by(items: &[Value], key: &str) -> Value {
    let mut counts = BTreeMap::<String, u64>::new();
    for item in items {
        let value = item
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *counts.entry(value).or_insert(0) += 1;
    }
    Value::Object(
        counts
            .into_iter()
            .map(|(key, count)| (key, Value::from(count)))
            .collect::<Map<_, _>>(),
    )
}

fn write_readme(index: &Value, output: &Path) -> RunnerResult<()> {
    let mut lines = vec![
        "# Material Research Bundles".to_string(),
        String::new(),
        format!("Generated: {}", field(index, "generated_at_utc")),
        String::new(),
        format!(
            "Bundles: {}",
            index.get("bundle_count").unwrap_or(&Value::Null)
        ),
        String::new(),
        "| Study | Winner | Final winner | Metrics | Gates | Next round | Chain |".to_string(),
        "| --- | --- | --- | --- | --- | --- | --- |".to_string(),
    ];
    for bundle in index
        .get("bundles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}@{}` steps=`{}` | `{}/{}` rounds=`{}` trace=`{}` |",
            field(bundle, "study"),
            field(bundle, "winner_candidate_id"),
            field(bundle, "final_winner_candidate_id"),
            array_len(bundle, "primary_metric_ids"),
            array_len(bundle, "violated_quality_gate_ids"),
            field(bundle, "next_round_decision"),
            bundle.get("next_iteration").unwrap_or(&Value::Null),
            bundle
                .get("runnable_next_step_count")
                .unwrap_or(&Value::Null),
            field(bundle, "chain_stop_reason"),
            field(bundle, "chain_convergence_state"),
            bundle.get("chain_round_count").unwrap_or(&Value::Null),
            bundle.get("chain_trace_round_count").unwrap_or(&Value::Null),
        ));
    }
    lines.push(String::new());
    fs::write(output, format!("{}\n", lines.join("\n")))
        .map_err(|error| format!("failed to write {}: {error}", output.display()))
}

fn read_bundle(path: &Path, label: &str) -> RunnerResult<(Value, String)> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {label}: {error}"))?;
    let bundle =
        serde_json::from_str(&text).map_err(|error| format!("{label}: invalid json: {error}"))?;
    Ok((bundle, text))
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode bundle index: {error}"))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
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

fn pointer_or_null(value: &Value, pointer: &str) -> Value {
    value.pointer(pointer).cloned().unwrap_or(Value::Null)
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn array_len(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{BundleProfile, IndexEntry, build_index};
    use serde_json::json;

    #[test]
    fn self_test_fixture_builds_counts() {
        let index = build_index(vec![IndexEntry {
            profile: BundleProfile {
                study: "heat-spreader",
                file: "heat-spreader.json",
            },
            path: "tmp/a.json".to_string(),
            bundle: json!({
                "study": "heat-spreader",
                "bundle_id": "bundle.a",
                "posture": "screening_research_bundle",
                "summary": {
                    "winner_candidate_id": "candidate-a",
                    "reliability_decision": "blocked_by_quality_gates",
                    "next_round_decision": "mitigate_design_risk",
                    "runnable_next_step_count": 3,
                    "next_iteration": 2,
                    "chain_stop_reason": "risk_mitigation_required",
                    "chain_convergence_state": "blocked_by_quality_gates",
                    "chain_round_count": 2,
                },
                "research_evidence": {
                    "candidate_count": 2,
                    "ranked_candidate_ids": ["candidate-a", "candidate-b"],
                    "winner_candidate_id": "candidate-a",
                    "primary_metric_ids": ["peak_temperature_c"],
                    "metric_objective_count": 1,
                    "violated_quality_gate_ids": ["gate.temperature"],
                    "focus_candidate_ids": ["candidate-a"],
                    "quality_gate_decision": "blocked_by_quality_gates",
                    "plan_decision": "mitigate_design_risk",
                    "plan_step_count": 3,
                    "chain_round_count": 2,
                    "chain_trace_round_count": 2,
                    "final_winner_candidate_id": "candidate-b",
                },
            }),
        }]);
        assert_eq!(
            index.pointer("/reliability_decision_counts/blocked_by_quality_gates"),
            Some(&json!(1))
        );
        assert_eq!(index.pointer("/bundles/0/next_iteration"), Some(&json!(2)));
        assert_eq!(
            index.pointer("/bundles/0/final_winner_candidate_id"),
            Some(&json!("candidate-b"))
        );
        assert_eq!(
            index.pointer("/winner_changed_in_chain_count"),
            Some(&json!(1))
        );
    }
}
