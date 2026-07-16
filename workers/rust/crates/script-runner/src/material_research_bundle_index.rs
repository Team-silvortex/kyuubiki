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

#[path = "material_research_bundle_index_self_test.rs"]
mod material_research_bundle_index_self_test;

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
        material_research_bundle_index_self_test::run_self_test()
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
        material_research_bundle_index_self_test::run_check_self_test()
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
pub(super) struct BundleProfile {
    pub(super) study: &'static str,
    pub(super) file: &'static str,
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

pub(super) struct IndexEntry {
    pub(super) profile: BundleProfile,
    pub(super) path: String,
    pub(super) bundle: Value,
}

pub(super) fn build_index(entries: Vec<IndexEntry>) -> Value {
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
        "validation_priority_counts": counts_by(&bundles, "validation_priority"),
        "bundles": bundles,
    })
}

pub(super) fn validate_index(index: &Value) -> RunnerResult<()> {
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
    assert_count_map(
        index,
        bundles,
        "validation_priority_counts",
        "validation_priority",
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
        "validation_priority",
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
        "validation_priority_rank",
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
    let reasons = string_array(bundle.get("validation_blocking_reasons"));
    if !reasons
        .iter()
        .any(|reason| *reason == "external_validation_required")
    {
        return Err(format!(
            "{context}.validation_blocking_reasons must include external_validation_required"
        ));
    }
    if array_len(bundle, "violated_quality_gate_ids") > 0
        && !reasons
            .iter()
            .any(|reason| *reason == "violated_quality_gates")
    {
        return Err(format!(
            "{context}.validation_blocking_reasons must include violated_quality_gates"
        ));
    }
    if bundle
        .pointer("/candidate_confidence_counts/low")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
        && !reasons
            .iter()
            .any(|reason| *reason == "low_confidence_material_cards")
    {
        return Err(format!(
            "{context}.validation_blocking_reasons must include low_confidence_material_cards"
        ));
    }
    let (expected_priority, expected_rank) = validation_priority_for_index_bundle(bundle);
    if field(bundle, "validation_priority") != expected_priority
        || bundle
            .get("validation_priority_rank")
            .and_then(Value::as_u64)
            != Some(expected_rank)
    {
        return Err(format!(
            "{context}.validation_priority must match index evidence"
        ));
    }
    if array_len(bundle, "validation_priority_reasons") == 0 {
        return Err(format!(
            "{context}.validation_priority_reasons must be non-empty"
        ));
    }
    if string_array(bundle.get("validation_priority_reasons"))
        != validation_priority_reasons_for_index_bundle(bundle)
    {
        return Err(format!(
            "{context}.validation_priority_reasons must match index evidence"
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
    let (validation_priority, validation_priority_rank, validation_priority_reasons) =
        validation_priority_for_bundle(bundle, winner_changed);
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
        "validation_priority": validation_priority,
        "validation_priority_rank": validation_priority_rank,
        "validation_priority_reasons": validation_priority_reasons,
        "profile_study": profile.study,
    })
}

fn validation_priority_for_bundle(
    bundle: &Value,
    winner_changed: bool,
) -> (&'static str, u64, Vec<Value>) {
    let mut reasons = Vec::new();
    if winner_changed {
        reasons.push(Value::from("winner_changed_in_chain"));
        return ("p0_validation_repair", 0, reasons);
    }
    let has_gate = bundle
        .pointer("/research_evidence/violated_quality_gate_ids")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    let has_low_confidence = bundle
        .pointer("/validation_evidence/candidate_confidence_counts/low")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;
    if has_gate {
        reasons.push(Value::from("violated_quality_gates"));
    }
    if has_low_confidence {
        reasons.push(Value::from("low_confidence_material_cards"));
    }
    if has_gate || has_low_confidence {
        ("p1_validation_repair", 1, reasons)
    } else {
        (
            "p2_validation_followup",
            2,
            vec![Value::from("screening_followup")],
        )
    }
}

fn validation_priority_for_index_bundle(bundle: &Value) -> (&'static str, u64) {
    if bundle
        .get("winner_changed_in_chain")
        .and_then(Value::as_bool)
        == Some(true)
    {
        return ("p0_validation_repair", 0);
    }
    let has_gate = array_len(bundle, "violated_quality_gate_ids") > 0;
    let has_low_confidence = bundle
        .pointer("/candidate_confidence_counts/low")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;
    if has_gate || has_low_confidence {
        ("p1_validation_repair", 1)
    } else {
        ("p2_validation_followup", 2)
    }
}

fn validation_priority_reasons_for_index_bundle(bundle: &Value) -> Vec<&'static str> {
    if bundle
        .get("winner_changed_in_chain")
        .and_then(Value::as_bool)
        == Some(true)
    {
        return vec!["winner_changed_in_chain"];
    }
    let mut reasons = Vec::new();
    if array_len(bundle, "violated_quality_gate_ids") > 0 {
        reasons.push("violated_quality_gates");
    }
    if bundle
        .pointer("/candidate_confidence_counts/low")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        reasons.push("low_confidence_material_cards");
    }
    if reasons.is_empty() {
        reasons.push("screening_followup");
    }
    reasons
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
        format!(
            "Validation priority counts: {}",
            index
                .get("validation_priority_counts")
                .unwrap_or(&Value::Null)
        ),
        String::new(),
        "| Study | Winner | Final winner | Metrics | Gates | Priority | Validation | Next round | Chain |"
            .to_string(),
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |".to_string(),
    ];
    for bundle in index
        .get("bundles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` rank=`{}` reasons=`{}` | `{}` score=`{}` reasons=`{}` actions=`{}` | `{}@{}` steps=`{}` | `{}/{}` rounds=`{}` trace=`{}` |",
            field(bundle, "study"),
            field(bundle, "winner_candidate_id"),
            field(bundle, "final_winner_candidate_id"),
            array_len(bundle, "primary_metric_ids"),
            array_len(bundle, "violated_quality_gate_ids"),
            field(bundle, "validation_priority"),
            bundle
                .get("validation_priority_rank")
                .unwrap_or(&Value::Null),
            array_len(bundle, "validation_priority_reasons"),
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

fn string_array(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

#[cfg(test)]
#[path = "material_research_bundle_index_tests.rs"]
mod material_research_bundle_index_tests;
