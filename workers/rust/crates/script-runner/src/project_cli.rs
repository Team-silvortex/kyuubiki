use kyuubiki_project_automation::{
    AutomationRunOptions, list_project_automation_presets, render_project_automation_preset,
    risk_label, run_project_automation_preset,
};
use kyuubiki_project_bundle::{
    create_project_bundle, diff_project_bundles, inspect_project_bundle, normalize_project_bundle,
    pack_project_bundle, unpack_project_bundle, validate_project_bundle, validation_passed,
};
use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

type RunnerResult<T> = Result<T, String>;

const NATIVE_COMMANDS: &[&str] = &[
    "create",
    "inspect",
    "validate",
    "normalize",
    "unpack",
    "pack",
    "diff",
    "automation-presets",
    "automation-render",
    "automation-run",
];

pub(crate) fn run_project_command(args: Vec<OsString>) -> RunnerResult<u8> {
    let command = args
        .first()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    if !NATIVE_COMMANDS.contains(&command.as_str()) {
        return Err(format!("unknown project command: {command}"));
    }

    let options = parse_options(&args[1..])?;
    match command.as_str() {
        "create" => run_create(options),
        "inspect" => run_inspect(options),
        "validate" => run_validate(options),
        "normalize" => run_normalize(options),
        "unpack" => run_unpack(options),
        "pack" => run_pack(options),
        "diff" => run_diff(options),
        "automation-presets" => run_automation_presets(options),
        "automation-render" => run_automation_render(options),
        "automation-run" => run_automation(options),
        _ => unreachable!("native command allowlist and dispatch must match"),
    }
}

#[derive(Default)]
struct Options {
    positional: Vec<String>,
    output: Option<String>,
    preset: Option<String>,
    payload: Option<String>,
    state: Option<String>,
    api_base_url: Option<String>,
    artifacts_dir: Option<String>,
    json: bool,
    execute: bool,
    allow_sensitive: bool,
    allow_destructive: bool,
}

fn parse_options(args: &[OsString]) -> RunnerResult<Options> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let value = args[index].to_string_lossy();
        match value.as_ref() {
            "--json" => options.json = true,
            "--execute" => options.execute = true,
            "--allow-sensitive" => options.allow_sensitive = true,
            "--allow-destructive" => options.allow_destructive = true,
            "--out" => {
                index += 1;
                options.output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path".to_string())?
                        .to_string_lossy()
                        .to_string(),
                );
            }
            value if value.starts_with("--out=") => {
                options.output = Some(value.trim_start_matches("--out=").to_string());
            }
            "--preset" => options.preset = Some(take_option(args, &mut index, "--preset")?),
            "--payload" => options.payload = Some(take_option(args, &mut index, "--payload")?),
            "--state" => options.state = Some(take_option(args, &mut index, "--state")?),
            "--api-base-url" => {
                options.api_base_url = Some(take_option(args, &mut index, "--api-base-url")?);
            }
            "--artifacts-dir" => {
                options.artifacts_dir = Some(take_option(args, &mut index, "--artifacts-dir")?);
            }
            value if value.starts_with('-') => {
                return Err(format!("unsupported native project option: {value}"));
            }
            value => options.positional.push(value.to_string()),
        }
        index += 1;
    }
    Ok(options)
}

fn take_option(args: &[OsString], index: &mut usize, option: &str) -> RunnerResult<String> {
    *index += 1;
    args.get(*index)
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| format!("{option} requires a value"))
}

fn run_create(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 0, 1, "project create [bundle]")?;
    let path = options
        .positional
        .first()
        .map(|value| absolute_create_path(value))
        .transpose()?
        .unwrap_or_default();
    let rendered = create_project_bundle(&path)?;
    if options.json {
        println!("{rendered}");
    } else {
        let report = parse_json(&rendered)?;
        println!(
            "Created project: {}",
            report["path"].as_str().unwrap_or("unknown path")
        );
    }
    Ok(0)
}

fn run_inspect(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "project inspect <input>")?;
    let rendered = inspect_project_bundle(&options.positional[0])?;
    if options.json {
        println!("{rendered}");
    } else {
        print_inspection(&parse_json(&rendered)?);
    }
    Ok(0)
}

fn run_validate(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "project validate <input>")?;
    let rendered = validate_project_bundle(&options.positional[0])?;
    let passed = validation_passed(&rendered)?;
    if options.json {
        println!("{rendered}");
    } else {
        let report = parse_json(&rendered)?;
        println!(
            "Project validation: {}",
            if passed { "ok" } else { "failed" }
        );
        println!("Issues: {}", report["issue_count"].as_u64().unwrap_or(0));
        if let Some(issues) = report["issues"].as_array() {
            for issue in issues.iter().filter_map(Value::as_str) {
                println!("- {issue}");
            }
        }
    }
    Ok(if passed { 0 } else { 1 })
}

fn run_normalize(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "project normalize <input> --out <output>")?;
    println!(
        "{}",
        normalize_project_bundle(&options.positional[0], required_output(&options)?)?
    );
    Ok(0)
}

fn run_unpack(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "project unpack <bundle> --out <directory>")?;
    println!(
        "{}",
        unpack_project_bundle(&options.positional[0], required_output(&options)?)?
    );
    Ok(0)
}

fn run_pack(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "project pack <directory> --out <bundle>")?;
    println!(
        "{}",
        pack_project_bundle(&options.positional[0], required_output(&options)?)?
    );
    Ok(0)
}

fn run_diff(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 2, 2, "project diff <left> <right>")?;
    let rendered = diff_project_bundles(&options.positional[0], &options.positional[1])?;
    if options.json {
        println!("{rendered}");
    } else {
        let report = parse_json(&rendered)?;
        println!(
            "Project identity changed: {}",
            report["changed_project_identity"]
        );
        println!("Active model changed: {}", report["active_model_changed"]);
        println!(
            "Active version changed: {}",
            report["active_version_changed"]
        );
    }
    Ok(0)
}

fn run_automation_presets(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "project automation-presets <input>")?;
    let presets = list_project_automation_presets(&options.positional[0])?;
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "preset_count": presets.len(),
                "presets": presets,
            }))
            .map_err(|error| error.to_string())?
        );
    } else {
        println!("Automation presets: {}", presets.len());
        for preset in presets {
            println!("- {} ({})", preset.name, preset.preset_id);
            println!("  steps: {}", preset.step_count);
            println!(
                "  actions: {}",
                if preset.actions.is_empty() {
                    "--".to_string()
                } else {
                    preset.actions.join(", ")
                }
            );
        }
    }
    Ok(0)
}

fn run_automation_render(options: Options) -> RunnerResult<u8> {
    require_arity(
        &options,
        1,
        1,
        "project automation-render <input> --preset <id|name>",
    )?;
    let envelope = render_project_automation_preset(
        &options.positional[0],
        required_preset(&options)?,
        read_optional_json(options.payload.as_deref())?,
        read_optional_json(options.state.as_deref())?,
    )?;
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).map_err(|error| error.to_string())?
        );
    } else {
        println!(
            "Automation preset: {} ({})",
            envelope.source.preset_name.as_deref().unwrap_or("--"),
            envelope.source.preset_id.as_deref().unwrap_or("--")
        );
        println!(
            "Project: {}",
            envelope.source.project_id.as_deref().unwrap_or("--")
        );
        println!("Steps: {}", envelope.plan.step_count);
        println!(
            "Highest risk: {}",
            risk_label(envelope.risk_summary.highest_risk)
        );
        for (index, step) in envelope.plan.steps.iter().enumerate() {
            println!("{}. {} [{}]", index + 1, step.action, risk_label(step.risk));
            println!("   payload: {}", step.payload);
        }
    }
    Ok(0)
}

fn run_automation(options: Options) -> RunnerResult<u8> {
    require_arity(
        &options,
        1,
        1,
        "project automation-run <input> --preset <id|name>",
    )?;
    if options.execute && options.artifacts_dir.is_some() {
        return Err(
            "--artifacts-dir is browser-specific; native project automation live execution is service-only"
                .to_string(),
        );
    }
    let report = run_project_automation_preset(
        &options.positional[0],
        required_preset(&options)?,
        read_optional_json(options.payload.as_deref())?,
        read_optional_json(options.state.as_deref())?,
        &AutomationRunOptions {
            execute: options.execute,
            allow_sensitive: options.allow_sensitive,
            allow_destructive: options.allow_destructive,
            api_base_url: options.api_base_url.clone(),
            api_token: std::env::var("KYUUBIKI_API_TOKEN").ok(),
        },
    )?;
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
    } else {
        println!("Automation run: {}", report.metadata.macro_id);
        println!(
            "Mode: {}",
            if report.dry_run {
                "dry-run"
            } else {
                "live-execute"
            }
        );
        println!("Status: {}", report.status);
        println!(
            "Executed steps: {}/{}",
            report.executed_step_count, report.metadata.step_count
        );
        if let Some(step) = &report.blocked_by_confirmation {
            println!(
                "Blocked: step {} requires {} confirmation",
                step.index + 1,
                risk_label(step.risk)
            );
        }
        if let Some(step) = &report.failed_step {
            println!("Failed: step {} {}", step.index + 1, step.action);
            println!("Reason: {}", step.message);
        }
        for (index, step) in report.steps.iter().enumerate() {
            println!("{}. {} -> {}", index + 1, step.action, step.status);
            println!("   payload: {}", step.payload);
        }
    }
    Ok(if report.status == "failed" { 1 } else { 0 })
}

fn required_preset(options: &Options) -> RunnerResult<&str> {
    options
        .preset
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "project automation command requires --preset <id|name>".to_string())
}

fn read_optional_json(path: Option<&str>) -> RunnerResult<Value> {
    let Some(path) = path else {
        return Ok(serde_json::json!({}));
    };
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read automation input {path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid automation input {path}: {error}"))
}

fn required_output(options: &Options) -> RunnerResult<&str> {
    options
        .output
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "project command requires --out <output>".to_string())
}

fn require_arity(options: &Options, min: usize, max: usize, usage: &str) -> RunnerResult<()> {
    if (min..=max).contains(&options.positional.len()) {
        Ok(())
    } else {
        Err(format!("usage: kyuubiki {usage}"))
    }
}

fn absolute_create_path(value: &str) -> RunnerResult<String> {
    let path = PathBuf::from(value);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|error| format!("failed to resolve current directory: {error}"))?
            .join(path)
    };
    Ok(absolute.to_string_lossy().to_string())
}

fn parse_json(rendered: &str) -> RunnerResult<Value> {
    serde_json::from_str(rendered)
        .map_err(|error| format!("invalid native project output: {error}"))
}

fn print_inspection(summary: &Value) {
    println!(
        "Project: {} ({})",
        summary["project_name"].as_str().unwrap_or("--"),
        summary["project_id"].as_str().unwrap_or("--")
    );
    println!("Schema: {}", summary["schema"].as_str().unwrap_or("--"));
    println!("Layout: {}", summary["layout"].as_str().unwrap_or("--"));
    println!("Models: {}", summary["model_count"].as_u64().unwrap_or(0));
    println!(
        "Versions: {}",
        summary["version_count"].as_u64().unwrap_or(0)
    );
    println!("Jobs: {}", summary["job_count"].as_u64().unwrap_or(0));
    println!("Results: {}", summary["result_count"].as_u64().unwrap_or(0));
}
