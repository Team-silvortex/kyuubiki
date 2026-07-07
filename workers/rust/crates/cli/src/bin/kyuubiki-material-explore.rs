use std::fs;

use kyuubiki_headless_sdk::{
    HeadlessWorkflowStep, build_material_exploration_next_round_execution_plan,
    build_material_exploration_run, material_exploration_steps,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Flags {
    study: String,
    out: Option<String>,
    json: bool,
    plan_next: Option<String>,
    run_next: Option<String>,
}

impl Flags {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "help") {
            return Err(usage());
        }
        let mut study = args[0].clone();
        let mut out = None;
        let mut json = false;
        let mut plan_next = None;
        let mut run_next = None;
        let mut index = 1;
        if study == "--plan-next" || study == "--run-next" {
            let option = study.clone();
            let Some(path) = args.get(1) else {
                return Err(format!("{option} requires a value"));
            };
            if path.starts_with("--") {
                return Err(format!("{option} requires a value"));
            }
            if option == "--plan-next" {
                plan_next = Some(path.clone());
            } else {
                run_next = Some(path.clone());
            }
            study = "from-previous-exploration".to_string();
            index = 2;
        }
        while index < args.len() {
            match args[index].as_str() {
                "--json" => json = true,
                "--out" => out = Some(take_value(&args, &mut index, "--out")?),
                "--plan-next" => plan_next = Some(take_value(&args, &mut index, "--plan-next")?),
                "--run-next" => run_next = Some(take_value(&args, &mut index, "--run-next")?),
                other => return Err(format!("unsupported flag: {other}\n\n{}", usage())),
            }
            index += 1;
        }
        Ok(Self {
            study,
            out,
            json,
            plan_next,
            run_next,
        })
    }
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

fn run_next_round(path: &str) -> Result<Value, String> {
    let previous = read_exploration_input(path)?;
    let plan = build_material_exploration_next_round_execution_plan(&previous)?;
    let study = plan.study.clone();
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

    let run = build_material_exploration_run(&study, "local_solver_next_round", result_payloads)?;
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

fn to_value<T: Serialize>(result: Result<T, String>) -> Result<Value, String> {
    serde_json::to_value(result?).map_err(|error| error.to_string())
}

fn print_summary(exploration: &Value) {
    let report = &exploration["report"];
    println!(
        "Material exploration: {}",
        exploration["study"].as_str().unwrap_or("--")
    );
    println!("Mode: {}", exploration["mode"].as_str().unwrap_or("--"));
    println!(
        "Candidates: {}",
        exploration["candidate_count"].as_u64().unwrap_or(0)
    );
    println!(
        "Winner: {}",
        report
            .get("winner_candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
    if let Some(candidates) = report.get("candidates").and_then(Value::as_array) {
        for candidate in candidates {
            println!(
                "#{} {} score={:.6}",
                candidate.get("rank").and_then(Value::as_u64).unwrap_or(0),
                candidate
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or("--"),
                candidate
                    .get("score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            );
        }
    }
    if let Some(next_round) = exploration.get("next_round") {
        println!(
            "Next round: {}",
            next_round
                .get("decision")
                .and_then(Value::as_str)
                .unwrap_or("--")
        );
        if let Some(actions) = next_round.get("actions").and_then(Value::as_array) {
            let action_list = actions
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            if !action_list.is_empty() {
                println!("Actions: {action_list}");
            }
        }
    }
}

fn print_next_round_plan_summary(plan: &Value) {
    println!(
        "Material next round: {}",
        plan.get("study").and_then(Value::as_str).unwrap_or("--")
    );
    println!(
        "Decision: {}",
        plan.get("decision").and_then(Value::as_str).unwrap_or("--")
    );
    println!(
        "Runnable steps: {}",
        plan.get("runnable_step_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    if let Some(focus) = plan.get("focus_candidate_ids").and_then(Value::as_array) {
        let focus_list = focus
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !focus_list.is_empty() {
            println!("Focus: {focus_list}");
        }
    }
}

fn take_value(args: &[String], index: &mut usize, option: &str) -> Result<String, String> {
    *index += 1;
    let Some(value) = args.get(*index) else {
        return Err(format!("{option} requires a value"));
    };
    if value.starts_with("--") {
        return Err(format!("{option} requires a value"));
    }
    Ok(value.clone())
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

fn usage() -> String {
    "kyuubiki-material-explore <heat-spreader|dielectric-screening|thermo-shield|structural-panel> [--out exploration.json] [--json]\nkyuubiki-material-explore --plan-next previous-exploration.json [--out next-round.json] [--json]\nkyuubiki-material-explore --run-next previous-exploration.json [--out next-exploration.json] [--json]\n\nRuns candidate material studies locally through real solver kernels and builds a ranked material report.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explores_heat_spreader_with_real_solver_results() {
        let exploration = run_material_exploration("heat-spreader").expect("exploration");
        assert_eq!(
            exploration["schema_version"].as_str(),
            Some("kyuubiki.material-exploration-run/v1")
        );
        assert_eq!(exploration["candidate_count"].as_u64(), Some(3));
        assert!(exploration["report"]["winner_candidate_id"].is_string());
        assert!(
            matches!(
                exploration["next_round"]["decision"].as_str(),
                Some("expand_around_winner" | "repair_or_rerun")
            ),
            "real solver runs should produce an actionable next-round decision"
        );
        assert_eq!(
            exploration["result_payloads"].as_array().map(Vec::len),
            Some(3)
        );
    }

    #[test]
    fn explores_all_material_studies_with_real_solver_results() {
        for study in [
            "heat-spreader",
            "dielectric-screening",
            "thermo-shield",
            "structural-panel",
        ] {
            let exploration = run_material_exploration(study).expect("exploration");
            assert_eq!(exploration["candidate_count"].as_u64(), Some(3));
            assert!(exploration["report"]["winner_candidate_id"].is_string());
            assert!(exploration["next_round"]["actions"].is_array());
        }
    }

    #[test]
    fn plans_next_round_from_previous_exploration_json() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "kyuubiki-material-exploration-{}.json",
            std::process::id()
        ));
        let exploration = run_material_exploration("dielectric-screening").expect("exploration");
        fs::write(&path, serde_json::to_vec(&exploration).expect("json")).expect("write");

        let plan = plan_next_round(path.to_str().expect("utf8 path")).expect("plan");

        assert_eq!(
            plan["schema_version"].as_str(),
            Some("kyuubiki.material-exploration-next-round-execution/v1")
        );
        assert!(plan["runnable_step_count"].as_u64().unwrap_or(0) > 0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn plans_next_round_from_evidence_wrapper_json() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "kyuubiki-material-exploration-wrapper-{}.json",
            std::process::id()
        ));
        let exploration = run_material_exploration("structural-panel").expect("exploration");
        let wrapper = serde_json::json!({
            "schema_version": "kyuubiki.automated-material-research-example/v1",
            "exploration": exploration
        });
        fs::write(&path, serde_json::to_vec(&wrapper).expect("json")).expect("write");

        let plan = plan_next_round(path.to_str().expect("utf8 path")).expect("plan");

        assert_eq!(
            plan["schema_version"].as_str(),
            Some("kyuubiki.material-exploration-next-round-execution/v1")
        );
        assert!(plan["runnable_step_count"].as_u64().unwrap_or(0) > 0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn runs_next_round_from_previous_exploration_json() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "kyuubiki-material-exploration-run-next-{}.json",
            std::process::id()
        ));
        let exploration = run_material_exploration("dielectric-screening").expect("exploration");
        fs::write(&path, serde_json::to_vec(&exploration).expect("json")).expect("write");

        let next = run_next_round(path.to_str().expect("utf8 path")).expect("next run");

        assert_eq!(
            next["schema_version"].as_str(),
            Some("kyuubiki.material-exploration-run/v1")
        );
        assert_eq!(next["mode"].as_str(), Some("local_solver_next_round"));
        assert_eq!(next["candidate_count"].as_u64(), Some(3));
        assert!(next["report"]["winner_candidate_id"].is_string());
        let _ = fs::remove_file(path);
    }
}
