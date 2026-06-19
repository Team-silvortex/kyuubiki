use std::env;
use std::fs;
use std::path::PathBuf;

use kyuubiki_headless_sdk::{
    HeadlessEngine, HeadlessExecutionBatch, HeadlessRunReport, HeadlessRuntimeStyle,
    HeadlessTemplateDescriptor, HeadlessValidationReport, HeadlessWorkflowDocument,
    HybridHeadlessExecutor, MockHeadlessExecutor, ServiceHeadlessExecutor, build_template_document,
    collect_executor_compatibility_issues, execute_batch_with_executor,
    normalize_workflow_document, run_batch_dry, search_templates, suggest_template_details,
    suggest_templates, summarize_batch, validate_batch,
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
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        "templates" => handle_templates(&args[1..]),
        "suggest" => handle_suggest(&args[1..]),
        "init" => handle_init(&args[1..]),
        "inspect" => handle_inspect(&args[1..]),
        "validate" => handle_validate(&args[1..]),
        "run" => handle_run(&args[1..]),
        other => Err(format!("unknown command: {other}")),
    }
}

fn print_usage() {
    println!(
        "kyuubiki headless (Rust)\n\nUsage:\n  kyuubiki-headless help\n  kyuubiki-headless templates [--runtime service_only|browser_only|hybrid] [--category name] [--tag label] [--query text] [--json]\n  kyuubiki-headless suggest <query> [--json]\n  kyuubiki-headless init [--template <id>] [--runtime-style service_only|browser_only|hybrid] [--category name] [--tag label] [--query text] [--workflow-id workflow.id] [--out output.json] [--json]\n  kyuubiki-headless inspect <input> [--json]\n  kyuubiki-headless validate <input> [--json]\n  kyuubiki-headless run <input> [--json] [--report-out report.json] [--allow-sensitive] [--allow-destructive] [--execute] [--executor mock|service|hybrid] [--api-base-url http://127.0.0.1:3000] [--api-token token]"
    );
}

fn handle_templates(args: &[String]) -> Result<(), String> {
    let flags = Flags::parse(args);
    let templates = filter_templates(&flags);
    if flags.json {
        print_json(&TemplateListOutput {
            template_count: templates.len(),
            templates: templates
                .iter()
                .map(|template| TemplateView::from_descriptor(template))
                .collect(),
        })?;
        return Ok(());
    }
    println!("Headless templates: {}", templates.len());
    print_template_groups(&templates);
    Ok(())
}

fn handle_suggest(args: &[String]) -> Result<(), String> {
    let flags = Flags::parse(args);
    let query = flags
        .query
        .clone()
        .or_else(|| flags.positional.first().cloned())
        .ok_or_else(|| "suggest requires a query".to_string())?;
    let suggestions = suggest_template_details(&query, 5);
    if flags.json {
        print_json(&suggestions)?;
        return Ok(());
    }
    if suggestions.is_empty() {
        println!("No template suggestions for: {query}");
        return Ok(());
    }
    println!("Template suggestions for: {query}");
    for suggestion in suggestions {
        println!(
            "- {} [{}] score={} terms={}",
            suggestion.id,
            runtime_style_label(suggestion.runtime_style),
            suggestion.score,
            suggestion.matched_terms.join(", ")
        );
    }
    Ok(())
}

fn print_template_groups(templates: &[&HeadlessTemplateDescriptor]) {
    let mut categories = templates
        .iter()
        .map(|template| template.category)
        .collect::<Vec<_>>();
    categories.sort_unstable();
    categories.dedup();
    for category in categories {
        println!("\n[{}]", category);
        for template in templates
            .iter()
            .filter(|template| template.category == category)
        {
            let view = TemplateView::from_descriptor(template);
            println!("- {} ({} steps)", view.id, view.step_count);
            println!("  {}", view.title);
            println!("  {}", view.description);
            println!("  runtime: {}", runtime_style_label(view.runtime_style));
            println!("  tags: {}", view.tags.join(", "));
            println!("  actions: {}", view.actions.join(", "));
        }
    }
}

fn handle_init(args: &[String]) -> Result<(), String> {
    let flags = Flags::parse(args);
    let template = resolve_template(&flags)?;
    let document = build_template_document(template.id, flags.workflow_id.as_deref())
        .ok_or_else(|| format!("failed to build template {}", template.id))?;
    if flags.json && flags.out.is_none() {
        print_json(&document)?;
        return Ok(());
    }
    let output_path = flags
        .out
        .clone()
        .unwrap_or_else(|| format!("{}.headless-workflow.json", template.id));
    let resolved_output_path = PathBuf::from(&output_path);
    let output_bytes = serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?;
    fs::write(&output_path, output_bytes)
        .map_err(|error| format!("failed to write {}: {error}", output_path))?;
    println!(
        "initialized headless workflow -> {}",
        resolved_output_path
            .canonicalize()
            .unwrap_or(resolved_output_path)
            .display()
    );
    Ok(())
}

fn handle_inspect(args: &[String]) -> Result<(), String> {
    let flags = Flags::parse(args);
    let input_path = flags.input_path()?;
    let batch = load_batch_from_path(&input_path)?;
    let summary = summarize_batch(&batch);
    if flags.json {
        print_json(&summary)?;
        return Ok(());
    }
    println!("Headless workflow: {}", summary.workflow_id);
    println!("Schema: {}", summary.schema_version);
    println!("Language: {}", summary.language);
    println!("Steps: {}", summary.step_count);
    println!("Warnings: {}", summary.warning_count);
    println!("Actions: {}", summary.actions.join(", "));
    Ok(())
}

fn handle_validate(args: &[String]) -> Result<(), String> {
    let flags = Flags::parse(args);
    let input_path = flags.input_path()?;
    let batch = load_batch_from_path(&input_path)?;
    let report = validate_batch(&batch);
    if flags.json {
        print_json(&report)?;
        return if report.ok {
            Ok(())
        } else {
            Err("validation failed".to_string())
        };
    }
    print_validation_report(&report);
    if report.ok {
        Ok(())
    } else {
        Err("validation failed".to_string())
    }
}

fn handle_run(args: &[String]) -> Result<(), String> {
    let flags = Flags::parse(args);
    let input_path = flags.input_path()?;
    let batch = load_batch_from_path(&input_path)?;
    let report = if flags.execute {
        let executor_name = flags.executor.as_deref().unwrap_or("mock");
        let compatibility_issues = collect_executor_compatibility_issues(&batch, executor_name);
        if !compatibility_issues.is_empty() {
            return Err(format!(
                "executor compatibility check failed:\n{}",
                compatibility_issues.join("\n")
            ));
        }
        match executor_name {
            "mock" => {
                let mut executor = MockHeadlessExecutor;
                execute_batch_with_executor(
                    &batch,
                    &mut executor,
                    flags.allow_sensitive,
                    flags.allow_destructive,
                )
            }
            "service" => {
                let mut executor = ServiceHeadlessExecutor::new(
                    flags
                        .api_base_url
                        .as_deref()
                        .unwrap_or("http://127.0.0.1:3000"),
                );
                if let Some(token) = flags.api_token.as_deref() {
                    executor = ServiceHeadlessExecutor::with_token(
                        flags
                            .api_base_url
                            .as_deref()
                            .unwrap_or("http://127.0.0.1:3000"),
                        Some(token),
                    );
                }
                execute_batch_with_executor(
                    &batch,
                    &mut executor,
                    flags.allow_sensitive,
                    flags.allow_destructive,
                )
            }
            "hybrid" => {
                let mut executor = HybridHeadlessExecutor::with_token(
                    flags
                        .api_base_url
                        .as_deref()
                        .unwrap_or("http://127.0.0.1:3000"),
                    flags.api_token.as_deref(),
                );
                execute_batch_with_executor(
                    &batch,
                    &mut executor,
                    flags.allow_sensitive,
                    flags.allow_destructive,
                )
            }
            other => {
                return Err(format!(
                    "unsupported executor \"{}\"; currently available: mock, service, hybrid",
                    other
                ));
            }
        }
    } else {
        run_batch_dry(&batch, flags.allow_sensitive, flags.allow_destructive)
    };
    if let Some(report_out) = &flags.report_out {
        let output_bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
        fs::write(report_out, output_bytes)
            .map_err(|error| format!("failed to write {}: {error}", report_out))?;
    }
    if flags.json {
        print_json(&report)?;
        return if report.validation.ok {
            Ok(())
        } else {
            Err("run report generated from invalid batch".to_string())
        };
    }
    print_run_report(&report);
    if report.validation.ok {
        Ok(())
    } else {
        Err("run report generated from invalid batch".to_string())
    }
}

fn load_batch_from_path(path: &str) -> Result<HeadlessExecutionBatch, String> {
    let payload =
        fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let value = serde_json::from_str::<Value>(&payload)
        .map_err(|error| format!("failed to parse {path}: {error}"))?;
    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if schema_version == "kyuubiki.headless-execution-batch/v1" {
        return serde_json::from_value(value).map_err(|error| error.to_string());
    }
    if schema_version == "kyuubiki.headless-workflow/v1" {
        let document = serde_json::from_value::<HeadlessWorkflowDocument>(value)
            .map_err(|error| error.to_string())?;
        return normalize_workflow_document(&document);
    }
    Err(format!(
        "unsupported headless document schema: {schema_version}"
    ))
}

fn resolve_template(flags: &Flags) -> Result<&'static HeadlessTemplateDescriptor, String> {
    let matches = filter_templates(flags);
    if let Some(template_id) = flags.template.as_deref() {
        return matches
            .into_iter()
            .find(|template| template.id == template_id)
            .ok_or_else(|| unknown_template_message(template_id));
    }
    resolve_filtered_template(matches)
}

fn resolve_filtered_template(
    matches: Vec<&'static HeadlessTemplateDescriptor>,
) -> Result<&'static HeadlessTemplateDescriptor, String> {
    if matches.len() == 1 {
        return Ok(matches[0]);
    }
    if matches.is_empty() {
        return Err("no headless templates found for the current filters".to_string());
    }
    Err(format!(
        "template filters match multiple templates. Choose one with --template. Available templates: {}",
        matches
            .iter()
            .map(|template| template.id)
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn filter_templates(flags: &Flags) -> Vec<&'static HeadlessTemplateDescriptor> {
    search_templates(
        flags.runtime.as_deref().and_then(parse_runtime_style),
        flags.category.as_deref(),
        flags.tag.as_deref(),
        flags.query.as_deref(),
    )
}

fn unknown_template_message(template_id: &str) -> String {
    let suggestions = suggest_templates(template_id, 5);
    if suggestions.is_empty() {
        return format!("unknown headless template \"{template_id}\"");
    }
    format!(
        "unknown headless template \"{template_id}\". Closest matches: {}",
        suggestions
            .iter()
            .map(|template| template.id)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn template_actions(template_id: &str) -> Vec<String> {
    build_template_document(template_id, None)
        .map(|document| {
            document
                .workflow
                .steps
                .into_iter()
                .map(|step| step.action)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_runtime_style(value: &str) -> Option<HeadlessRuntimeStyle> {
    match value.trim().to_lowercase().as_str() {
        "service_only" => Some(HeadlessRuntimeStyle::ServiceOnly),
        "browser_only" => Some(HeadlessRuntimeStyle::BrowserOnly),
        "hybrid" => Some(HeadlessRuntimeStyle::Hybrid),
        _ => None,
    }
}

fn runtime_style_label(runtime: HeadlessRuntimeStyle) -> &'static str {
    match runtime {
        HeadlessRuntimeStyle::ServiceOnly => "service_only",
        HeadlessRuntimeStyle::BrowserOnly => "browser_only",
        HeadlessRuntimeStyle::Hybrid => "hybrid",
        HeadlessRuntimeStyle::Unknown => "unknown",
    }
}

fn print_validation_report(report: &HeadlessValidationReport) {
    println!(
        "Headless validation: {}",
        if report.ok { "ok" } else { "failed" }
    );
    if let Some(summary) = &report.summary {
        println!("Workflow: {}", summary.workflow_id);
        println!("Schema: {}", summary.schema_version);
        println!("Steps: {}", summary.step_count);
    }
    if let Some(policy) = &report.policy {
        println!(
            "Runtime: {}",
            runtime_style_label(policy.recommended_runtime)
        );
        println!(
            "Engines: {}",
            policy
                .required_engines
                .iter()
                .map(engine_label)
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "Risks: normal {}, sensitive {}, destructive {}",
            policy
                .risk_counts
                .get("normal")
                .copied()
                .unwrap_or_default(),
            policy
                .risk_counts
                .get("sensitive")
                .copied()
                .unwrap_or_default(),
            policy
                .risk_counts
                .get("destructive")
                .copied()
                .unwrap_or_default()
        );
        for note in &policy.notes {
            println!("note: {note}");
        }
    }
    println!("Warnings: {}", report.warning_count);
    println!("Issues: {}", report.issue_count);
    for warning in &report.warnings {
        println!("warning: {warning}");
    }
    for issue in &report.issues {
        println!("- {issue}");
    }
}

fn print_run_report(report: &HeadlessRunReport) {
    println!("Headless run: {}", report.workflow_id);
    println!("Mode: {}", report.mode);
    println!("Status: {}", report.status);
    println!(
        "Executed steps: {}/{}",
        report.executed_step_count,
        report.steps.len()
    );
    if report.warning_count > 0 {
        println!("Warnings: {}", report.warning_count);
    }
    if let Some(blocked) = &report.blocked_by_confirmation {
        println!(
            "Blocked: step {} requires {:?} confirmation",
            blocked.index, blocked.risk
        );
    }
    for step in &report.steps {
        println!("{}. {} -> {}", step.index, step.action, step.status);
        println!("   payload: {}", step.payload);
        println!("   preview: {}", step.result_preview);
    }
}

fn engine_label(engine: &HeadlessEngine) -> &'static str {
    match engine {
        HeadlessEngine::Browser => "browser",
        HeadlessEngine::Service => "service",
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    let payload = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    println!("{payload}");
    Ok(())
}

#[derive(Debug, Default)]
struct Flags {
    positional: Vec<String>,
    json: bool,
    execute: bool,
    executor: Option<String>,
    allow_sensitive: bool,
    allow_destructive: bool,
    api_base_url: Option<String>,
    api_token: Option<String>,
    runtime: Option<String>,
    category: Option<String>,
    tag: Option<String>,
    query: Option<String>,
    template: Option<String>,
    workflow_id: Option<String>,
    out: Option<String>,
    report_out: Option<String>,
}

impl Flags {
    fn parse(args: &[String]) -> Self {
        let mut flags = Self::default();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--json" => flags.json = true,
                "--execute" => flags.execute = true,
                "--executor" => {
                    index += 1;
                    flags.executor = args.get(index).cloned();
                }
                "--allow-sensitive" => flags.allow_sensitive = true,
                "--allow-destructive" => flags.allow_destructive = true,
                "--api-base-url" => {
                    index += 1;
                    flags.api_base_url = args.get(index).cloned();
                }
                "--api-token" => {
                    index += 1;
                    flags.api_token = args.get(index).cloned();
                }
                "--runtime" | "--runtime-style" => {
                    index += 1;
                    flags.runtime = args.get(index).cloned();
                }
                "--category" => {
                    index += 1;
                    flags.category = args.get(index).cloned();
                }
                "--tag" => {
                    index += 1;
                    flags.tag = args.get(index).cloned();
                }
                "--query" | "--search" => {
                    index += 1;
                    flags.query = args.get(index).cloned();
                }
                "--template" => {
                    index += 1;
                    flags.template = args.get(index).cloned();
                }
                "--workflow-id" => {
                    index += 1;
                    flags.workflow_id = args.get(index).cloned();
                }
                "--out" => {
                    index += 1;
                    flags.out = args.get(index).cloned();
                }
                "--report-out" => {
                    index += 1;
                    flags.report_out = args.get(index).cloned();
                }
                value if value.starts_with("--") => {}
                value => flags.positional.push(value.to_string()),
            }
            index += 1;
        }
        flags
    }

    fn input_path(&self) -> Result<String, String> {
        self.positional
            .first()
            .cloned()
            .ok_or_else(|| "command requires an input path".to_string())
    }
}

#[derive(Debug, Serialize)]
struct TemplateListOutput {
    template_count: usize,
    templates: Vec<TemplateView>,
}

#[derive(Debug, Serialize)]
struct TemplateView {
    id: String,
    title: String,
    description: String,
    runtime_style: HeadlessRuntimeStyle,
    category: String,
    tags: Vec<String>,
    step_count: usize,
    actions: Vec<String>,
}

impl TemplateView {
    fn from_descriptor(template: &HeadlessTemplateDescriptor) -> Self {
        Self {
            id: template.id.to_string(),
            title: template.title.to_string(),
            description: template.description.to_string(),
            runtime_style: template.runtime_style,
            category: template.category.to_string(),
            tags: template.tags.iter().map(|tag| (*tag).to_string()).collect(),
            step_count: template_actions(template.id).len(),
            actions: template_actions(template.id),
        }
    }
}
