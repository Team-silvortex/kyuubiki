use kyuubiki_project_automation::{
    AutomationRunOptions, inspect_macro_file, list_automation_action_capabilities,
    normalize_macro_file, render_macro_file, risk_label, run_macro_file, validate_macro_file,
};
use serde_json::Value;
use std::ffi::OsString;
use std::fs;

type RunnerResult<T> = Result<T, String>;

const NATIVE_COMMANDS: &[&str] = &[
    "actions",
    "inspect",
    "validate",
    "normalize",
    "render",
    "run",
];

pub(crate) fn run_macro_command(args: Vec<OsString>) -> RunnerResult<u8> {
    let command = args
        .first()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    if !NATIVE_COMMANDS.contains(&command.as_str()) {
        return Err(format!("unknown macro command: {command}"));
    }

    let options = parse_options(&args[1..])?;
    match command.as_str() {
        "actions" => run_actions(options),
        "inspect" => run_inspect(options),
        "validate" => run_validate(options),
        "normalize" => run_normalize(options),
        "render" => run_render(options),
        "run" => run_macro(options),
        _ => unreachable!("native command allowlist and dispatch must match"),
    }
}

#[derive(Default)]
struct Options {
    positional: Vec<String>,
    output: Option<String>,
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
            "--out" => options.output = Some(take_option(args, &mut index, "--out")?),
            "--payload" => options.payload = Some(take_option(args, &mut index, "--payload")?),
            "--state" => options.state = Some(take_option(args, &mut index, "--state")?),
            "--api-base-url" => {
                options.api_base_url = Some(take_option(args, &mut index, "--api-base-url")?);
            }
            "--artifacts-dir" => {
                options.artifacts_dir = Some(take_option(args, &mut index, "--artifacts-dir")?);
            }
            value if value.starts_with("--out=") => {
                options.output = Some(value.trim_start_matches("--out=").to_string());
            }
            value if value.starts_with('-') => {
                return Err(format!("unsupported native macro option: {value}"));
            }
            value => options.positional.push(value.to_string()),
        }
        index += 1;
    }
    Ok(options)
}

fn take_option(args: &[OsString], index: &mut usize, name: &str) -> RunnerResult<String> {
    *index += 1;
    args.get(*index)
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} requires a value"))
}

fn run_actions(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 0, 0, "macro actions")?;
    let actions = list_automation_action_capabilities();
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "action_count": actions.len(),
                "actions": actions,
            }))
            .map_err(|error| error.to_string())?
        );
    } else {
        println!("Automation actions: {}", actions.len());
        for action in actions {
            println!("- {} [{}]", action.action, risk_label(action.risk));
            println!("  engine: {:?}", action.engine);
            println!("  runtime: {:?}", action.runtime_style);
            println!(
                "  required payload: {}",
                joined_or_dash(&action.required_payload_keys)
            );
            if let Some(route) = action.direct_fem_route {
                println!("  direct FEM route: {route}");
            }
        }
    }
    Ok(0)
}

fn run_inspect(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "macro inspect <input>")?;
    let summary = inspect_macro_file(&options.positional[0])?;
    if options.json {
        print_json(&summary)?;
    } else {
        println!("Macro: {}", summary.id);
        println!("Steps: {}", summary.step_count);
        println!("Actions: {}", joined_or_dash(&summary.actions));
    }
    Ok(0)
}

fn run_validate(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "macro validate <input>")?;
    let report = validate_macro_file(&options.positional[0])?;
    if options.json {
        print_json(&report)?;
    } else {
        println!(
            "Macro validation: {}",
            if report.ok { "ok" } else { "failed" }
        );
        println!("Macro: {}", report.summary.id);
        println!("Steps: {}", report.summary.step_count);
        for issue in &report.issues {
            println!("- {issue}");
        }
    }
    Ok(if report.ok { 0 } else { 1 })
}

fn run_normalize(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "macro normalize <input> --out <output>")?;
    let output = options
        .output
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "macro normalize requires --out <output>".to_string())?;
    let output = normalize_macro_file(&options.positional[0], output)?;
    println!("normalized macro -> {output}");
    Ok(0)
}

fn run_render(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "macro render <input>")?;
    let envelope = render_macro_file(
        &options.positional[0],
        read_optional_json(options.payload.as_deref())?,
        read_optional_json(options.state.as_deref())?,
    )?;
    if options.json {
        print_json(&envelope)?;
    } else {
        println!("Macro render: {}", envelope.plan.id);
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

fn run_macro(options: Options) -> RunnerResult<u8> {
    require_arity(&options, 1, 1, "macro run <input>")?;
    if options.execute && options.artifacts_dir.is_some() {
        return Err(
            "--artifacts-dir is browser-specific; native macro live execution is service-only"
                .to_string(),
        );
    }
    let report = run_macro_file(
        &options.positional[0],
        read_optional_json(options.payload.as_deref())?,
        read_optional_json(options.state.as_deref())?,
        &AutomationRunOptions {
            execute: options.execute,
            allow_sensitive: options.allow_sensitive,
            allow_destructive: options.allow_destructive,
            api_base_url: options.api_base_url,
            api_token: std::env::var("KYUUBIKI_API_TOKEN").ok(),
        },
    )?;
    if options.json {
        print_json(&report)?;
    } else {
        println!("Macro run: {}", report.metadata.macro_id);
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

fn read_optional_json(path: Option<&str>) -> RunnerResult<Value> {
    let Some(path) = path else {
        return Ok(serde_json::json!({}));
    };
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read macro input {path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid macro input {path}: {error}"))
}

fn print_json(value: &impl serde::Serialize) -> RunnerResult<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn joined_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "--".to_string()
    } else {
        values.join(", ")
    }
}

fn require_arity(options: &Options, min: usize, max: usize, usage: &str) -> RunnerResult<()> {
    if (min..=max).contains(&options.positional.len()) {
        Ok(())
    } else {
        Err(format!("usage: kyuubiki {usage}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_accepts_runtime_and_risk_options() {
        let args = [
            "input.json",
            "--payload",
            "payload.json",
            "--execute",
            "--allow-sensitive",
        ]
        .map(OsString::from);
        let options = parse_options(&args).expect("parse options");

        assert_eq!(options.positional, vec!["input.json"]);
        assert_eq!(options.payload.as_deref(), Some("payload.json"));
        assert!(options.execute);
        assert!(options.allow_sensitive);
    }

    #[test]
    fn parser_rejects_unknown_options() {
        let error = parse_options(&[OsString::from("--browser")]).err();
        assert!(error.is_some_and(|value| value.contains("unsupported native macro option")));
    }
}
