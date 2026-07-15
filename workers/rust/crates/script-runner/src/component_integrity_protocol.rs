use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "deploy/installation-integrity-contract.json";
const INSTALL_SCHEMA: &str = "kyuubiki.installation-contract/v1";
const COMPONENT_PROTOCOL: &str = "kyuubiki.component-integrity/v1";
const REPORT_SCHEMA: &str = "kyuubiki.component-integrity-report/v1";
const REQUIRED_CENTRAL_COMPONENTS: &[&str] = &[
    "central.store.api",
    "central.store.contracts",
    "central.store.readiness",
];

pub(crate) fn run_check_component_integrity_protocol(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner check-component-integrity-protocol [--self-test] [--out <path>]"
        );
        return Ok(0);
    }
    let contract = if options.self_test {
        self_test_contract()
    } else {
        read_json(root, CONTRACT_PATH)?
    };
    let issues = validate_contract(&contract);
    let report = build_report(&contract, &issues);

    if let Some(out) = options.out {
        write_json(root, &out, &report)?;
    }

    if !issues.is_empty() {
        for issue in issues {
            eprintln!("component-integrity-protocol: {issue}");
        }
        return Ok(1);
    }

    let components = array_at(&contract, "/components").unwrap_or_default();
    let required = array_at(&contract, "/required_layout")
        .unwrap_or_default()
        .into_iter()
        .filter(|rule| bool_at(rule, "/required") == Some(true))
        .count();
    println!(
        "component integrity protocol passed: {} component(s), {} required layout rule(s)",
        components.len(),
        required
    );
    Ok(0)
}

#[derive(Default)]
struct Options {
    help: bool,
    self_test: bool,
    out: Option<String>,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        let mut index = 0usize;
        while index < args.len() {
            let arg = args[index].to_string_lossy();
            match arg.as_ref() {
                "--self-test" => options.self_test = true,
                "--out" => {
                    index += 1;
                    options.out = Some(
                        args.get(index)
                            .ok_or_else(|| "--out requires a value".to_string())?
                            .to_string_lossy()
                            .to_string(),
                    );
                }
                "--help" | "-h" => options.help = true,
                other => return Err(format!("unknown argument {other}")),
            }
            index += 1;
        }
        Ok(options)
    }
}

fn validate_contract(contract: &Value) -> Vec<String> {
    let mut issues = Vec::new();
    if string_at(contract, "/schema_version") != Some(INSTALL_SCHEMA) {
        issues.push(format!(
            "{CONTRACT_PATH}: schema_version must be {INSTALL_SCHEMA}"
        ));
    }
    let shipping = string_at(contract, "/shipping_version").unwrap_or_default();
    let global_protected = string_set_at(contract, "/protected_paths");
    let global_required = required_layout_paths(contract);
    let components = array_at(contract, "/components").unwrap_or_default();
    if components.is_empty() {
        issues.push("components must not be empty".to_string());
    }

    let mut seen_ids = BTreeSet::new();
    for component in &components {
        validate_component(
            &mut issues,
            component,
            &shipping,
            &global_protected,
            &mut seen_ids,
        );
    }
    validate_required_layout_coverage(&mut issues, &components, &global_required);
    validate_required_central_components(&mut issues, &seen_ids);
    issues
}

fn validate_component(
    issues: &mut Vec<String>,
    component: &Value,
    shipping: &str,
    global_protected: &BTreeSet<String>,
    seen_ids: &mut BTreeSet<String>,
) {
    let id = string_at(component, "/id").unwrap_or("<missing-id>");
    if !seen_ids.insert(id.to_string()) {
        issues.push(format!("component id is duplicated: {id}"));
    }
    for pointer in ["/id", "/kind", "/version", "/owner"] {
        if string_at(component, pointer).is_none_or(str::is_empty) {
            issues.push(format!("{id}: missing string field {pointer}"));
        }
    }
    if string_at(component, "/version") != Some(shipping) {
        issues.push(format!(
            "{id}: version must match shipping_version {shipping}"
        ));
    }

    let owned = string_vec_at(component, "/owned_paths");
    if owned.is_empty() {
        issues.push(format!("{id}: owned_paths must not be empty"));
    }
    for field in ["/required_paths", "/protected_paths", "/removable_patterns"] {
        if !is_string_array(component, field) {
            issues.push(format!("{id}: {field} must be a string array"));
        }
    }
    for required in string_vec_at(component, "/required_paths") {
        if !path_is_covered_by_any(&required, &owned) {
            issues.push(format!(
                "{id}: required path {required} is outside owned_paths"
            ));
        }
    }
    for protected in string_vec_at(component, "/protected_paths") {
        if !global_protected.contains(&protected) {
            issues.push(format!(
                "{id}: protected path {protected} is not in global protected_paths"
            ));
        }
    }
    for pattern in string_vec_at(component, "/removable_patterns") {
        if !pattern_is_scoped_to_owned_path(&pattern, &owned) {
            issues.push(format!(
                "{id}: removable pattern {pattern} is outside owned_paths"
            ));
        }
    }
    validate_visible_rules(issues, id, component);
    validate_checks(issues, id, component);
}

fn validate_visible_rules(issues: &mut Vec<String>, id: &str, component: &Value) {
    let Some(rules) = array_at(component, "/visible_rules") else {
        issues.push(format!("{id}: visible_rules must be an array"));
        return;
    };
    if rules.is_empty() {
        issues.push(format!(
            "{id}: visible_rules must make component behavior visible"
        ));
    }
    for rule in rules {
        for field in ["/label", "/value", "/description"] {
            if string_at(rule, field).is_none_or(str::is_empty) {
                issues.push(format!("{id}: visible rule missing {field}"));
            }
        }
        if bool_at(rule, "/editable").is_none() {
            issues.push(format!("{id}: visible rule editable must be boolean"));
        }
    }
}

fn validate_checks(issues: &mut Vec<String>, id: &str, component: &Value) {
    let Some(checks) = array_at(component, "/checks") else {
        issues.push(format!("{id}: checks must be an array"));
        return;
    };
    let kind = string_at(component, "/kind").unwrap_or_default();
    if kind.starts_with("central-") && checks.is_empty() {
        issues.push(format!(
            "{id}: central components must declare at least one check"
        ));
    }
    for check in checks {
        for field in ["/command", "/purpose"] {
            if string_at(check, field).is_none_or(str::is_empty) {
                issues.push(format!("{id}: check missing {field}"));
            }
        }
    }
}

fn validate_required_layout_coverage(
    issues: &mut Vec<String>,
    components: &[&Value],
    required_paths: &[String],
) {
    for required in required_paths {
        let covered = components.iter().any(|component| {
            path_is_covered_by_any(required, &string_vec_at(component, "/owned_paths"))
        });
        if !covered {
            issues.push(format!(
                "required layout path {required} is not owned by any component"
            ));
        }
    }
}

fn validate_required_central_components(issues: &mut Vec<String>, seen_ids: &BTreeSet<String>) {
    for id in REQUIRED_CENTRAL_COMPONENTS {
        if !seen_ids.contains(*id) {
            issues.push(format!("missing required central component {id}"));
        }
    }
}

fn build_report(contract: &Value, issues: &[String]) -> Value {
    let components = array_at(contract, "/components").unwrap_or_default();
    let required_paths = required_layout_paths(contract);
    let covered_required_path_count = required_paths
        .iter()
        .filter(|path| {
            components.iter().any(|component| {
                path_is_covered_by_any(path, &string_vec_at(component, "/owned_paths"))
            })
        })
        .count();
    let central_components = components
        .iter()
        .filter(|component| {
            string_at(component, "/kind").is_some_and(|kind| kind.starts_with("central-"))
        })
        .map(|component| component_summary(component))
        .collect::<Vec<_>>();
    let seen_ids = components
        .iter()
        .filter_map(|component| string_at(component, "/id").map(str::to_string))
        .collect::<BTreeSet<_>>();

    json!({
        "schema_version": REPORT_SCHEMA,
        "protocol_schema_version": COMPONENT_PROTOCOL,
        "install_contract_schema_version": string_at(contract, "/schema_version").unwrap_or_default(),
        "shipping_version": string_at(contract, "/shipping_version").unwrap_or_default(),
        "status": if issues.is_empty() { "ok" } else { "fail" },
        "component_count": components.len(),
        "required_layout_count": required_paths.len(),
        "covered_required_layout_count": covered_required_path_count,
        "central_components": central_components,
        "central_required_coverage": central_required_coverage(&seen_ids),
        "issues": issues,
    })
}

fn central_required_coverage(seen_ids: &BTreeSet<String>) -> Value {
    let required = REQUIRED_CENTRAL_COMPONENTS
        .iter()
        .map(|id| (*id).to_string())
        .collect::<Vec<_>>();
    let covered = required
        .iter()
        .filter(|id| seen_ids.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    let missing = required
        .iter()
        .filter(|id| !seen_ids.contains(*id))
        .cloned()
        .collect::<Vec<_>>();

    json!({
        "required": required,
        "covered": covered,
        "missing": missing,
        "required_count": REQUIRED_CENTRAL_COMPONENTS.len(),
        "covered_count": covered.len(),
        "complete": missing.is_empty()
    })
}

fn component_summary(component: &Value) -> Value {
    json!({
        "id": string_at(component, "/id").unwrap_or_default(),
        "kind": string_at(component, "/kind").unwrap_or_default(),
        "owner": string_at(component, "/owner").unwrap_or_default(),
        "version": string_at(component, "/version").unwrap_or_default(),
        "owned_paths": string_vec_at(component, "/owned_paths"),
        "required_paths": string_vec_at(component, "/required_paths"),
        "check_commands": array_at(component, "/checks")
            .unwrap_or_default()
            .into_iter()
            .filter_map(|check| string_at(check, "/command").map(str::to_string))
            .collect::<Vec<_>>(),
    })
}

fn required_layout_paths(contract: &Value) -> Vec<String> {
    array_at(contract, "/required_layout")
        .unwrap_or_default()
        .into_iter()
        .filter(|rule| bool_at(rule, "/required") == Some(true))
        .filter_map(|rule| string_at(rule, "/path").map(str::to_string))
        .collect()
}

fn path_is_covered_by_any(path: &str, owned_paths: &[String]) -> bool {
    owned_paths.iter().any(|owned| path_is_under(path, owned))
}

fn path_is_under(path: &str, owned: &str) -> bool {
    path == owned
        || owned == "."
        || path
            .strip_prefix(owned)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn pattern_is_scoped_to_owned_path(pattern: &str, owned_paths: &[String]) -> bool {
    let normalized = pattern.trim_start_matches("./");
    path_is_covered_by_any(normalized, owned_paths)
        || owned_paths.iter().any(|owned| {
            normalized.starts_with(&format!("{owned}/"))
                || normalized.starts_with(&format!("{owned}*"))
        })
}

fn string_set_at(value: &Value, pointer: &str) -> BTreeSet<String> {
    string_vec_at(value, pointer).into_iter().collect()
}

fn string_vec_at(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn is_string_array(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().all(Value::is_string))
}

fn array_at<'a>(value: &'a Value, pointer: &str) -> Option<Vec<&'a Value>> {
    value
        .pointer(pointer)?
        .as_array()
        .map(|items| items.iter().collect())
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn bool_at(value: &Value, pointer: &str) -> Option<bool> {
    value.pointer(pointer).and_then(Value::as_bool)
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn write_json(root: &Path, relative_path: &str, value: &Value) -> RunnerResult<()> {
    let path = repo_path(root, relative_path)?;
    fs::create_dir_all(
        path.parent()
            .ok_or_else(|| format!("path has no parent: {relative_path}"))?,
    )
    .map_err(|error| format!("failed to create parent for {relative_path}: {error}"))?;
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|error| format!("failed to render report: {error}"))?
        ),
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))
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

fn self_test_contract() -> Value {
    json!({
        "schema_version": INSTALL_SCHEMA,
        "shipping_version": "1.20.0",
        "required_layout": [
            { "label": "root", "path": ".", "required": true },
            { "label": "central api", "path": "apps/web/lib/kyuubiki_web/central_store.ex", "required": true }
        ],
        "protected_paths": [],
        "components": [
            component_fixture("workspace.root", "workspace", ["."], ["."], []),
            component_fixture(
                "central.store.api",
                "central-service",
                ["apps/web/lib/kyuubiki_web/central_store.ex"],
                ["apps/web/lib/kyuubiki_web/central_store.ex"],
                ["make check-central-store-contract"]
            ),
            component_fixture(
                "central.store.contracts",
                "central-contracts",
                ["config/architecture/central-store-contract.json"],
                ["config/architecture/central-store-contract.json"],
                ["make check-central-readiness-report"]
            ),
            component_fixture(
                "central.store.readiness",
                "central-readiness",
                ["workers/rust/crates/script-runner/src/central_readiness_report.rs"],
                ["workers/rust/crates/script-runner/src/central_readiness_report.rs"],
                ["make build-central-readiness-report"]
            )
        ]
    })
}

fn component_fixture<const N: usize, const M: usize, const C: usize>(
    id: &str,
    kind: &str,
    owned: [&str; N],
    required: [&str; M],
    checks: [&str; C],
) -> Value {
    json!({
        "id": id,
        "kind": kind,
        "version": "1.20.0",
        "owner": "kyuubiki",
        "owned_paths": owned.to_vec(),
        "required_paths": required.to_vec(),
        "protected_paths": [],
        "removable_patterns": [],
        "visible_rules": [
            {
                "label": "protocol",
                "value": COMPONENT_PROTOCOL,
                "editable": false,
                "description": "Component behavior is visible to integrity checks."
            }
        ],
        "checks": checks.iter().map(|command| json!({
            "command": command,
            "purpose": "self-test command"
        })).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_test_contract_is_valid() {
        assert!(validate_contract(&self_test_contract()).is_empty());
    }

    #[test]
    fn detects_missing_central_component() {
        let mut contract = self_test_contract();
        contract["components"] = json!([component_fixture(
            "workspace.root",
            "workspace",
            ["."],
            ["."],
            []
        )]);
        let issues = validate_contract(&contract);
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("central.store.api"))
        );
    }

    #[test]
    fn report_exposes_central_components() {
        let contract = self_test_contract();
        let report = build_report(&contract, &validate_contract(&contract));
        assert_eq!(string_at(&report, "/status"), Some("ok"));
        assert_eq!(
            report
                .pointer("/central_components")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(3)
        );
        assert_eq!(
            report
                .pointer("/central_required_coverage/complete")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            report
                .pointer("/central_required_coverage/covered_count")
                .and_then(Value::as_u64),
            Some(3)
        );
    }
}
