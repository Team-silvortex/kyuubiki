use crate::central_database_readiness;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod markdown;

type RunnerResult<T> = Result<T, String>;

pub(super) const REPORT_SCHEMA: &str = "kyuubiki.central-readiness-report/v1";
pub(super) const STORAGE_SCHEMA: &str = "kyuubiki.central-database-contract/v1";
const DEFAULT_OUT: &str = "tmp/central-readiness-report.json";
pub(super) const ENDPOINTS: &[&str] = &[
    "/api/v1/central/catalog",
    "/api/v1/central/session-policy",
    "/api/v1/central/publish-policy",
    "/api/v1/central/publish-readiness",
    "/api/v1/central/database-policy",
    "/api/v1/central/provenance-policy",
    "/api/v1/central/database-status",
];
pub(super) const SCHEMA_FILES: &[&str] = &[
    "schemas/central-store-contract-check.schema.json",
    "schemas/central-store-catalog.schema.json",
    "schemas/central-session-policy.schema.json",
    "schemas/central-publish-policy.schema.json",
    "schemas/central-publish-readiness.schema.json",
    "schemas/central-database-policy.schema.json",
    "schemas/central-provenance-policy.schema.json",
    "schemas/central-database-status.schema.json",
    "schemas/central-readiness-report.schema.json",
];
pub(super) const CONFIG_FILES: &[&str] = &[
    "config/architecture/central-store-contract.json",
    "config/architecture/module-topology.json",
];

pub(crate) fn run_build_central_readiness_report(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = BuildOptions::parse(args)?;
    if options.self_test {
        let report = build_report("local", "sqlite", self_test_readiness(), self_test_files());
        let issues = validate_report(&report);
        if !issues.is_empty() {
            return Err(format!("self-test failed: {}", issues.join("; ")));
        }
        println!("central readiness report self-test passed");
        return Ok(0);
    }

    let mode = options
        .mode
        .unwrap_or_else(|| env::var("MODE").unwrap_or_else(|_| "local".to_string()));
    let backend = options
        .backend
        .unwrap_or_else(|| env::var("BACKEND").unwrap_or_else(|_| "sqlite".to_string()));
    let readiness = central_database_readiness::build_readiness_report(root, &mode, &backend)?;
    if readiness.get("status").and_then(Value::as_str) != Some("ok") {
        println!(
            "{}",
            serde_json::to_string_pretty(&readiness)
                .map_err(|error| format!("failed to render readiness: {error}"))?
        );
        return Ok(1);
    }
    let report = build_report(&mode, &backend, readiness, read_required_files(root)?);
    let issues = validate_report(&report);
    if !issues.is_empty() {
        return Err(format!(
            "central readiness report invalid: {}",
            issues.join("; ")
        ));
    }
    let out = options
        .out
        .unwrap_or_else(|| env::var("OUT").unwrap_or_else(|_| DEFAULT_OUT.to_string()));
    let markdown = options.markdown_out.unwrap_or_else(|| {
        env::var("MARKDOWN_OUT").unwrap_or_else(|_| sibling_markdown_path(&out))
    });
    write_json(root, &out, &report)?;
    write_text(root, &markdown, &markdown::render_markdown(&report))?;
    println!("central readiness report written: {out}");
    println!("central readiness summary written: {markdown}");
    Ok(0)
}

pub(crate) fn run_check_central_readiness_report(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = CheckOptions::parse(args)?;
    if options.self_test {
        let mut issues = validate_report(&self_test_checker_report());
        issues.extend(markdown::validate_markdown(&markdown::markdown_fixture(
            &self_test_checker_report(),
        )));
        if !issues.is_empty() {
            return Err(format!("self-test failed: {}", issues.join("; ")));
        }
        println!("central readiness report checker self-test passed");
        return Ok(0);
    }
    let input = options
        .input
        .unwrap_or_else(|| env::var("IN").unwrap_or_else(|_| DEFAULT_OUT.to_string()));
    let markdown = options.markdown_in.unwrap_or_else(|| {
        env::var("MARKDOWN_IN").unwrap_or_else(|_| sibling_markdown_path(&input))
    });
    let report = read_json(root, &input)?;
    let mut issues = validate_report(&report);
    let markdown_path = repo_path(root, &markdown)?;
    if markdown_path.exists() {
        issues.extend(markdown::validate_markdown(
            &fs::read_to_string(&markdown_path)
                .map_err(|error| format!("failed to read {}: {error}", markdown_path.display()))?,
        ));
    } else {
        issues.push(format!("missing markdown summary {markdown}"));
    }
    if !issues.is_empty() {
        return Err(format!(
            "central readiness report failed: {}",
            issues.join("; ")
        ));
    }
    println!("central readiness report passed: {input}");
    Ok(0)
}

fn build_report(
    mode: &str,
    backend: &str,
    readiness: Value,
    files: HashMap<String, String>,
) -> Value {
    json!({
        "schema_version": REPORT_SCHEMA,
        "generated_at": generated_at(),
        "mode": mode,
        "backend": backend,
        "status": if readiness.get("status").and_then(Value::as_str) == Some("ok") { "ok" } else { "fail" },
        "readiness": readiness,
        "api_surface": { "endpoints": ENDPOINTS.iter().map(|endpoint| endpoint_status(endpoint, &files)).collect::<Vec<_>>() },
        "schema_surface": { "schema_files": SCHEMA_FILES.iter().map(|file| json!({ "path": file, "present": files.contains_key(*file) })).collect::<Vec<_>>() },
        "config_surface": { "config_files": CONFIG_FILES.iter().map(|file| config_status(file, &files)).collect::<Vec<_>>() },
        "service_surface": {
            "id": "central-web-service",
            "module_id": "orchestra-control-plane",
            "kind": "self_host_web",
            "topology_present": includes_all(files.get("config/architecture/module-topology.json"), &["central-web-service", "self_host_web", "orchestra-control-plane"]),
            "boundary_documented": includes_all(files.get("docs/central-server-components.md"), &["central-web-service", "not a separate top-level module"])
        },
        "storage_contract": {
            "schema_version": STORAGE_SCHEMA,
            "table_contract_present": includes_all(files.get("apps/web/lib/kyuubiki_web/storage/central_database.ex"), &[STORAGE_SCHEMA, "central_store_entries", "central_artifact_signatures"])
        },
        "runbook": {
            "local_readiness": "make check-central-database-readiness MODE=local BACKEND=sqlite",
            "remote_dry_run": "make remote-central-database-smoke REMOTE=kyuubiki-lab",
            "postgres_smoke": "RUN_DB_SMOKE=1 MODE=cloud BACKEND=postgres make test-central-database-smoke"
        }
    })
}

fn endpoint_status(endpoint: &str, files: &HashMap<String, String>) -> Value {
    let suffix = endpoint.strip_prefix("/api/v1/central").unwrap_or(endpoint);
    json!({
        "path": endpoint,
        "router_present": files.get("apps/web/lib/kyuubiki_web/central_store_router.ex").is_some_and(|text| text.contains(suffix)),
        "client_present": files.get("apps/frontend/src/lib/api/central-store-client.ts").is_some_and(|text| text.contains(endpoint))
    })
}

fn config_status(file: &str, files: &HashMap<String, String>) -> Value {
    let expected = if file == "config/architecture/central-store-contract.json" {
        "kyuubiki.central-store-contract-check/v1"
    } else {
        "kyuubiki.module-topology/v1"
    };
    json!({
        "path": file,
        "present": files.contains_key(file),
        "schema_version_present": files.get(file).is_some_and(|text| text.contains(expected))
    })
}

fn validate_report(report: &Value) -> Vec<String> {
    let mut issues = Vec::new();
    if report.get("schema_version").and_then(Value::as_str) != Some(REPORT_SCHEMA) {
        issues.push("unexpected schema_version".to_string());
    }
    if report.get("status").and_then(Value::as_str) != Some("ok") {
        issues.push("status must be ok".to_string());
    }
    validate_items(
        &mut issues,
        report.pointer("/api_surface/endpoints"),
        ENDPOINTS,
        "endpoint",
        true,
    );
    validate_items(
        &mut issues,
        report.pointer("/schema_surface/schema_files"),
        SCHEMA_FILES,
        "schema",
        false,
    );
    validate_items(
        &mut issues,
        report.pointer("/config_surface/config_files"),
        CONFIG_FILES,
        "config",
        false,
    );
    if report
        .pointer("/storage_contract/schema_version")
        .and_then(Value::as_str)
        != Some(STORAGE_SCHEMA)
    {
        issues.push("storage contract schema version mismatch".to_string());
    }
    if report
        .pointer("/storage_contract/table_contract_present")
        .and_then(Value::as_bool)
        != Some(true)
    {
        issues.push("storage table contract missing".to_string());
    }
    for (pointer, expected, label) in [
        (
            "/service_surface/id",
            "central-web-service",
            "central web service surface id missing",
        ),
        (
            "/service_surface/module_id",
            "orchestra-control-plane",
            "central web service module binding mismatch",
        ),
        (
            "/service_surface/kind",
            "self_host_web",
            "central web service surface kind mismatch",
        ),
    ] {
        if report.pointer(pointer).and_then(Value::as_str) != Some(expected) {
            issues.push(label.to_string());
        }
    }
    if report
        .pointer("/service_surface/topology_present")
        .and_then(Value::as_bool)
        != Some(true)
    {
        issues.push("central web service topology coverage missing".to_string());
    }
    if report
        .pointer("/service_surface/boundary_documented")
        .and_then(Value::as_bool)
        != Some(true)
    {
        issues.push("central web service boundary documentation missing".to_string());
    }
    issues.extend(unsafe_text_issues(&report.to_string()));
    issues
}

fn validate_items(
    issues: &mut Vec<String>,
    items: Option<&Value>,
    required: &[&str],
    label: &str,
    endpoint: bool,
) {
    let rows = items.and_then(Value::as_array).cloned().unwrap_or_default();
    for required_path in required {
        let actual = rows
            .iter()
            .find(|entry| entry.get("path").and_then(Value::as_str) == Some(*required_path));
        if actual.is_none() {
            issues.push(format!("missing {label} {required_path}"));
            continue;
        }
        let actual = actual.unwrap();
        if endpoint {
            if actual.get("router_present").and_then(Value::as_bool) != Some(true) {
                issues.push(format!("router missing {required_path}"));
            }
            if actual.get("client_present").and_then(Value::as_bool) != Some(true) {
                issues.push(format!("client missing {required_path}"));
            }
        } else if actual.get("present").and_then(Value::as_bool) != Some(true) {
            issues.push(format!("{label} not present {required_path}"));
        }
        if label == "config"
            && actual
                .get("schema_version_present")
                .and_then(Value::as_bool)
                != Some(true)
        {
            issues.push(format!("config schema version missing {required_path}"));
        }
    }
}

fn required_files() -> Vec<&'static str> {
    let mut files = vec![
        "apps/web/lib/kyuubiki_web/central_store_router.ex",
        "apps/frontend/src/lib/api/central-store-client.ts",
        "apps/web/lib/kyuubiki_web/storage/central_database.ex",
        "docs/central-server-components.md",
    ];
    files.extend(SCHEMA_FILES);
    files.extend(CONFIG_FILES);
    files
}

fn read_required_files(root: &Path) -> RunnerResult<HashMap<String, String>> {
    let mut files = HashMap::new();
    for file in required_files() {
        files.insert(
            file.to_string(),
            fs::read_to_string(root.join(file))
                .map_err(|error| format!("failed to read {file}: {error}"))?,
        );
    }
    Ok(files)
}

fn self_test_files() -> HashMap<String, String> {
    let mut files = HashMap::new();
    files.insert(
        "apps/web/lib/kyuubiki_web/central_store_router.ex".to_string(),
        ENDPOINTS
            .iter()
            .map(|endpoint| endpoint.replace("/api/v1/central", ""))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    files.insert(
        "apps/frontend/src/lib/api/central-store-client.ts".to_string(),
        ENDPOINTS.join("\n"),
    );
    files.insert(
        "apps/web/lib/kyuubiki_web/storage/central_database.ex".to_string(),
        format!("{STORAGE_SCHEMA} central_store_entries central_artifact_signatures"),
    );
    for file in SCHEMA_FILES {
        files.insert((*file).to_string(), "{}".to_string());
    }
    files.insert(
        "config/architecture/central-store-contract.json".to_string(),
        "kyuubiki.central-store-contract-check/v1".to_string(),
    );
    files.insert(
        "config/architecture/module-topology.json".to_string(),
        "kyuubiki.module-topology/v1 central-web-service self_host_web orchestra-control-plane"
            .to_string(),
    );
    files.insert(
        "docs/central-server-components.md".to_string(),
        "central-web-service not a separate top-level module".to_string(),
    );
    files
}

fn self_test_readiness() -> Value {
    json!({"schema_version": "kyuubiki.central-database-readiness/v1", "status": "ok", "issues": [], "checks": {"static_contract_files": ["a"]}})
}

fn self_test_checker_report() -> Value {
    build_report("local", "sqlite", self_test_readiness(), self_test_files())
}

pub(super) fn unsafe_text_issues(text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    for (needle, label) in [
        ("ssh_password", "ssh password field"),
        ("ssh_pass", "ssh password field"),
    ] {
        if text.to_ascii_lowercase().contains(needle) {
            issues.push(format!("unsafe text detected: {label}"));
        }
    }
    if text.contains("ecto://") && text.contains('@') {
        issues.push("unsafe text detected: inline ecto credential".to_string());
    }
    if text.contains("DATABASE_URL=") && text.contains("://") && text.contains('@') {
        issues.push("unsafe text detected: inline DATABASE_URL secret".to_string());
    }
    issues
}

fn includes_all(text: Option<&String>, needles: &[&str]) -> bool {
    text.is_some_and(|text| needles.iter().all(|needle| text.contains(needle)))
}

fn generated_at() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    format!("unix:{seconds}")
}

fn sibling_markdown_path(path: &str) -> String {
    path.strip_suffix(".json")
        .map_or_else(|| format!("{path}.md"), |stem| format!("{stem}.md"))
}

fn write_json(root: &Path, relative_path: &str, value: &Value) -> RunnerResult<()> {
    write_text(
        root,
        relative_path,
        &format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|error| format!("failed to render json: {error}"))?
        ),
    )
}

fn write_text(root: &Path, relative_path: &str, text: &str) -> RunnerResult<()> {
    let path = repo_path(root, relative_path)?;
    fs::create_dir_all(
        path.parent()
            .ok_or_else(|| format!("path has no parent: {relative_path}"))?,
    )
    .map_err(|error| format!("failed to create parent for {relative_path}: {error}"))?;
    fs::write(&path, text).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let path = repo_path(root, relative_path)?;
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn repo_path(root: &Path, relative_path: &str) -> RunnerResult<PathBuf> {
    if relative_path.is_empty()
        || relative_path.starts_with('/')
        || relative_path.split('/').any(|part| part == "..")
    {
        return Err(format!("path must stay inside repository: {relative_path}"));
    }
    Ok(root.join(relative_path))
}

#[derive(Default)]
struct BuildOptions {
    mode: Option<String>,
    backend: Option<String>,
    out: Option<String>,
    markdown_out: Option<String>,
    self_test: bool,
}

#[derive(Default)]
struct CheckOptions {
    input: Option<String>,
    markdown_in: Option<String>,
    self_test: bool,
}

impl BuildOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        parse_options(args, |flag, value| match flag {
            "--mode" => {
                options.mode = Some(value()?);
                Ok(())
            }
            "--backend" => {
                options.backend = Some(value()?);
                Ok(())
            }
            "--out" => {
                options.out = Some(value()?);
                Ok(())
            }
            "--markdown-out" => {
                options.markdown_out = Some(value()?);
                Ok(())
            }
            "--self-test" => {
                options.self_test = true;
                Ok(())
            }
            _ => return Err(format!("unknown argument {flag}")),
        })?;
        Ok(options)
    }
}

impl CheckOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        parse_options(args, |flag, value| match flag {
            "--in" => {
                options.input = Some(value()?);
                Ok(())
            }
            "--markdown-in" => {
                options.markdown_in = Some(value()?);
                Ok(())
            }
            "--self-test" => {
                options.self_test = true;
                Ok(())
            }
            _ => return Err(format!("unknown argument {flag}")),
        })?;
        Ok(options)
    }
}

fn parse_options<F>(args: Vec<OsString>, mut visit: F) -> RunnerResult<()>
where
    F: FnMut(&str, &mut dyn FnMut() -> RunnerResult<String>) -> RunnerResult<()>,
{
    let mut index = 0;
    while index < args.len() {
        let arg = args[index]
            .to_str()
            .ok_or_else(|| "non-utf8 argument".to_string())?;
        let mut consumed_value = false;
        let mut value = || {
            consumed_value = true;
            args.get(index + 1)
                .and_then(|item| item.to_str())
                .map(str::to_string)
                .ok_or_else(|| format!("{arg} requires a value"))
        };
        visit(arg, &mut value)?;
        index += if consumed_value { 2 } else { 1 };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{markdown, self_test_checker_report, validate_report};

    #[test]
    fn self_test_report_is_valid() {
        assert!(validate_report(&self_test_checker_report()).is_empty());
    }

    #[test]
    fn markdown_fixture_is_valid() {
        assert!(
            markdown::validate_markdown("# nope")
                .iter()
                .any(|issue| issue.contains("markdown missing"))
        );
    }
}
