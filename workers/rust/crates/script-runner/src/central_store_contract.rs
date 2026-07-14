use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const DEFAULT_CONFIG: &str = "config/architecture/central-store-contract.json";
const CONFIG_SCHEMA: &str = "schemas/central-store-contract-check.schema.json";
const CONFIG_SCHEMA_VERSION: &str = "kyuubiki.central-store-contract-check/v1";
const REPO_PATH_PATTERN: &str = "^(?!/)(?![A-Za-z]:)(?!.*(^|/)\\.\\.(/|$)).+";

pub(crate) fn run_check_central_store_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    let config_path = options.config_path();
    let config = read_json(root, &config_path)?;
    if options.self_test {
        let report = validate(
            root,
            &config_path,
            &config,
            self_test_files(&config_path, &config)?,
        );
        if !report.errors.is_empty() {
            for error in report.errors {
                eprintln!("central-store-contract self-test: {error}");
            }
            return Ok(1);
        }
        println!("central store contract self-test passed");
        return Ok(0);
    }

    let files = read_runtime_files(root, &config_path, &config)?;
    let report = validate(root, &config_path, &config, files);
    if !report.errors.is_empty() {
        for error in report.errors {
            eprintln!("central-store-contract: {error}");
        }
        return Ok(1);
    }
    println!(
        "central store contract passed: {} schema(s), {} required file(s), {} text check(s)",
        report.schema_count, report.required_file_count, report.text_check_count
    );
    Ok(0)
}

fn read_runtime_files(
    root: &Path,
    config_path: &str,
    config: &Value,
) -> RunnerResult<HashMap<String, String>> {
    let mut bootstrap = HashMap::new();
    bootstrap.insert(
        config_path.to_string(),
        serde_json::to_string(config).unwrap_or_default(),
    );
    bootstrap.insert(CONFIG_SCHEMA.to_string(), read_text(root, CONFIG_SCHEMA)?);
    if !validate_config(root, config_path, config, &bootstrap).is_empty() {
        return Ok(bootstrap);
    }

    let mut files = HashMap::new();
    for file in all_contract_files(config_path, config) {
        files.insert(file.clone(), read_text(root, &file)?);
    }
    Ok(files)
}

fn self_test_files(config_path: &str, config: &Value) -> RunnerResult<HashMap<String, String>> {
    let mut files = all_contract_files(config_path, config)
        .into_iter()
        .map(|file| (file, String::new()))
        .collect::<HashMap<_, _>>();
    files.insert(
        config_path.to_string(),
        serde_json::to_string(config).unwrap_or_default(),
    );
    files.insert(
        CONFIG_SCHEMA.to_string(),
        json!({
            "properties": { "schema_version": { "const": CONFIG_SCHEMA_VERSION } },
            "$defs": { "repoPath": { "pattern": REPO_PATH_PATTERN } }
        })
        .to_string(),
    );
    for contract in contracts(config) {
        let schema_version = field(contract, "schema_version").unwrap_or_default();
        files.insert(
            field(contract, "schema_path")
                .unwrap_or_default()
                .to_string(),
            json!({ "properties": { "schema_version": { "const": schema_version } } }).to_string(),
        );
        append_fixture(
            &mut files,
            field(contract, "backend_path").unwrap_or_default(),
            schema_version,
        );
        append_fixture(
            &mut files,
            field(contract, "frontend_types_path").unwrap_or_default(),
            schema_version,
        );
    }
    for check in text_checks(config) {
        append_fixture(
            &mut files,
            field(check, "file").unwrap_or_default(),
            field(check, "text").unwrap_or_default(),
        );
    }
    Ok(files)
}

fn validate(
    root: &Path,
    config_path: &str,
    config: &Value,
    files: HashMap<String, String>,
) -> Report {
    let mut errors = validate_config(root, config_path, config, &files);
    if !errors.is_empty() {
        return build_report(config, errors);
    }

    for file in all_contract_files(config_path, config) {
        if !files.contains_key(&file) {
            errors.push(format!("missing required file: {file}"));
        }
    }
    for contract in contracts(config) {
        validate_schema(contract, &files, &mut errors);
        validate_text_contains(
            field(contract, "backend_path").unwrap_or_default(),
            field(contract, "schema_version").unwrap_or_default(),
            "backend",
            &files,
            &mut errors,
        );
        validate_text_contains(
            field(contract, "frontend_types_path").unwrap_or_default(),
            field(contract, "schema_version").unwrap_or_default(),
            "frontend types",
            &files,
            &mut errors,
        );
    }
    for check in text_checks(config) {
        validate_text_contains(
            field(check, "file").unwrap_or_default(),
            field(check, "text").unwrap_or_default(),
            field(check, "label").unwrap_or_default(),
            &files,
            &mut errors,
        );
    }
    build_report(config, errors)
}

fn validate_config(
    root: &Path,
    config_path: &str,
    config: &Value,
    files: &HashMap<String, String>,
) -> Vec<String> {
    let mut errors = Vec::new();
    if field(config, "schema_version") != Some(CONFIG_SCHEMA_VERSION) {
        errors.push("unexpected central store contract config schema_version".to_string());
    }
    validate_config_shape(config, &mut errors);
    validate_config_invariants(config_path, config, &mut errors);
    validate_config_schema(files, &mut errors);
    for file in all_contract_files(config_path, config) {
        if repo_path(root, &file).is_err() {
            errors.push(format!("path must stay inside repository: {file}"));
        }
    }
    errors
}

fn validate_config_shape(config: &Value, errors: &mut Vec<String>) {
    require_array(config.get("contracts"), "contracts", errors);
    require_array(config.get("required_files"), "required_files", errors);
    require_array(config.get("text_checks"), "text_checks", errors);
    for contract in contracts(config) {
        for key in [
            "id",
            "schema_version",
            "schema_path",
            "backend_path",
            "frontend_types_path",
        ] {
            require_string(contract.get(key), &format!("contract.{key}"), errors);
        }
    }
    for check in text_checks(config) {
        for key in ["file", "text", "label"] {
            require_string(check.get(key), &format!("text_check.{key}"), errors);
        }
    }
}

fn validate_config_invariants(config_path: &str, config: &Value, errors: &mut Vec<String>) {
    report_duplicates(
        contracts(config)
            .iter()
            .filter_map(|item| field(item, "id"))
            .collect(),
        "contract id",
        errors,
    );
    report_duplicates(
        contracts(config)
            .iter()
            .filter_map(|item| field(item, "schema_path"))
            .collect(),
        "contract schema path",
        errors,
    );
    report_duplicates(
        text_checks(config)
            .iter()
            .map(|item| {
                format!(
                    "{}::{}",
                    field(item, "file").unwrap_or_default(),
                    field(item, "text").unwrap_or_default()
                )
            })
            .collect(),
        "text check",
        errors,
    );

    for path in all_contract_files(config_path, config) {
        validate_repo_relative_path(&path, errors);
    }
    let serialized = serde_json::to_string(config).unwrap_or_default();
    for (needle, label) in unsafe_needles() {
        if serialized.contains(needle) {
            errors.push(format!(
                "unsafe central store contract config text: {label}"
            ));
        }
    }
    if serialized.contains("DATABASE_URL=")
        && serialized.contains("://")
        && serialized.contains('@')
    {
        errors.push(
            "unsafe central store contract config text: inline DATABASE_URL secret".to_string(),
        );
    }
}

fn validate_config_schema(files: &HashMap<String, String>, errors: &mut Vec<String>) {
    match files
        .get(CONFIG_SCHEMA)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
    {
        Some(schema) => {
            if schema
                .pointer("/properties/schema_version/const")
                .and_then(Value::as_str)
                != Some(CONFIG_SCHEMA_VERSION)
            {
                errors.push("central store contract config schema const mismatch".to_string());
            }
            if schema
                .pointer("/$defs/repoPath/pattern")
                .and_then(Value::as_str)
                != Some(REPO_PATH_PATTERN)
            {
                errors.push(
                    "central store contract config schema must define repo-relative path pattern"
                        .to_string(),
                );
            }
        }
        None => errors.push(format!("{CONFIG_SCHEMA}: invalid json")),
    }
}

fn validate_schema(contract: &Value, files: &HashMap<String, String>, errors: &mut Vec<String>) {
    let schema_path = field(contract, "schema_path").unwrap_or_default();
    let expected = field(contract, "schema_version").unwrap_or_default();
    match files
        .get(schema_path)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
    {
        Some(schema) => {
            if schema
                .pointer("/properties/schema_version/const")
                .and_then(Value::as_str)
                != Some(expected)
            {
                errors.push(format!(
                    "{} schema const must be {expected}",
                    field(contract, "id").unwrap_or_default()
                ));
            }
        }
        None => errors.push(format!("{schema_path}: invalid json")),
    }
}

fn validate_text_contains(
    file: &str,
    needle: &str,
    label: &str,
    files: &HashMap<String, String>,
    errors: &mut Vec<String>,
) {
    if !files.get(file).is_some_and(|text| text.contains(needle)) {
        errors.push(format!("{label} missing {needle} in {file}"));
    }
}

fn all_contract_files(config_path: &str, config: &Value) -> Vec<String> {
    let mut values = vec![config_path.to_string(), CONFIG_SCHEMA.to_string()];
    for contract in contracts(config) {
        for key in ["schema_path", "backend_path", "frontend_types_path"] {
            if let Some(value) = field(contract, key) {
                values.push(value.to_string());
            }
        }
    }
    values.extend(string_array(config, "required_files"));
    for check in text_checks(config) {
        if let Some(file) = field(check, "file") {
            values.push(file.to_string());
        }
    }
    unique(values)
}

fn append_fixture(files: &mut HashMap<String, String>, file: &str, text: &str) {
    let current = files.entry(file.to_string()).or_default();
    if !file.ends_with(".json") {
        current.push('\n');
        current.push_str(text);
        return;
    }
    let parsed = serde_json::from_str::<Value>(current).unwrap_or_else(|_| json!({}));
    let mut object = parsed.as_object().cloned().unwrap_or_default();
    let previous = object
        .get("_self_test_text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    object.insert(
        "_self_test_text".to_string(),
        Value::String(
            [previous, text]
                .into_iter()
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    );
    *current = Value::Object(object).to_string();
}

fn build_report(config: &Value, errors: Vec<String>) -> Report {
    Report {
        errors,
        schema_count: contracts(config).len(),
        required_file_count: string_array(config, "required_files").len(),
        text_check_count: text_checks(config).len(),
    }
}

fn contracts(config: &Value) -> Vec<&Value> {
    config
        .get("contracts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect()
}

fn text_checks(config: &Value) -> Vec<&Value> {
    config
        .get("text_checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect()
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
}

fn require_array(value: Option<&Value>, label: &str, errors: &mut Vec<String>) {
    if !value
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        errors.push(format!("{label} must be a non-empty array"));
    }
}

fn require_string(value: Option<&Value>, label: &str, errors: &mut Vec<String>) {
    if !value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
    {
        errors.push(format!("{label} must be a non-empty string"));
    }
}

fn report_duplicates<T: AsRef<str>>(values: Vec<T>, label: &str, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for value in values {
        let value = value.as_ref();
        if value.trim().is_empty() {
            continue;
        }
        if !seen.insert(value.to_string()) {
            duplicates.insert(value.to_string());
        }
    }
    for value in duplicates {
        errors.push(format!("duplicate {label}: {value}"));
    }
}

fn validate_repo_relative_path(value: &str, errors: &mut Vec<String>) {
    if value.starts_with('/') || looks_like_windows_absolute(value) {
        errors.push(format!("path must be repository-relative: {value}"));
    }
    if value.split(['/', '\\']).any(|part| part == "..") {
        errors.push(format!(
            "path must not traverse parent directories: {value}"
        ));
    }
}

fn unsafe_needles() -> [(&'static str, &'static str); 2] {
    [
        ("ssh_password", "ssh password field"),
        ("ssh_pass", "ssh password field"),
    ]
}

fn looks_like_windows_absolute(value: &str) -> bool {
    value.len() > 2
        && value.as_bytes()[1] == b':'
        && matches!(value.as_bytes()[2], b'/' | b'\\')
        && value.as_bytes()[0].is_ascii_alphabetic()
}

fn unique(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    let path = repo_path(root, relative_path)?;
    fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn repo_path(root: &Path, relative_path: &str) -> RunnerResult<PathBuf> {
    if relative_path.is_empty()
        || relative_path.starts_with('/')
        || looks_like_windows_absolute(relative_path)
        || relative_path.split(['/', '\\']).any(|part| part == "..")
    {
        return Err(format!("path must stay inside repository: {relative_path}"));
    }
    Ok(root.join(relative_path))
}

struct Report {
    errors: Vec<String>,
    schema_count: usize,
    required_file_count: usize,
    text_check_count: usize,
}

struct Options {
    config: Option<String>,
    self_test: bool,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            config: None,
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
                "--config" => {
                    let value = args
                        .get(index + 1)
                        .and_then(|item| item.to_str())
                        .ok_or_else(|| "--config requires a value".to_string())?;
                    options.config = Some(value.to_string());
                    index += 2;
                }
                _ => return Err(format!("unknown argument {arg}")),
            }
        }
        Ok(options)
    }

    fn config_path(&self) -> String {
        self.config
            .clone()
            .or_else(|| env::var("CONFIG").ok())
            .unwrap_or_else(|| DEFAULT_CONFIG.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{append_fixture, looks_like_windows_absolute, report_duplicates};
    use std::collections::HashMap;

    #[test]
    fn windows_absolute_path_is_detected() {
        assert!(looks_like_windows_absolute("C:\\tmp\\secret.json"));
        assert!(looks_like_windows_absolute("D:/tmp/secret.json"));
    }

    #[test]
    fn duplicate_report_is_stable() {
        let mut errors = Vec::new();
        report_duplicates(vec!["a", "b", "a"], "contract id", &mut errors);
        assert_eq!(errors, vec!["duplicate contract id: a"]);
    }

    #[test]
    fn json_fixture_appends_text_anchor() {
        let mut files = HashMap::from([("x.json".to_string(), "{}".to_string())]);
        append_fixture(&mut files, "x.json", "needle");
        assert!(files["x.json"].contains("needle"));
    }
}
