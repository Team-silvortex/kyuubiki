use std::fs;

#[path = "kyuubiki-material-explore/chain.rs"]
mod chain;
#[path = "kyuubiki-material-explore/display.rs"]
mod display;
#[path = "kyuubiki-material-explore/flags.rs"]
mod flags;
#[path = "kyuubiki-material-explore/materialization.rs"]
mod materialization;
#[cfg(test)]
#[path = "kyuubiki-material-explore/materialization_tests.rs"]
mod materialization_tests;
#[cfg(test)]
#[path = "kyuubiki-material-explore/tests.rs"]
mod tests;

use kyuubiki_headless_sdk::{
    HeadlessWorkflowStep, build_material_exploration_next_round_execution_plan,
    build_material_exploration_run, build_material_exploration_run_for_iteration,
    build_material_study_execution_plan, describe_material_study, material_exploration_steps,
    material_study_catalog,
};
use kyuubiki_protocol::{
    SolveElectrostaticPlaneQuad2dRequest, SolveHeatPlaneQuad2dRequest, SolvePlaneQuad2dRequest,
    SolveThermalPlaneQuad2dRequest,
};
use kyuubiki_solver::{
    solve_electrostatic_plane_quad_2d, solve_heat_plane_quad_2d, solve_plane_quad_2d,
    solve_thermal_plane_quad_2d,
};
use serde::Serialize;
use serde_json::Value;

use chain::chain_next_rounds_from_initial;
use display::{
    print_catalog_summary, print_chain_summary, print_next_round_plan_summary,
    print_study_plan_summary, print_study_summary, print_summary,
};
use flags::Flags;
use materialization::{
    approve_review_template, materialize_reviewed_candidates, print_materialization_summary,
    print_materialized_rerun_summary, print_review_decision_summary, print_review_template_summary,
    required_flag, review_decision_template, run_materialized_candidates,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let flags = Flags::parse(std::env::args().skip(1).collect::<Vec<_>>())?;
    if flags.catalog {
        let catalog = material_catalog_payload()?;
        if let Some(out) = &flags.out {
            write_json(out, &catalog)?;
        }
        if flags.json {
            print_json(&catalog)?;
        } else {
            print_catalog_summary(&catalog);
        }
        return Ok(());
    }
    if let Some(study) = &flags.describe_study {
        let payload = material_study_payload(study)?;
        if let Some(out) = &flags.out {
            write_json(out, &payload)?;
        }
        if flags.json {
            print_json(&payload)?;
        } else {
            print_study_summary(&payload);
        }
        return Ok(());
    }
    if let Some(study) = &flags.plan_study {
        let payload = material_study_plan_payload(study)?;
        if let Some(out) = &flags.out {
            write_json(out, &payload)?;
        }
        if flags.json {
            print_json(&payload)?;
        } else {
            print_study_plan_summary(&payload);
        }
        return Ok(());
    }
    if let Some(path) = &flags.run_next {
        let exploration = run_next_round(path)?;
        if let Some(out) = &flags.out {
            write_json(out, &exploration)?;
        }
        if flags.json {
            print_json(&exploration)?;
        } else {
            print_summary(&exploration);
        }
        return Ok(());
    }
    if let Some(path) = &flags.review_template {
        let template = review_decision_template(path)?;
        if let Some(out) = &flags.out {
            write_json(out, &template)?;
        }
        if flags.json {
            print_json(&template)?;
        } else {
            print_review_template_summary(&template);
        }
        return Ok(());
    }
    if let Some(path) = &flags.approve_review_template {
        let reviewer_id = required_flag(&flags.reviewer_id, "--reviewer-id")?;
        let reason = required_flag(&flags.reason, "--reason")?;
        let decided_at = required_flag(&flags.decided_at, "--decided-at")?;
        let decision = approve_review_template(
            path,
            reviewer_id,
            flags.reviewer_name.as_deref().unwrap_or(reviewer_id),
            reason,
            decided_at,
        )?;
        if let Some(out) = &flags.out {
            write_json(out, &decision)?;
        }
        if flags.json {
            print_json(&decision)?;
        } else {
            print_review_decision_summary(&decision);
        }
        return Ok(());
    }
    if let Some(path) = &flags.materialize_reviewed {
        let decision_path = flags
            .review_decision
            .as_deref()
            .ok_or_else(|| "--materialize-reviewed requires --review-decision".to_string())?;
        let materialization = materialize_reviewed_candidates(path, decision_path)?;
        if let Some(out) = &flags.out {
            write_json(out, &materialization)?;
        }
        if flags.json {
            print_json(&materialization)?;
        } else {
            print_materialization_summary(&materialization);
        }
        return Ok(());
    }
    if let Some(path) = &flags.run_materialized {
        let rerun = run_materialized_candidates(path)?;
        if let Some(out) = &flags.out {
            write_json(out, &rerun)?;
        }
        if flags.json {
            print_json(&rerun)?;
        } else {
            print_materialized_rerun_summary(&rerun);
        }
        return Ok(());
    }
    if let Some(path) = &flags.chain_next {
        let chain = chain_next_rounds(path, flags.rounds)?;
        if let Some(out) = &flags.out {
            write_json(out, &chain)?;
        }
        if flags.json {
            print_json(&chain)?;
        } else {
            print_chain_summary(&chain);
        }
        return Ok(());
    }
    if let Some(path) = &flags.plan_next {
        let plan = plan_next_round(path)?;
        if let Some(out) = &flags.out {
            write_json(out, &plan)?;
        }
        if flags.json {
            print_json(&plan)?;
        } else {
            print_next_round_plan_summary(&plan);
        }
        return Ok(());
    }
    let exploration = run_material_exploration(&flags.study)?;
    if let Some(path) = &flags.out {
        write_json(path, &exploration)?;
    }
    if flags.json {
        print_json(&exploration)?;
    } else {
        print_summary(&exploration);
        if let Some(path) = &flags.out {
            println!("Exploration: {path}");
        }
    }
    Ok(())
}

fn material_catalog_payload() -> Result<Value, String> {
    let studies = material_study_catalog();
    Ok(serde_json::json!({
        "schema_version": "kyuubiki.material-study-catalog/v1",
        "study_count": studies.len(),
        "studies": studies,
        "next_steps": [
            "choose a study id or alias",
            "run kyuubiki-material-explore <study>",
            "inspect next_round decision",
            "use --plan-next, --review-template, --materialize-reviewed, and --run-materialized for the closed loop"
        ]
    }))
}

fn material_study_payload(study: &str) -> Result<Value, String> {
    let descriptor = describe_material_study(study)
        .ok_or_else(|| format!("unsupported material study: {study}"))?;
    Ok(serde_json::json!({
        "schema_version": "kyuubiki.material-study-description/v1",
        "study": descriptor,
        "id": descriptor.id,
        "domain": descriptor.domain,
        "template_id": descriptor.template_id,
        "report_schema_version": descriptor.schema_version,
        "metric_count": descriptor.metric_specs.len(),
        "metric_specs": descriptor.metric_specs,
        "recommended_flow": [
            "run the study with local solver kernels",
            "review report.winner_candidate_id and report.reliability.summary",
            "plan the next round with --plan-next",
            "materialize reviewed candidates only after review approval",
            "rerun materialized candidates with --run-materialized"
        ]
    }))
}

fn material_study_plan_payload(study: &str) -> Result<Value, String> {
    serde_json::to_value(build_material_study_execution_plan(study)?)
        .map_err(|error| error.to_string())
}

fn plan_next_round(path: &str) -> Result<Value, String> {
    let payload = read_json_file(path)?;
    let exploration = payload.get("exploration").cloned().unwrap_or(payload);
    let plan = build_material_exploration_next_round_execution_plan(&exploration)?;
    serde_json::to_value(plan).map_err(|error| error.to_string())
}

pub(crate) fn read_json_file(path: &str) -> Result<Value, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("failed to parse {path}: {error}"))
}

fn run_next_round(path: &str) -> Result<Value, String> {
    let previous = read_exploration_input(path)?;
    run_next_round_from_exploration(&previous)
}

fn chain_next_rounds(path: &str, rounds: usize) -> Result<Value, String> {
    let initial = read_exploration_input(path)?;
    chain_next_rounds_from_initial(initial, rounds, run_next_round_from_exploration)
}

fn run_next_round_from_exploration(previous: &Value) -> Result<Value, String> {
    let plan = build_material_exploration_next_round_execution_plan(&previous)?;
    let study = plan.study.clone();
    let iteration = plan.iteration;
    let mut result_payloads = previous_result_payloads(&previous)?;

    for step in &plan.steps {
        if !step.action.starts_with("solve_") {
            continue;
        }
        let result = run_solve_step(step)?;
        if let Some(candidate_id) = candidate_id_for_step(step) {
            if let Some(index) = candidate_index(&previous, candidate_id) {
                if index < result_payloads.len() {
                    result_payloads[index] = result;
                    continue;
                }
            }
        }
        result_payloads.push(result);
    }

    let run = build_material_exploration_run_for_iteration(
        &study,
        "local_solver_next_round",
        result_payloads,
        iteration,
    )?;
    let plan_value = serde_json::to_value(&plan).map_err(|error| error.to_string())?;
    let mut payload = serde_json::to_value(run).map_err(|error| error.to_string())?;
    attach_next_round_lineage(&mut payload, previous, &plan_value);
    Ok(payload)
}

fn read_exploration_input(path: &str) -> Result<Value, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let payload: Value =
        serde_json::from_str(&text).map_err(|error| format!("failed to parse {path}: {error}"))?;
    Ok(payload.get("exploration").cloned().unwrap_or(payload))
}

fn run_material_exploration(study: &str) -> Result<Value, String> {
    let steps = material_exploration_steps(study)?;
    let result_payloads = steps
        .iter()
        .filter(|step| step.action.starts_with("solve_"))
        .map(run_solve_step)
        .collect::<Result<Vec<_>, _>>()?;
    let run = build_material_exploration_run(study, "local_solver", result_payloads)?;
    serde_json::to_value(run).map_err(|error| error.to_string())
}

fn previous_result_payloads(exploration: &Value) -> Result<Vec<Value>, String> {
    exploration
        .get("result_payloads")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "previous exploration is missing result_payloads".to_string())
}

fn candidate_index(exploration: &Value, candidate_id: &str) -> Option<usize> {
    exploration
        .get("report")
        .and_then(|report| report.get("candidates"))
        .and_then(Value::as_array)?
        .iter()
        .position(|candidate| {
            candidate.get("candidate_id").and_then(Value::as_str) == Some(candidate_id)
        })
}

fn candidate_id_for_step(step: &HeadlessWorkflowStep) -> Option<&str> {
    step.payload
        .get("research")
        .and_then(|research| research.get("candidate_id"))
        .and_then(Value::as_str)
}

fn attach_next_round_lineage(payload: &mut Value, previous: &Value, plan: &Value) {
    let lineage = serde_json::json!({
        "schema_version": "kyuubiki.material-next-round-lineage/v1",
        "source_schema_version": previous.get("schema_version").and_then(Value::as_str),
        "source_iteration": previous.get("iteration").and_then(Value::as_u64),
        "source_winner_candidate_id": previous
            .get("report")
            .and_then(|report| report.get("winner_candidate_id"))
            .and_then(Value::as_str),
        "plan_schema_version": plan.get("schema_version").and_then(Value::as_str),
        "planned_iteration": plan.get("iteration").and_then(Value::as_u64),
        "decision": plan.get("decision").and_then(Value::as_str),
        "focus_candidate_ids": plan
            .get("focus_candidate_ids")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "material_card_refs": plan
            .get("material_card_refs")
            .cloned()
            .or_else(|| previous.get("material_card_refs").cloned())
            .unwrap_or_else(|| serde_json::json!([])),
        "optimization_objectives": plan
            .get("optimization_objectives")
            .cloned()
            .unwrap_or(Value::Null),
        "runnable_step_count": plan.get("runnable_step_count").and_then(Value::as_u64),
    });
    payload["lineage"] = lineage;
}

pub(crate) fn run_solve_step(step: &HeadlessWorkflowStep) -> Result<Value, String> {
    if step.action == "solve_composite_thermo_electric_panel" {
        return run_composite_solve_step(step);
    }
    let model = step
        .payload
        .get("model")
        .cloned()
        .ok_or_else(|| format!("{} step is missing model payload", step.action))?;
    match step.action.as_str() {
        "solve_heat_plane_quad_2d" => {
            let request: SolveHeatPlaneQuad2dRequest =
                serde_json::from_value(model).map_err(|error| error.to_string())?;
            to_value(solve_heat_plane_quad_2d(&request))
        }
        "solve_electrostatic_plane_quad_2d" => {
            let request: SolveElectrostaticPlaneQuad2dRequest =
                serde_json::from_value(model).map_err(|error| error.to_string())?;
            to_value(solve_electrostatic_plane_quad_2d(&request))
        }
        "solve_thermal_plane_quad_2d" => {
            let request: SolveThermalPlaneQuad2dRequest =
                serde_json::from_value(model).map_err(|error| error.to_string())?;
            to_value(solve_thermal_plane_quad_2d(&request))
        }
        "solve_plane_quad_2d" => {
            let request: SolvePlaneQuad2dRequest =
                serde_json::from_value(model).map_err(|error| error.to_string())?;
            to_value(solve_plane_quad_2d(&request))
        }
        other => Err(format!("unsupported material solve action: {other}")),
    }
}

fn run_composite_solve_step(step: &HeadlessWorkflowStep) -> Result<Value, String> {
    let electrostatic_request: SolveElectrostaticPlaneQuad2dRequest =
        serde_json::from_value(required_payload(step, "electrostatic_model")?)
            .map_err(|error| error.to_string())?;
    let heat_request: SolveHeatPlaneQuad2dRequest =
        serde_json::from_value(required_payload(step, "heat_model")?)
            .map_err(|error| error.to_string())?;
    let thermal_request: SolveThermalPlaneQuad2dRequest =
        serde_json::from_value(required_payload(step, "thermal_model")?)
            .map_err(|error| error.to_string())?;
    let electrostatic = solve_electrostatic_plane_quad_2d(&electrostatic_request)
        .map_err(|error| format!("composite electrostatic solve failed: {error}"))?;
    let heat = solve_heat_plane_quad_2d(&heat_request)
        .map_err(|error| format!("composite heat solve failed: {error}"))?;
    let thermal = solve_thermal_plane_quad_2d(&thermal_request)
        .map_err(|error| format!("composite thermal solve failed: {error}"))?;
    Ok(serde_json::json!({
        "schema_version": "kyuubiki.composite-thermo-electric-panel-result/v1",
        "research": step.payload.get("research").cloned().unwrap_or(Value::Null),
        "electrostatic": electrostatic,
        "heat": heat,
        "thermal": thermal,
    }))
}

fn required_payload(step: &HeadlessWorkflowStep, key: &str) -> Result<Value, String> {
    step.payload
        .get(key)
        .cloned()
        .ok_or_else(|| format!("{} step is missing {key}", step.action))
}

fn to_value<T: Serialize>(result: Result<T, String>) -> Result<Value, String> {
    serde_json::to_value(result?).map_err(|error| error.to_string())
}

fn write_json(path: &str, payload: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(payload).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| format!("failed to write {path}: {error}"))
}

fn print_json(payload: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(payload).map_err(|error| error.to_string())?;
    println!("{text}");
    Ok(())
}
