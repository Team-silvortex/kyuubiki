use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const DEFAULT_SURFACE: &str = "config/architecture/contracts-runtime-api-surface.json";
const SCHEMA_PATH: &str = "schemas/contracts-runtime-api-surface.schema.json";
const SCHEMA_VERSION: &str = "kyuubiki.contracts-runtime-api-surface/v1";
const REQUIRED_FAMILIES: &[&str] = &[
    "frontend-runtime-api",
    "protocol-runtime-api",
    "orchestra-runtime-api",
    "central-store-runtime-api",
];

pub(crate) fn run_check_contracts_runtime_api_surface(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    let surface = if options.self_test {
        self_test_surface()
    } else {
        read_json(root, &options.surface)?
    };
    let report = validate_surface(root, &surface);
    if !report.errors.is_empty() {
        let prefix = if options.self_test {
            "contracts-runtime-api-surface self-test"
        } else {
            "contracts-runtime-api-surface"
        };
        for error in report.errors {
            eprintln!("{prefix}: {error}");
        }
        return Ok(1);
    }
    if options.self_test {
        println!("contracts runtime API surface self-test passed");
    } else {
        println!(
            "contracts runtime API surface passed: {} contract family(s)",
            report.family_count
        );
    }
    Ok(0)
}

fn validate_surface(root: &Path, surface: &Value) -> Report {
    let mut errors = Vec::new();
    if field(surface, "schema_version") != Some(SCHEMA_VERSION) {
        errors.push("unexpected contracts runtime API surface schema_version".to_string());
    }
    if field(surface, "module_id") != Some("contracts") {
        errors.push("contracts runtime API surface must belong to contracts module".to_string());
    }
    if field(surface, "$schema") != Some("../../schemas/contracts-runtime-api-surface.schema.json")
    {
        errors.push("contracts runtime API surface must declare its schema".to_string());
    }
    if surface
        .pointer("/runtime_api/owner")
        .and_then(Value::as_str)
        != Some("contracts")
    {
        errors.push("contracts runtime API owner must be contracts".to_string());
    }
    validate_schema_contract(root, &mut errors);

    let families = surface
        .pointer("/runtime_api/contract_families")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen = BTreeSet::new();
    for family in &families {
        let id = field(family, "id").unwrap_or_default();
        if !seen.insert(id.to_string()) {
            errors.push(format!("duplicate contract family: {id}"));
        }
    }
    for family_id in REQUIRED_FAMILIES {
        if !families
            .iter()
            .any(|family| field(family, "id") == Some(*family_id))
        {
            errors.push(format!("missing contract family: {family_id}"));
        }
    }
    if let Some(central) = families
        .iter()
        .find(|family| field(family, "id") == Some("central-store-runtime-api"))
    {
        validate_central_family(&mut errors, central);
    }
    for family in &families {
        validate_family(root, &mut errors, family);
    }
    for command in string_array_at(surface, "/runtime_api/verification_commands") {
        if !command.starts_with("node ") && !command.starts_with("make ") {
            errors.push(format!(
                "unsupported verification command prefix: {command}"
            ));
        }
    }
    Report {
        errors,
        family_count: families.len(),
    }
}

fn validate_central_family(errors: &mut Vec<String>, family: &Value) {
    for source in [
        "config/architecture/central-store-contract.json",
        "schemas/central-store-contract-check.schema.json",
        "apps/web/lib/kyuubiki_web/storage/central_database.ex",
        "schemas/central-database-status.schema.json",
    ] {
        require_includes(
            errors,
            &string_array(family, "sources"),
            source,
            "central-store-runtime-api source",
        );
    }
    for contract in [
        "database status",
        "self-hosted website service surface",
        "central database table contract",
    ] {
        require_includes(
            errors,
            &string_array(family, "stability_contracts"),
            contract,
            "central-store-runtime-api stability contract",
        );
    }
    require_service_surface(errors, family);
}

fn validate_family(root: &Path, errors: &mut Vec<String>, family: &Value) {
    let id = field(family, "id").unwrap_or_default();
    let sources = string_array(family, "sources");
    if sources.is_empty() {
        errors.push(format!("{id} has no source files"));
        return;
    }
    for source in sources {
        if repo_path(root, &source).is_err() {
            errors.push(format!("{id} source must be repository-relative: {source}"));
            continue;
        }
        if !root.join(&source).exists() {
            errors.push(format!("{id} source does not exist: {source}"));
        }
    }
    if string_array(family, "client_surfaces").is_empty() {
        errors.push(format!("{id} has no client surfaces"));
    }
    if string_array(family, "stability_contracts").is_empty() {
        errors.push(format!("{id} has no stability contracts"));
    }
}

fn validate_schema_contract(root: &Path, errors: &mut Vec<String>) {
    let Ok(schema_path) = repo_path(root, SCHEMA_PATH) else {
        errors.push(format!("schema does not exist: {SCHEMA_PATH}"));
        return;
    };
    let Ok(schema_text) = fs::read_to_string(&schema_path) else {
        errors.push(format!("schema does not exist: {SCHEMA_PATH}"));
        return;
    };
    match serde_json::from_str::<Value>(&schema_text) {
        Ok(schema) => {
            if schema
                .pointer("/properties/schema_version/const")
                .and_then(Value::as_str)
                != Some(SCHEMA_VERSION)
            {
                errors.push("contracts runtime API schema version const mismatch".to_string());
            }
        }
        Err(error) => errors.push(format!("{SCHEMA_PATH}: {error}")),
    }
    if !schema_text.contains("repoPath") || !schema_text.contains("^(?!/)") {
        errors.push(
            "contracts runtime API schema must define repo-relative path pattern".to_string(),
        );
    }
    if !schema_text.contains("serviceSurface") {
        errors.push("contracts runtime API schema must define service surface shape".to_string());
    }
}

fn require_includes(errors: &mut Vec<String>, values: &[String], expected: &str, label: &str) {
    if !values.iter().any(|value| value == expected) {
        errors.push(format!("{label} missing {expected}"));
    }
}

fn require_service_surface(errors: &mut Vec<String>, family: &Value) {
    let surfaces = family
        .get("service_surfaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(actual) = surfaces
        .iter()
        .find(|surface| field(surface, "id") == Some("central-web-service"))
    else {
        errors.push(
            "central-store-runtime-api service surface missing central-web-service".to_string(),
        );
        return;
    };
    for (field_name, expected) in [
        ("id", "central-web-service"),
        ("module_id", "orchestra-control-plane"),
        ("kind", "self_host_web"),
    ] {
        if field(actual, field_name) != Some(expected) {
            errors.push(format!(
                "central-store-runtime-api service surface central-web-service {field_name} mismatch"
            ));
        }
    }
}

fn self_test_surface() -> Value {
    json!({
        "$schema": "../../schemas/contracts-runtime-api-surface.schema.json",
        "schema_version": SCHEMA_VERSION,
        "module_id": "contracts",
        "runtime_api": {
            "owner": "contracts",
            "contract_families": [
                family("frontend-runtime-api", "workbench-shell", "typed payloads"),
                family("protocol-runtime-api", "runtime-agent-cli", "TaskIR"),
                family("orchestra-runtime-api", "orchestra-control-plane", "control-plane surface"),
                {
                    "id": "central-store-runtime-api",
                    "sources": [
                        "scripts/check-contracts-runtime-api-surface.mjs",
                        "config/architecture/central-store-contract.json",
                        "schemas/central-store-contract-check.schema.json",
                        "apps/web/lib/kyuubiki_web/storage/central_database.ex",
                        "schemas/central-database-status.schema.json"
                    ],
                    "client_surfaces": ["workbench-shell"],
                    "service_surfaces": [{
                        "id": "central-web-service",
                        "module_id": "orchestra-control-plane",
                        "kind": "self_host_web"
                    }],
                    "stability_contracts": [
                        "central store catalog",
                        "database status",
                        "self-hosted website service surface",
                        "central database table contract"
                    ]
                }
            ],
            "verification_commands": ["node scripts/check-contracts-runtime-api-surface.mjs"]
        }
    })
}

fn family(id: &str, client: &str, contract: &str) -> Value {
    json!({
        "id": id,
        "sources": ["scripts/check-contracts-runtime-api-surface.mjs"],
        "client_surfaces": [client],
        "stability_contracts": [contract]
    })
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
        || relative_path.split(['/', '\\']).any(|part| part == "..")
        || looks_like_windows_absolute(relative_path)
    {
        return Err(format!("path must stay inside repository: {relative_path}"));
    }
    Ok(root.join(relative_path))
}

fn looks_like_windows_absolute(value: &str) -> bool {
    value.len() > 2
        && value.as_bytes()[1] == b':'
        && matches!(value.as_bytes()[2], b'/' | b'\\')
        && value.as_bytes()[0].is_ascii_alphabetic()
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn string_array_at(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

struct Report {
    errors: Vec<String>,
    family_count: usize,
}

struct Options {
    surface: String,
    self_test: bool,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            surface: DEFAULT_SURFACE.to_string(),
            self_test: false,
        };
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
                "--surface" => {
                    options.surface = args
                        .get(index + 1)
                        .and_then(|value| value.to_str())
                        .ok_or_else(|| "--surface requires a value".to_string())?
                        .to_string();
                    index += 2;
                }
                _ => return Err(format!("unknown argument {arg}")),
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::{looks_like_windows_absolute, repo_path};
    use std::path::Path;

    #[test]
    fn rejects_absolute_paths() {
        assert!(repo_path(Path::new("."), "/tmp/x").is_err());
        assert!(looks_like_windows_absolute("C:\\tmp\\x"));
    }
}
