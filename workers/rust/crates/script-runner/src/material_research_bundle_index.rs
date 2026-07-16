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
const DEFAULT_INDEX_INPUT: &str = "tmp/material-research-bundles/index.json";
const INDEX_SCHEMA_VERSION: &str = "kyuubiki.material-research-bundle-index/v1";
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

pub(crate) fn run_check_material_research_bundle_index(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_check_args(args)?;
    let result = if options.self_test {
        run_check_self_test()
    } else {
        check_index_file(root, &options.input)
    };
    match result {
        Ok(message) => {
            println!("{message}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("material research bundle index check failed: {issue}");
            Ok(1)
        }
    }
}

struct Options {
    out_dir: String,
    ensure_bundles: bool,
    self_test: bool,
}

struct CheckOptions {
    input: String,
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

fn parse_check_args(args: Vec<OsString>) -> RunnerResult<CheckOptions> {
    let mut options = CheckOptions {
        input: DEFAULT_INDEX_INPUT.to_string(),
        self_test: false,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner check-material-research-bundle-index [--self-test] [--in tmp/material-research-bundles/index.json]"
                );
                return Ok(options);
            }
            "--self-test" => options.self_test = true,
            "--in" => {
                options.input = iter
                    .next()
                    .map(|value| value.to_string_lossy().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "--in requires a repo-local path".to_string())?;
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
            "validation_evidence": {
                "validation_posture": "screening_validation",
                "baseline_refs": [{ "baseline_id": "baseline-a" }],
                "candidate_confidence_counts": { "low": 1, "medium": 1, "high": 0, "unknown": 0 },
                "acceptance_criteria": [{ "criterion_id": "gate.temperature" }],
                "uncertainty_summary": { "external_validation_required": true },
                "validation_readiness": {
                    "decision": "screening_only",
                    "score": 0.4,
                    "blocking_reasons": ["external_validation_required"],
                    "next_validation_actions": ["run_external_solver_or_analytic_baseline"],
                },
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

fn run_check_self_test() -> RunnerResult<String> {
    let mut index = build_index(vec![IndexEntry {
        profile: BundleProfile {
            study: "heat-spreader",
            file: "a.json",
        },
        path: "tmp/a.json".to_string(),
        bundle: self_test_bundle(),
    }]);
    validate_index(&index)?;
    index["winner_changed_in_chain_count"] = Value::from(0);
    if validate_index(&index).is_ok() {
        return Err("self-test did not reject winner drift count mismatch".to_string());
    }
    Ok("material research bundle index check self-test passed".to_string())
}

fn self_test_bundle() -> Value {
    json!({
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
        "validation_evidence": {
            "validation_posture": "screening_validation",
            "baseline_refs": [{ "baseline_id": "baseline-a" }],
            "candidate_confidence_counts": { "low": 1, "medium": 1, "high": 0, "unknown": 0 },
            "acceptance_criteria": [{ "criterion_id": "gate.temperature" }],
            "uncertainty_summary": { "external_validation_required": true },
            "validation_readiness": {
                "decision": "screening_only",
                "score": 0.4,
                "blocking_reasons": ["external_validation_required"],
                "next_validation_actions": ["run_external_solver_or_analytic_baseline"],
            },
        },
    })
}

fn check_index_file(root: &Path, input: &str) -> RunnerResult<String> {
    let (path, relative) = repo_local_path(root, input, "--in")?;
    let text =
        fs::read_to_string(&path).map_err(|error| format!("failed to read {relative}: {error}"))?;
    let index: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{relative}: invalid json: {error}"))?;
    validate_index(&index)?;
    Ok(format!("material research bundle index ok: {relative}"))
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
        "schema_version": INDEX_SCHEMA_VERSION,
        "generated_at_utc": utc_iso_timestamp(),
        "bundle_count": bundles.len(),
        "studies": bundles.iter().filter_map(|bundle| bundle.get("study").cloned()).collect::<Vec<_>>(),
        "winner_changed_in_chain_count": bundles.iter().filter(|bundle| bundle.get("winner_changed_in_chain").and_then(Value::as_bool) == Some(true)).count(),
        "reliability_decision_counts": counts_by(&bundles, "reliability_decision"),
        "next_round_decision_counts": counts_by(&bundles, "next_round_decision"),
        "bundles": bundles,
    })
}

fn validate_index(index: &Value) -> RunnerResult<()> {
    if field(index, "schema_version") != INDEX_SCHEMA_VERSION {
        return Err(format!("schema_version must be {INDEX_SCHEMA_VERSION}"));
    }
    let bundles = index
        .get("bundles")
        .and_then(Value::as_array)
        .ok_or_else(|| "bundles must be an array".to_string())?;
    let bundle_count = index
        .get("bundle_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| "bundle_count must be an integer".to_string())?;
    if bundle_count != bundles.len() as u64 {
        return Err("bundle_count must match bundles length".to_string());
    }
    let studies = index
        .get("studies")
        .and_then(Value::as_array)
        .ok_or_else(|| "studies must be an array".to_string())?;
    if studies.len() != bundles.len() {
        return Err("studies length must match bundles length".to_string());
    }
    for (index, bundle) in bundles.iter().enumerate() {
        validate_index_bundle(bundle, index)?;
    }
    assert_count_map(
        index,
        bundles,
        "reliability_decision_counts",
        "reliability_decision",
    )?;
    assert_count_map(
        index,
        bundles,
        "next_round_decision_counts",
        "next_round_decision",
    )?;
    let changed = bundles
        .iter()
        .filter(|bundle| {
            bundle
                .get("winner_changed_in_chain")
                .and_then(Value::as_bool)
                == Some(true)
        })
        .count() as u64;
    if index
        .get("winner_changed_in_chain_count")
        .and_then(Value::as_u64)
        != Some(changed)
    {
        return Err("winner_changed_in_chain_count must match bundle rows".to_string());
    }
    Ok(())
}

pub(crate) fn validate_material_research_bundle_index_value(index: &Value) -> RunnerResult<()> {
    validate_index(index)
}

fn validate_index_bundle(bundle: &Value, index: usize) -> RunnerResult<()> {
    let context = format!("bundles[{index}]");
    for key in [
        "study",
        "bundle_id",
        "path",
        "posture",
        "winner_candidate_id",
        "final_winner_candidate_id",
        "reliability_decision",
        "next_round_decision",
        "chain_stop_reason",
        "chain_convergence_state",
        "validation_readiness_decision",
        "profile_study",
    ] {
        if field(bundle, key).is_empty() {
            return Err(format!("{context}.{key} must be a non-empty string"));
        }
    }
    for key in [
        "runnable_next_step_count",
        "next_iteration",
        "chain_round_count",
        "chain_trace_round_count",
        "research_candidate_count",
        "metric_objective_count",
        "baseline_ref_count",
        "acceptance_criteria_count",
        "next_validation_action_count",
    ] {
        if bundle.get(key).and_then(Value::as_u64).is_none() {
            return Err(format!("{context}.{key} must be an integer"));
        }
    }
    if !matches!(bundle.get("winner_changed_in_chain"), Some(Value::Bool(_))) {
        return Err(format!(
            "{context}.winner_changed_in_chain must be a boolean"
        ));
    }
    let expected_changed =
        field(bundle, "winner_candidate_id") != field(bundle, "final_winner_candidate_id");
    if bundle
        .get("winner_changed_in_chain")
        .and_then(Value::as_bool)
        != Some(expected_changed)
    {
        return Err(format!(
            "{context}.winner_changed_in_chain must match winner/final winner"
        ));
    }
    if array_len(bundle, "primary_metric_ids") == 0 {
        return Err(format!("{context}.primary_metric_ids must be non-empty"));
    }
    if array_len(bundle, "focus_candidate_ids") == 0 {
        return Err(format!("{context}.focus_candidate_ids must be non-empty"));
    }
    if bundle
        .get("chain_trace_round_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
    {
        return Err(format!(
            "{context}.chain_trace_round_count must be positive"
        ));
    }
    if field(bundle, "validation_posture") != "screening_validation" {
        return Err(format!(
            "{context}.validation_posture must be screening_validation"
        ));
    }
    if bundle
        .get("external_validation_required")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(format!(
            "{context}.external_validation_required must be true"
        ));
    }
    if bundle
        .get("candidate_confidence_counts")
        .and_then(Value::as_object)
        .is_none()
    {
        return Err(format!(
            "{context}.candidate_confidence_counts must be an object"
        ));
    }
    if field(bundle, "validation_readiness_decision") != "screening_only" {
        return Err(format!(
            "{context}.validation_readiness_decision must be screening_only"
        ));
    }
    let readiness_score = bundle
        .get("validation_readiness_score")
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{context}.validation_readiness_score must be a number"))?;
    if !(0.0..=1.0).contains(&readiness_score) {
        return Err(format!(
            "{context}.validation_readiness_score must be between 0 and 1"
        ));
    }
    if array_len(bundle, "validation_blocking_reasons") == 0 {
        return Err(format!(
            "{context}.validation_blocking_reasons must be non-empty"
        ));
    }
    Ok(())
}

fn assert_count_map(
    index: &Value,
    bundles: &[Value],
    map_key: &str,
    row_key: &str,
) -> RunnerResult<()> {
    if index.get(map_key) != Some(&counts_by(bundles, row_key)) {
        return Err(format!("{map_key} must match bundles.{row_key}"));
    }
    Ok(())
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
        "validation_posture": pointer_or_null(bundle, "/validation_evidence/validation_posture"),
        "external_validation_required": pointer_or_null(bundle, "/validation_evidence/uncertainty_summary/external_validation_required"),
        "baseline_ref_count": array_count(bundle.pointer("/validation_evidence/baseline_refs")),
        "acceptance_criteria_count": array_count(bundle.pointer("/validation_evidence/acceptance_criteria")),
        "candidate_confidence_counts": pointer_or_null(bundle, "/validation_evidence/candidate_confidence_counts"),
        "validation_readiness_decision": pointer_or_null(bundle, "/validation_evidence/validation_readiness/decision"),
        "validation_readiness_score": pointer_or_null(bundle, "/validation_evidence/validation_readiness/score"),
        "validation_blocking_reasons": pointer_or_null(bundle, "/validation_evidence/validation_readiness/blocking_reasons"),
        "next_validation_action_count": array_count(bundle.pointer("/validation_evidence/validation_readiness/next_validation_actions")),
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
        "| Study | Winner | Final winner | Metrics | Gates | Validation | Next round | Chain |"
            .to_string(),
        "| --- | --- | --- | --- | --- | --- | --- | --- |".to_string(),
    ];
    for bundle in index
        .get("bundles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` score=`{}` reasons=`{}` actions=`{}` | `{}@{}` steps=`{}` | `{}/{}` rounds=`{}` trace=`{}` |",
            field(bundle, "study"),
            field(bundle, "winner_candidate_id"),
            field(bundle, "final_winner_candidate_id"),
            array_len(bundle, "primary_metric_ids"),
            array_len(bundle, "violated_quality_gate_ids"),
            field(bundle, "validation_readiness_decision"),
            bundle
                .get("validation_readiness_score")
                .unwrap_or(&Value::Null),
            array_len(bundle, "validation_blocking_reasons"),
            bundle
                .get("next_validation_action_count")
                .unwrap_or(&Value::Null),
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

fn array_count(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map(Vec::len).unwrap_or(0)
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
                "validation_evidence": {
                    "validation_posture": "screening_validation",
                    "baseline_refs": [{ "baseline_id": "baseline-a" }],
                    "candidate_confidence_counts": { "low": 1, "medium": 1, "high": 0, "unknown": 0 },
                    "acceptance_criteria": [{ "criterion_id": "gate.temperature" }],
                    "uncertainty_summary": { "external_validation_required": true },
                    "validation_readiness": {
                        "decision": "screening_only",
                        "score": 0.4,
                        "blocking_reasons": ["external_validation_required"],
                        "next_validation_actions": ["run_external_solver_or_analytic_baseline"],
                    },
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
