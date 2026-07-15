use std::fs;

use kyuubiki_headless_sdk::{
    MaterialOptimizationProfile, build_material_report_with_optimization, describe_material_study,
    extract_material_result_payloads, material_study_catalog,
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
    match flags.command {
        CommandMode::List => return print_study_list(flags.json),
        CommandMode::Describe => return print_study_description(&flags.study, flags.json),
        CommandMode::BuildReport => {}
    }
    let payload = read_json(&flags.input)?;
    let result_payloads = extract_material_result_payloads(&payload)?;
    let profile = flags
        .profile
        .as_deref()
        .map(read_optimization_profile)
        .transpose()?;
    let report = build_material_report_with_optimization(&flags.study, &result_payloads, profile)?;

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
    command: CommandMode,
    study: String,
    input: String,
    profile: Option<String>,
    out: Option<String>,
    json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandMode {
    BuildReport,
    List,
    Describe,
}

impl Flags {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "help") {
            return Err(usage());
        }
        let mut command = CommandMode::BuildReport;
        let mut study = args[0].clone();
        let mut index = 1;
        if args[0] == "list" || args[0] == "studies" {
            command = CommandMode::List;
            study = String::new();
        } else if args[0] == "describe" {
            command = CommandMode::Describe;
            study = args
                .get(1)
                .cloned()
                .ok_or_else(|| "describe requires a material study id or alias".to_string())?;
            index = 2;
        }
        let mut input = None;
        let mut profile = None;
        let mut out = None;
        let mut json = false;
        while index < args.len() {
            match args[index].as_str() {
                "--json" => json = true,
                "--results" | "--input" => {
                    require_build_command(&command, "--results")?;
                    input = Some(take_value(&args, &mut index, "--results")?);
                }
                "--out" => {
                    require_build_command(&command, "--out")?;
                    out = Some(take_value(&args, &mut index, "--out")?);
                }
                "--profile" => {
                    require_build_command(&command, "--profile")?;
                    profile = Some(take_value(&args, &mut index, "--profile")?);
                }
                other => return Err(format!("unsupported flag: {other}\n\n{}", usage())),
            }
            index += 1;
        }
        if command == CommandMode::BuildReport && input.is_none() {
            return Err(usage());
        }
        Ok(Self {
            command,
            study,
            input: input.unwrap_or_default(),
            profile,
            out,
            json,
        })
    }
}

fn require_build_command(command: &CommandMode, option: &str) -> Result<(), String> {
    if *command == CommandMode::BuildReport {
        return Ok(());
    }
    Err(format!(
        "{option} is only valid when building a material report"
    ))
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

fn usage() -> String {
    "kyuubiki-material-report list [--json]\nkyuubiki-material-report describe <study> [--json]\nkyuubiki-material-report <heat-spreader|dielectric-screening|thermo-shield|structural-panel|composite-thermo-electric-panel> --results results.json [--profile profile.json] [--out report.json] [--json]\n\nresults.json may be a raw result array, an object with results/result_payloads, or a kyuubiki.headless-execution-run/v1 report.".to_string()
}

fn print_study_list(json: bool) -> Result<(), String> {
    let studies = material_study_catalog();
    if json {
        return print_json(&serde_json::json!({
            "schema_version": "kyuubiki.material-study-catalog/v1",
            "study_count": studies.len(),
            "studies": studies,
        }));
    }
    println!("Material studies: {}", studies.len());
    for study in studies {
        println!("- {} [{}]", study.id, study.domain);
        println!("  {}", study.objective);
        println!("  template: {}", study.template_id);
        println!("  metrics: {}", study.metric_specs.len());
        println!(
            "  material cards: {} refs via {}",
            study.material_card_ref_count, study.material_card_schema_version
        );
    }
    Ok(())
}

fn print_study_description(study: &str, json: bool) -> Result<(), String> {
    let description = describe_material_study(study)
        .ok_or_else(|| format!("unsupported material report study: {study}"))?;
    if json {
        return print_json(&description);
    }
    println!("Material study: {}", description.id);
    println!("Title: {}", description.title);
    println!("Domain: {}", description.domain);
    println!("Objective: {}", description.objective);
    println!("Schema: {}", description.schema_version);
    println!("Template: {}", description.template_id);
    println!("Aliases: {}", description.aliases.join(", "));
    println!(
        "Material cards: {} refs via {}{}",
        description.material_card_ref_count,
        description.material_card_schema_version,
        if description.material_card_contract_required {
            " (required)"
        } else {
            ""
        }
    );
    println!("Metrics: {}", description.metric_specs.len());
    for metric in description.metric_specs {
        println!(
            "- {} [{}] {} weight={}",
            metric.id, metric.unit, metric.objective, metric.weight
        );
    }
    Ok(())
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

fn print_json<T: Serialize>(report: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(report).map_err(|error| error.to_string())?;
    println!("{text}");
    Ok(())
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
    print_reliability_summary(report);
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

fn print_reliability_summary(report: &Value) {
    let Some(summary) = report
        .get("reliability")
        .and_then(|reliability| reliability.get("summary"))
    else {
        return;
    };
    println!(
        "Reliability: {}",
        summary
            .get("decision")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
    println!(
        "Quality gates: pass={} violate={} unknown={} observe={}",
        summary
            .get("pass_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        summary
            .get("violation_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        summary
            .get("unknown_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        summary
            .get("observe_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    let blocking_gate_ids = summary
        .get("blocking_gate_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if !blocking_gate_ids.is_empty() {
        println!("Blocking gates: {}", blocking_gate_ids.join(", "));
    }
}

fn primary_metric(candidate: &Value) -> Option<f64> {
    candidate
        .get("peak_temperature_c")
        .or_else(|| candidate.get("max_electric_field_v_m"))
        .or_else(|| candidate.get("max_stress_pa"))
        .and_then(Value::as_f64)
}

fn format_optional(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.3}"))
        .unwrap_or_else(|| "--".to_string())
}
