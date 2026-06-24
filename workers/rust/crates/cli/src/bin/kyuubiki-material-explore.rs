use std::fs;

use kyuubiki_headless_sdk::{
    HeadlessWorkflowStep, build_material_exploration_run, material_exploration_steps,
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
}

impl Flags {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "help") {
            return Err(usage());
        }
        let study = args[0].clone();
        let mut out = None;
        let mut json = false;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--json" => json = true,
                "--out" => out = Some(take_value(&args, &mut index, "--out")?),
                other => return Err(format!("unsupported flag: {other}\n\n{}", usage())),
            }
            index += 1;
        }
        Ok(Self { study, out, json })
    }
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
    "kyuubiki-material-explore <heat-spreader|dielectric-screening|thermo-shield|structural-panel> [--out exploration.json] [--json]\n\nRuns candidate material studies locally through real solver kernels and builds a ranked material report.".to_string()
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
        }
    }
}
