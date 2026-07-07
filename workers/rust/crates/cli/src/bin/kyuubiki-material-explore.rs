use std::fs;

#[path = "kyuubiki-material-explore/chain.rs"]
mod chain;
#[path = "kyuubiki-material-explore/display.rs"]
mod display;
#[path = "kyuubiki-material-explore/flags.rs"]
mod flags;
#[cfg(test)]
#[path = "kyuubiki-material-explore/tests.rs"]
mod tests;

use kyuubiki_headless_sdk::{
    HeadlessWorkflowStep, build_composite_materialized_candidate_report,
    build_composite_materialized_candidate_steps,
    build_material_exploration_next_round_execution_plan,
    build_material_exploration_next_round_plan, build_material_exploration_run,
    build_material_exploration_run_for_iteration, material_exploration_steps,
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
use display::{print_chain_summary, print_next_round_plan_summary, print_summary};
use flags::Flags;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let flags = Flags::parse(std::env::args().skip(1).collect::<Vec<_>>())?;
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

fn run_materialized_candidates(path: &str) -> Result<Value, String> {
    let plan = read_materialization_plan(path)?;
    let steps = build_composite_materialized_candidate_steps(&plan)?;
    let result_payloads = steps
        .iter()
        .map(run_solve_step)
        .collect::<Result<Vec<_>, _>>()?;
    let report = build_composite_materialized_candidate_report(&result_payloads)?;
    let next_round = build_material_exploration_next_round_plan(&report, 1);
    Ok(serde_json::json!({
        "schema_version": "kyuubiki.materialized-candidate-rerun/v1",
        "mode": "local_solver_materialized_rerun",
        "study": "material_composite_thermo_electric_panel",
        "step_count": steps.len(),
        "result_payloads": result_payloads,
        "report": report,
        "next_round": next_round
    }))
}

fn plan_next_round(path: &str) -> Result<Value, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let payload: Value =
        serde_json::from_str(&text).map_err(|error| format!("failed to parse {path}: {error}"))?;
    let exploration = payload.get("exploration").cloned().unwrap_or(payload);
    let plan = build_material_exploration_next_round_execution_plan(&exploration)?;
    serde_json::to_value(plan).map_err(|error| error.to_string())
}

fn read_materialization_plan(path: &str) -> Result<Value, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let payload: Value =
        serde_json::from_str(&text).map_err(|error| format!("failed to parse {path}: {error}"))?;
    Ok(payload
        .get("materialization_plan")
        .cloned()
        .or_else(|| payload.get("plan").cloned())
        .unwrap_or(payload))
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
    serde_json::to_value(run).map_err(|error| error.to_string())
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

fn run_solve_step(step: &HeadlessWorkflowStep) -> Result<Value, String> {
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

fn print_materialized_rerun_summary(payload: &Value) {
    println!(
        "Materialized rerun: {}",
        payload["study"].as_str().unwrap_or("unknown")
    );
    println!("Steps: {}", payload["step_count"].as_u64().unwrap_or(0));
    if let Some(winner) = payload["report"]["winner_candidate_id"].as_str() {
        println!("Winner: {winner}");
    }
    if let Some(decision) = payload["next_round"]["decision"].as_str() {
        println!("Next round: {decision}");
    }
}
