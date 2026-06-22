use std::fs;

use kyuubiki_headless_sdk::{
    MaterialOptimizationProfile, build_heat_spreader_screening_report,
    build_heat_spreader_screening_report_with_optimization,
    build_structural_panel_screening_report,
    build_structural_panel_screening_report_with_optimization,
    build_thermo_shield_screening_report, build_thermo_shield_screening_report_with_optimization,
};
use serde_json::Value;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let flags = Flags::parse(std::env::args().skip(1).collect::<Vec<_>>())?;
    let payload = read_json(&flags.input)?;
    let result_payloads = extract_result_payloads(&payload)?;
    let profile = flags
        .profile
        .as_deref()
        .map(read_optimization_profile)
        .transpose()?;
    let report = match flags.study.as_str() {
        "heat-spreader" | "material_heat_spreader_screening" => {
            serde_json::to_value(match profile {
                Some(profile) => build_heat_spreader_screening_report_with_optimization(
                    &result_payloads,
                    profile,
                ),
                None => build_heat_spreader_screening_report(&result_payloads),
            }?)
            .map_err(|error| error.to_string())?
        }
        "thermo-shield" | "material_thermo_shield_screening" => {
            serde_json::to_value(match profile {
                Some(profile) => build_thermo_shield_screening_report_with_optimization(
                    &result_payloads,
                    profile,
                ),
                None => build_thermo_shield_screening_report(&result_payloads),
            }?)
            .map_err(|error| error.to_string())?
        }
        "structural-panel" | "material_structural_panel_screening" => {
            serde_json::to_value(match profile {
                Some(profile) => build_structural_panel_screening_report_with_optimization(
                    &result_payloads,
                    profile,
                ),
                None => build_structural_panel_screening_report(&result_payloads),
            }?)
            .map_err(|error| error.to_string())?
        }
        other => return Err(format!("unsupported material report study: {other}")),
    };

    if let Some(out) = &flags.out {
        write_json(out, &report)?;
    }
    if flags.json {
        print_json(&report)?;
    } else {
        print_report(&report);
        if let Some(out) = &flags.out {
            println!("Report: {out}");
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Flags {
    study: String,
    input: String,
    profile: Option<String>,
    out: Option<String>,
    json: bool,
}

impl Flags {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "help") {
            return Err(usage());
        }
        let study = args[0].clone();
        let mut input = None;
        let mut profile = None;
        let mut out = None;
        let mut json = false;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--results" | "--input" => {
                    index += 1;
                    input = args.get(index).cloned();
                }
                "--out" => {
                    index += 1;
                    out = args.get(index).cloned();
                }
                "--profile" => {
                    index += 1;
                    profile = args.get(index).cloned();
                }
                "--json" => json = true,
                other => return Err(format!("unsupported flag: {other}\n\n{}", usage())),
            }
            index += 1;
        }
        Ok(Self {
            study,
            input: input.ok_or_else(usage)?,
            profile,
            out,
            json,
        })
    }
}

fn usage() -> String {
    "kyuubiki-material-report <heat-spreader|thermo-shield|structural-panel> --results results.json [--profile profile.json] [--out report.json] [--json]\n\nresults.json may be a raw result array, an object with results/result_payloads, or a kyuubiki.headless-execution-run/v1 report.".to_string()
}

fn read_json(path: &str) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("failed to parse {path}: {error}"))
}

fn read_optimization_profile(path: &str) -> Result<MaterialOptimizationProfile, String> {
    let payload = read_json(path)?;
    serde_json::from_value(payload)
        .map_err(|error| format!("failed to parse optimization profile {path}: {error}"))
}

fn write_json(path: &str, report: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| format!("failed to write {path}: {error}"))
}

fn print_json(report: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(report).map_err(|error| error.to_string())?;
    println!("{text}");
    Ok(())
}

fn extract_result_payloads(payload: &Value) -> Result<Vec<Value>, String> {
    if let Some(array) = payload.as_array() {
        return Ok(array.clone());
    }
    if payload.get("schema_version").and_then(Value::as_str)
        == Some("kyuubiki.headless-execution-run/v1")
    {
        return extract_result_payloads_from_headless_run(payload);
    }
    for key in ["results", "result_payloads"] {
        if let Some(array) = payload.get(key).and_then(Value::as_array) {
            return Ok(array.clone());
        }
    }
    Err("material report input must be an array, include a results array, or be a headless execution run report".to_string())
}

fn extract_result_payloads_from_headless_run(payload: &Value) -> Result<Vec<Value>, String> {
    let results = payload
        .get("steps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|step| {
            step.get("action").and_then(Value::as_str) == Some("result_fetch")
                && step.get("status").and_then(Value::as_str) != Some("blocked")
                && step.get("status").and_then(Value::as_str) != Some("failed")
        })
        .filter_map(|step| {
            let preview = step.get("result_preview")?;
            preview
                .get("result")
                .cloned()
                .or_else(|| Some(preview.clone()))
        })
        .collect::<Vec<_>>();
    if results.is_empty() {
        return Err(
            "headless execution run report does not contain successful result_fetch payloads"
                .to_string(),
        );
    }
    Ok(results)
}

fn print_report(report: &Value) {
    println!(
        "Material research report: {}",
        report.get("study").and_then(Value::as_str).unwrap_or("--")
    );
    println!(
        "Schema: {}",
        report
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
    println!(
        "Objective: {}",
        report
            .get("objective")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
    println!(
        "Winner: {}",
        report
            .get("winner_candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
    let candidates = report
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    println!("Candidates: {}", candidates.len());
    for candidate in candidates {
        println!(
            "{}. {} score={:.3} primary={} areal_mass={:.3}",
            candidate.get("rank").and_then(Value::as_u64).unwrap_or(0),
            candidate
                .get("candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("--"),
            candidate
                .get("score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            format_optional(primary_metric(&candidate)),
            candidate
                .get("areal_mass_kg_m2")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        );
    }
    let warnings = report
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !warnings.is_empty() {
        println!("Warnings: {}", warnings.len());
        for warning in warnings {
            println!(" - {warning}");
        }
    }
}

fn primary_metric(candidate: &Value) -> Option<f64> {
    candidate
        .get("peak_temperature_c")
        .or_else(|| candidate.get("max_stress_pa"))
        .and_then(Value::as_f64)
}

fn format_optional(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.3}"))
        .unwrap_or_else(|| "--".to_string())
}
