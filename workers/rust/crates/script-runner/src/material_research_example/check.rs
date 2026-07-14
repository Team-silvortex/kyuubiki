use serde_json::Value;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use super::{repo_local_path, required_value};
use support::{
    array, assert_eq_str, assert_eq_value, assert_finite, assert_no_absolute_repo_path, field,
    read_json_path, string_array, validate_convergence_assessment, validate_documentation,
    validate_next_round_lineage, value,
};

mod support;

const DEFAULT_INPUT: &str = "tmp/material-research-example.json";
const MANIFEST_PATH: &str = "docs/automated-material-research-example.manifest.json";

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_check_material_research_example(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let input = parse_check_args(args)?;
    let (absolute, _relative) = repo_local_path(root, &input, "--in")?;
    if !absolute.exists() {
        eprintln!("material research example check failed: input does not exist: {input}");
        return Ok(1);
    }
    let evidence = read_json_path(&absolute, &input)?;
    let manifest = read_json_path(&root.join(MANIFEST_PATH), MANIFEST_PATH)?;
    match validate_material_research_example(root, &input, &evidence, &manifest) {
        Ok(()) => {
            println!("material research example ok: {input}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("material research example check failed: {issue}");
            Ok(1)
        }
    }
}

fn parse_check_args(args: Vec<OsString>) -> RunnerResult<String> {
    let mut input = DEFAULT_INPUT.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner check-material-research-example [--in tmp/material-research-example.json]"
                );
                return Ok(input);
            }
            "--in" => input = required_value(&mut iter, "--in")?,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(input)
}

fn validate_material_research_example(
    root: &Path,
    input: &str,
    evidence: &Value,
    manifest: &Value,
) -> RunnerResult<()> {
    validate_documentation(root, manifest)?;
    let expected = value(manifest, "expected");
    assert_eq_value(
        evidence.get("schema_version"),
        expected.get("evidence_schema_version"),
        "evidence schema",
    )?;
    assert_eq_value(
        evidence.get("example_id"),
        manifest.get("example_id"),
        "example_id",
    )?;
    assert_eq_str(
        field(evidence, "posture"),
        "screening_research_example",
        "posture",
    )?;
    assert_eq_value(
        evidence.pointer("/summary/winner_candidate_id"),
        expected.get("winner_candidate_id"),
        "summary winner",
    )?;
    assert_eq_value(
        evidence.pointer("/summary/iteration"),
        expected.get("initial_iteration"),
        "summary iteration",
    )?;
    assert_eq_value(
        evidence.pointer("/summary/next_round_iteration"),
        expected.get("next_run_iteration"),
        "summary next_round_iteration",
    )?;
    assert_eq_value(
        evidence.pointer("/summary/result_payload_count"),
        expected.get("result_payload_count"),
        "summary result count",
    )?;
    validate_command(value(evidence, "command"), manifest)?;
    validate_exploration(value(evidence, "exploration"), manifest)?;
    assert_no_absolute_repo_path(root, evidence, "evidence")?;
    validate_next_round_command(root, input, manifest)?;
    validate_run_next_command(root, input, manifest)?;
    validate_chain_next_command(root, input, manifest)
}

fn validate_command(command: &Value, manifest: &Value) -> RunnerResult<()> {
    let command_manifest = value(manifest, "command");
    assert_eq_value(command.get("id"), command_manifest.get("id"), "command.id")?;
    assert_eq_value(
        command.get("cwd"),
        command_manifest.get("cwd"),
        "command.cwd",
    )?;
    let argv = string_array(command.get("argv"));
    if argv.len() < 8 {
        return Err("command.argv must include the cargo material exploration command".to_string());
    }
    for expected in string_array(command_manifest.get("argv_contains")) {
        if !argv.contains(&expected) {
            return Err(format!("command.argv missing {expected}"));
        }
    }
    if command.get("ok").and_then(Value::as_bool) != Some(true)
        || command.get("status").and_then(Value::as_i64) != Some(0)
    {
        return Err("command must pass with status 0".to_string());
    }
    assert_finite(command.get("duration_ms"), "command.duration_ms")
}

fn validate_exploration(exploration: &Value, manifest: &Value) -> RunnerResult<()> {
    let expected = value(manifest, "expected");
    assert_eq_value(
        exploration.get("schema_version"),
        expected.get("exploration_schema_version"),
        "exploration schema",
    )?;
    assert_eq_value(
        exploration.get("study"),
        expected.get("exploration_study"),
        "exploration.study",
    )?;
    assert_eq_value(
        exploration.get("mode"),
        expected.get("mode"),
        "exploration.mode",
    )?;
    assert_eq_value(
        exploration.get("iteration"),
        expected.get("initial_iteration"),
        "exploration.iteration",
    )?;
    assert_eq_value(
        exploration.get("candidate_count"),
        expected.get("candidate_count"),
        "exploration.candidate_count",
    )?;
    let results = array(exploration, "result_payloads");
    if Some(results.len() as u64) != expected.get("result_payload_count").and_then(Value::as_u64) {
        return Err(
            "exploration.result_payloads must contain exactly three solver outputs".to_string(),
        );
    }
    for (index, result) in results.iter().enumerate() {
        assert_finite(
            result.get("max_temperature"),
            &format!("result_payloads[{index}].max_temperature"),
        )?;
        assert_finite(
            result.get("max_heat_flux"),
            &format!("result_payloads[{index}].max_heat_flux"),
        )?;
        if array(result, "nodes").len() != 4 {
            return Err(format!(
                "result_payloads[{index}] must expose four thermal nodes"
            ));
        }
    }
    validate_report(value(exploration, "report"), manifest)?;
    validate_next_round(value(exploration, "next_round"), manifest)
}

fn validate_report(report: &Value, manifest: &Value) -> RunnerResult<()> {
    let expected = value(manifest, "expected");
    assert_eq_value(
        report.get("schema_version"),
        expected.get("report_schema_version"),
        "report schema",
    )?;
    assert_eq_value(
        report.get("winner_candidate_id"),
        expected.get("winner_candidate_id"),
        "winner_candidate_id",
    )?;
    assert_eq_value(
        report.pointer("/optimization/id"),
        expected.get("optimization_id"),
        "optimization.id",
    )?;
    assert_eq_value(
        report.pointer("/reliability/posture"),
        expected.get("reliability_posture"),
        "reliability.posture",
    )?;
    for section in string_array(manifest.get("required_report_sections")) {
        if report.get(section).is_none() {
            return Err(format!("report missing section {section}"));
        }
    }
    for section in string_array(manifest.get("required_reliability_sections")) {
        if value(report, "reliability").get(section).is_none() {
            return Err(format!("reliability missing section {section}"));
        }
    }
    let candidates = array(report, "candidates");
    if Some(candidates.len() as u64) != expected.get("candidate_count").and_then(Value::as_u64) {
        return Err("report.candidates must contain exactly three candidates".to_string());
    }
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.get("rank").and_then(Value::as_u64) != Some(index as u64 + 1) {
            return Err(format!("candidate[{index}].rank: expected {}", index + 1));
        }
        for field_name in string_array(manifest.get("required_candidate_fields")) {
            if candidate.get(field_name).is_none() {
                return Err(format!("candidate[{index}] missing {field_name}"));
            }
        }
        for field_name in ["score", "peak_temperature_c", "areal_mass_kg_m2"] {
            assert_finite(
                candidate.get(field_name),
                &format!("candidate[{index}].{field_name}"),
            )?;
        }
        if array(candidate, "optimization_terms").len() < 3 {
            return Err(format!("candidate[{index}] must expose optimization terms"));
        }
    }
    if array(value(report, "reliability"), "quality_gates").len() < 3 {
        return Err("reliability quality gates must be present".to_string());
    }
    if array(value(report, "reliability"), "limitations").len() < 3 {
        return Err("reliability limitations must stay visible".to_string());
    }
    Ok(())
}

fn validate_next_round(next_round: &Value, manifest: &Value) -> RunnerResult<()> {
    if !next_round.is_object() {
        return Err("exploration.next_round must be present".to_string());
    }
    let expected = value(manifest, "expected");
    assert_eq_value(
        next_round.get("schema_version"),
        expected.get("next_round_schema_version"),
        "next_round schema",
    )?;
    for section in string_array(manifest.get("required_next_round_sections")) {
        if next_round.get(section).is_none() {
            return Err(format!("next_round missing section {section}"));
        }
    }
    if !string_array(manifest.get("allowed_next_round_decisions"))
        .contains(&field(next_round, "decision"))
    {
        return Err(format!(
            "next_round.decision is not allowed: {}",
            field(next_round, "decision")
        ));
    }
    if next_round
        .get("iteration")
        .and_then(Value::as_i64)
        .is_none_or(|value| value < 2)
    {
        return Err("next_round.iteration must point at a future iteration".to_string());
    }
    for field_name in ["focus_candidate_ids", "actions", "rationale"] {
        if array(next_round, field_name).is_empty() {
            return Err(format!(
                "next_round.{field_name} must include at least one entry"
            ));
        }
    }
    Ok(())
}

fn validate_next_round_command(root: &Path, input: &str, manifest: &Value) -> RunnerResult<()> {
    let plan = run_material_explore(root, &["--plan-next", &root.join(input).to_string_lossy()])?;
    let expected = value(manifest, "expected");
    assert_eq_value(
        plan.get("schema_version"),
        expected.get("next_round_execution_schema_version"),
        "next round execution schema",
    )?;
    if array(&plan, "steps").is_empty() {
        return Err("next round execution plan must include runnable steps".to_string());
    }
    assert_finite(
        plan.get("runnable_step_count"),
        "next_round_execution.runnable_step_count",
    )?;
    if plan.get("runnable_step_count").and_then(Value::as_u64)
        != Some(array(&plan, "steps").len() as u64)
    {
        return Err("next round runnable step count: expected steps length".to_string());
    }
    validate_optimization_objectives(
        value(&plan, "optimization_objectives"),
        manifest,
        "next_round_execution",
    )
}

fn validate_run_next_command(root: &Path, input: &str, manifest: &Value) -> RunnerResult<()> {
    let exploration =
        run_material_explore(root, &["--run-next", &root.join(input).to_string_lossy()])?;
    let expected = value(manifest, "expected");
    assert_eq_value(
        exploration.get("schema_version"),
        expected.get("exploration_schema_version"),
        "run-next exploration schema",
    )?;
    assert_eq_str(
        field(&exploration, "mode"),
        "local_solver_next_round",
        "run-next mode",
    )?;
    assert_eq_value(
        exploration.get("iteration"),
        expected.get("next_run_iteration"),
        "run-next iteration",
    )?;
    assert_eq_value(
        exploration.get("candidate_count"),
        expected.get("candidate_count"),
        "run-next candidate count",
    )?;
    validate_next_round(value(&exploration, "next_round"), manifest)?;
    assert_eq_value(
        exploration.pointer("/next_round/iteration"),
        expected.get("next_run_next_round_iteration"),
        "run-next next_round iteration",
    )?;
    validate_next_round_lineage(value(&exploration, "lineage"), manifest)
}

fn validate_chain_next_command(root: &Path, input: &str, manifest: &Value) -> RunnerResult<()> {
    let rounds = value(manifest, "chain_next_command")
        .get("rounds")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .to_string();
    let chain = run_material_explore(
        root,
        &[
            "--chain-next",
            &root.join(input).to_string_lossy(),
            "--rounds",
            &rounds,
        ],
    )?;
    let expected = value(manifest, "expected");
    assert_eq_value(
        chain.get("schema_version"),
        expected.get("chain_schema_version"),
        "chain-next schema",
    )?;
    assert_eq_value(
        chain.get("round_count"),
        expected.get("chain_round_count"),
        "chain round count",
    )?;
    assert_eq_value(
        chain.get("final_iteration"),
        expected.get("chain_final_iteration"),
        "chain final iteration",
    )?;
    assert_eq_value(
        chain.get("stop_reason"),
        expected.get("chain_stop_reason"),
        "chain stop reason",
    )?;
    assert_eq_value(
        chain.get("all_winners_stable"),
        expected.get("chain_all_winners_stable"),
        "chain winner stability",
    )?;
    validate_convergence_assessment(value(&chain, "convergence_assessment"), manifest)?;
    if !value(&chain, "decision_counts").is_object() {
        return Err("chain-next must expose decision_counts".to_string());
    }
    let round_count = expected
        .get("chain_round_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    if array(&chain, "optimization_trace").len() != round_count {
        return Err(
            "chain-next must expose one optimization_trace entry per requested round".to_string(),
        );
    }
    for (index, trace) in array(&chain, "optimization_trace").iter().enumerate() {
        if array(trace, "primary_metric_ids").is_empty() {
            return Err(format!(
                "optimization_trace[{index}] must expose primary_metric_ids"
            ));
        }
    }
    if !value(&chain, "repair_summary").is_object() || !value(&chain, "repair_plan").is_object() {
        return Err("chain-next must expose repair_summary and repair_plan".to_string());
    }
    if array(&chain, "runs").len() != round_count || array(&chain, "summaries").len() != round_count
    {
        return Err(
            "chain-next must expose one exploration artifact and summary per requested round"
                .to_string(),
        );
    }
    assert_eq_value(
        array(&chain, "summaries")
            .last()
            .and_then(|summary| summary.get("iteration")),
        expected.get("chain_final_iteration"),
        "chain final summary iteration",
    )?;
    for (index, summary) in array(&chain, "summaries").iter().enumerate() {
        validate_optimization_objectives(
            value(summary, "optimization_objectives"),
            manifest,
            &format!("chain.summaries[{index}]"),
        )?;
        assert_finite(
            summary.get("winner_score"),
            &format!("chain.summaries[{index}].winner_score"),
        )?;
    }
    Ok(())
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

fn validate_optimization_objectives(
    objectives: &Value,
    manifest: &Value,
    context: &str,
) -> RunnerResult<()> {
    assert_eq_value(
        objectives.get("schema_version"),
        value(manifest, "expected").get("next_round_optimization_objectives_schema_version"),
        &format!("{context}.optimization_objectives schema"),
    )?;
    if array(objectives, "primary_metric_ids").is_empty()
        || array(objectives, "metric_objectives").is_empty()
    {
        return Err(format!(
            "{context}.optimization_objectives must expose metric objectives"
        ));
    }
    Ok(())
}
