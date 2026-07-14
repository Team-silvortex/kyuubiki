use serde_json::{Value, json};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const SCHEMA_VERSION: &str = "kyuubiki.central-database-readiness/v1";
const DEFAULT_SQLITE_PATH: &str = "./tmp/data/kyuubiki_dev.sqlite3";
const REQUIRED_FILES: &[&str] = &[
    "apps/web/config/config.exs",
    "apps/web/lib/kyuubiki_web/central_store.ex",
    "apps/web/lib/kyuubiki_web/central_store_router.ex",
    "apps/web/lib/kyuubiki_web/storage/central_database.ex",
    "apps/web/lib/kyuubiki_web/storage/schema_setup.ex",
    "apps/frontend/src/lib/api/central-store-client.ts",
    "apps/frontend/src/lib/api/central-store-types.ts",
    "schemas/central-database-policy.schema.json",
    "schemas/central-database-status.schema.json",
];

pub(crate) fn run_check_central_database_readiness(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    if options.self_test {
        let report = validate("cloud", "postgres", &self_test_env(), self_test_files());
        if has_issues(&report) {
            for issue in issues(&report) {
                eprintln!("central-database-readiness self-test: {issue}");
            }
            return Ok(1);
        }
        println!("central database readiness self-test passed");
        return Ok(0);
    }

    let mode = options
        .mode
        .or_else(|| env::var("KYUUBIKI_DEPLOYMENT_MODE").ok())
        .unwrap_or_else(|| "local".to_string());
    let backend = options
        .backend
        .or_else(|| env::var("KYUUBIKI_STORAGE_BACKEND").ok())
        .unwrap_or_else(|| default_backend_for_mode(&mode).to_string());
    let report = build_readiness_report(root, &mode, &backend)?;

    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to render readiness report: {error}"))?
        );
    }
    if has_issues(&report) {
        if !options.json {
            eprintln!("central database readiness failed:");
            for issue in issues(&report) {
                eprintln!("- {issue}");
            }
        }
        return Ok(1);
    }
    if !options.json {
        println!("central database readiness ok: mode={mode}, backend={backend}");
    }
    Ok(0)
}

pub(crate) fn build_readiness_report(
    root: &Path,
    mode: &str,
    backend: &str,
) -> RunnerResult<Value> {
    let files = read_required_files(root)?;
    let env_map = env::vars().collect::<HashMap<_, _>>();
    Ok(validate(mode, backend, &env_map, files))
}

fn validate(
    mode: &str,
    backend: &str,
    env_map: &HashMap<String, String>,
    files: HashMap<String, String>,
) -> Value {
    let mut issues = Vec::new();
    if !matches!(backend, "sqlite" | "postgres") {
        issues.push(format!(
            "unsupported backend {backend}; expected sqlite or postgres"
        ));
    }
    if matches!(mode, "cloud" | "distributed") && backend != "postgres" {
        issues.push(format!("{mode} mode must use postgres backend"));
    }
    if backend == "postgres" && !env_map.contains_key("DATABASE_URL") {
        issues.push("DATABASE_URL is required for postgres backend".to_string());
    }
    let sqlite_path = env_map
        .get("SQLITE_DATABASE_PATH")
        .map(String::as_str)
        .unwrap_or(DEFAULT_SQLITE_PATH);
    if backend == "sqlite" && !sqlite_path.ends_with(".sqlite3") {
        issues.push("SQLITE_DATABASE_PATH should point to a .sqlite3 file".to_string());
    }
    for (file, text) in &files {
        if text.is_empty() {
            issues.push(format!("missing required file: {file}"));
        }
    }
    for (file, needle) in required_text_checks() {
        require_contains(&mut issues, &files, file, needle);
    }

    json!({
        "schema_version": SCHEMA_VERSION,
        "mode": mode,
        "backend": backend,
        "status": if issues.is_empty() { "ok" } else { "fail" },
        "checks": {
            "static_contract_files": REQUIRED_FILES,
            "postgres_requires_database_url": backend == "postgres",
            "sqlite_path": sqlite_path
        },
        "issues": issues
    })
}

fn required_text_checks() -> Vec<(&'static str, &'static str)> {
    vec![
        ("apps/web/config/config.exs", "KYUUBIKI_STORAGE_BACKEND"),
        ("apps/web/config/config.exs", "DATABASE_URL"),
        ("apps/web/config/config.exs", "SQLITE_DATABASE_PATH"),
        (
            "apps/web/lib/kyuubiki_web/central_store.ex",
            "kyuubiki.central-database-policy/v1",
        ),
        (
            "apps/web/lib/kyuubiki_web/central_store.ex",
            "CentralDatabase.table_specs",
        ),
        (
            "apps/web/lib/kyuubiki_web/central_store_router.ex",
            "/database-policy",
        ),
        (
            "apps/web/lib/kyuubiki_web/central_store_router.ex",
            "/database-status",
        ),
        (
            "apps/web/lib/kyuubiki_web/storage/central_database.ex",
            "kyuubiki.central-database-contract/v1",
        ),
        (
            "apps/web/lib/kyuubiki_web/storage/central_database.ex",
            "central_store_entries",
        ),
        (
            "apps/web/lib/kyuubiki_web/storage/central_database.ex",
            "central_artifact_signatures",
        ),
        (
            "apps/web/lib/kyuubiki_web/storage/schema_setup.ex",
            "CentralDatabase.create_table_sqls",
        ),
        (
            "apps/frontend/src/lib/api/central-store-client.ts",
            "/api/v1/central/database-policy",
        ),
        (
            "apps/frontend/src/lib/api/central-store-client.ts",
            "/api/v1/central/database-status",
        ),
        (
            "apps/frontend/src/lib/api/central-store-types.ts",
            "CentralDatabaseTableSpec",
        ),
        (
            "apps/frontend/src/lib/api/central-store-types.ts",
            "CentralDatabaseStatusPayload",
        ),
        (
            "apps/frontend/src/lib/api/central-store-types.ts",
            "kyuubiki.central-database-contract/v1",
        ),
        (
            "schemas/central-database-policy.schema.json",
            "kyuubiki.central-database-policy/v1",
        ),
        (
            "schemas/central-database-policy.schema.json",
            "kyuubiki.central-database-contract/v1",
        ),
        (
            "schemas/central-database-status.schema.json",
            "kyuubiki.central-database-status/v1",
        ),
    ]
}

fn require_contains(
    issues: &mut Vec<String>,
    files: &HashMap<String, String>,
    file: &str,
    needle: &str,
) {
    if !files.get(file).is_some_and(|text| text.contains(needle)) {
        issues.push(format!("{file} must include {needle}"));
    }
}

fn read_required_files(root: &Path) -> RunnerResult<HashMap<String, String>> {
    let mut files = HashMap::new();
    for file in REQUIRED_FILES {
        let text = fs::read_to_string(root.join(file))
            .map_err(|error| format!("failed to read {file}: {error}"))?;
        files.insert((*file).to_string(), text);
    }
    Ok(files)
}

fn self_test_files() -> HashMap<String, String> {
    HashMap::from([
        (
            "apps/web/config/config.exs".to_string(),
            "KYUUBIKI_STORAGE_BACKEND DATABASE_URL SQLITE_DATABASE_PATH".to_string(),
        ),
        (
            "apps/web/lib/kyuubiki_web/central_store.ex".to_string(),
            "kyuubiki.central-database-policy/v1 CentralDatabase.table_specs".to_string(),
        ),
        (
            "apps/web/lib/kyuubiki_web/central_store_router.ex".to_string(),
            "/database-policy /database-status".to_string(),
        ),
        (
            "apps/web/lib/kyuubiki_web/storage/central_database.ex".to_string(),
            "kyuubiki.central-database-contract/v1 central_store_entries central_artifact_signatures".to_string(),
        ),
        (
            "apps/web/lib/kyuubiki_web/storage/schema_setup.ex".to_string(),
            "CentralDatabase.create_table_sqls".to_string(),
        ),
        (
            "apps/frontend/src/lib/api/central-store-client.ts".to_string(),
            "/api/v1/central/database-policy /api/v1/central/database-status".to_string(),
        ),
        (
            "apps/frontend/src/lib/api/central-store-types.ts".to_string(),
            "CentralDatabaseTableSpec CentralDatabaseStatusPayload kyuubiki.central-database-contract/v1".to_string(),
        ),
        (
            "schemas/central-database-policy.schema.json".to_string(),
            "kyuubiki.central-database-policy/v1 kyuubiki.central-database-contract/v1".to_string(),
        ),
        (
            "schemas/central-database-status.schema.json".to_string(),
            "kyuubiki.central-database-status/v1".to_string(),
        ),
    ])
}

fn self_test_env() -> HashMap<String, String> {
    HashMap::from([(
        "DATABASE_URL".to_string(),
        "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev".to_string(),
    )])
}

fn default_backend_for_mode(mode: &str) -> &'static str {
    if matches!(mode, "cloud" | "distributed") {
        "postgres"
    } else {
        "sqlite"
    }
}

fn has_issues(report: &Value) -> bool {
    !issues(report).is_empty()
}

fn issues(report: &Value) -> Vec<String> {
    report
        .get("issues")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

#[derive(Default)]
struct Options {
    mode: Option<String>,
    backend: Option<String>,
    json: bool,
    self_test: bool,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        let mut index = 0;
        while index < args.len() {
            let arg = args[index]
                .to_str()
                .ok_or_else(|| "non-utf8 argument".to_string())?;
            match arg {
                "--self-test" => {
                    options.self_test = true;
                    index += 1;
                }
                "--json" => {
                    options.json = true;
                    index += 1;
                }
                "--mode" => {
                    options.mode = Some(next_arg(&args, index, "--mode")?.to_string());
                    index += 2;
                }
                "--backend" => {
                    options.backend = Some(next_arg(&args, index, "--backend")?.to_string());
                    index += 2;
                }
                _ => return Err(format!("unknown argument {arg}")),
            }
        }
        Ok(options)
    }
}

fn next_arg<'a>(args: &'a [OsString], index: usize, flag: &str) -> RunnerResult<&'a str> {
    args.get(index + 1)
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("{flag} requires a value"))
}

#[cfg(test)]
mod tests {
    use super::{default_backend_for_mode, has_issues, self_test_env, self_test_files, validate};

    #[test]
    fn cloud_defaults_to_postgres() {
        assert_eq!(default_backend_for_mode("cloud"), "postgres");
        assert_eq!(default_backend_for_mode("local"), "sqlite");
    }

    #[test]
    fn self_test_fixture_is_ready() {
        let report = validate("cloud", "postgres", &self_test_env(), self_test_files());
        assert!(!has_issues(&report));
    }
}
