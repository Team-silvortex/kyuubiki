use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const TOPOLOGY_PATH: &str = "config/architecture/module-topology.json";
const SCHEMA_VERSION: &str = "kyuubiki.module-topology/v1";
const REQUIRED_LAYERS: &[&str] = &[
    "product_shell",
    "control_plane",
    "runtime_data_plane",
    "sdk",
    "contract",
    "verification",
];
const ALLOWED_PLAN_SCOPES: &[&str] = &["local", "integration", "benchmark", "remote", "release"];
const ALLOWED_SERVICE_SURFACE_KINDS: &[&str] = &[
    "control_api",
    "self_host_web",
    "runtime_adapter",
    "storage_api",
];
const ALLOWED_COMMAND_PREFIXES: &[&str] = &[
    "make ",
    "./scripts/kyuubiki ",
    "node ",
    "cd apps/frontend && npm run ",
    "cd apps/web && mix ",
    "cd workers/rust && cargo ",
];

pub(crate) fn run_check_module_topology(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("module topology self-test passed");
        return Ok(0);
    }
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner check-module-topology [--self-test]");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-module-topology only accepts --self-test".to_string());
    }

    let topology = read_json(root, TOPOLOGY_PATH)?;
    check_topology(root, &topology, TOPOLOGY_PATH)?;
    println!("module topology check passed");
    Ok(0)
}

fn check_topology(root: &Path, topology: &Value, context: &str) -> RunnerResult<()> {
    if string_at(topology, "/schema_version") != Some(SCHEMA_VERSION) {
        return Err(format!(
            "{context}: schema_version must be {SCHEMA_VERSION}"
        ));
    }
    require_non_empty_string(topology.get("version_line"), "version_line", context)?;

    let benchmark_lanes = object_keys(topology, "benchmark_lanes");
    let security_lanes = object_keys(topology, "security_lanes");
    if benchmark_lanes.is_empty() {
        return Err(format!("{context}: benchmark_lanes must not be empty"));
    }
    if security_lanes.is_empty() {
        return Err(format!("{context}: security_lanes must not be empty"));
    }
    if value_array(topology, "modules").is_empty() {
        return Err(format!("{context}: modules must not be empty"));
    }

    check_lane_test_plan(topology, &benchmark_lanes, &security_lanes, context)?;

    let mut modules_by_id = BTreeMap::new();
    let mut seen_paths = BTreeMap::new();
    let mut seen_layers = BTreeSet::new();

    for (index, module) in value_array(topology, "modules").into_iter().enumerate() {
        let module_context = format!("{context}#modules/{index}");
        let id = required_string(module, "id", &module_context)?.to_string();
        let layer = required_string(module, "layer", &module_context)?;
        require_non_empty_string(module.get("summary"), "summary", &module_context)?;
        let owned_paths = require_string_array(module, "owned_paths", &module_context, 1)?;
        require_string_array(module, "risk_tags", &module_context, 1)?;
        require_string_array(module, "depends_on", &module_context, 0)?;

        if modules_by_id.contains_key(&id) {
            return Err(format!("{module_context}: duplicate module id {id}"));
        }
        if !REQUIRED_LAYERS.contains(&layer) {
            return Err(format!("{module_context}: unknown layer {layer}"));
        }

        seen_layers.insert(layer.to_string());
        check_lane_references(module, &benchmark_lanes, "benchmark_lanes", &module_context)?;
        check_lane_references(module, &security_lanes, "security_lanes", &module_context)?;
        check_service_surfaces(module, &module_context)?;

        for owned_path in owned_paths {
            check_path_exists(root, &owned_path, &module_context)?;
            if let Some(owner) = seen_paths.insert(owned_path.clone(), id.clone()) {
                return Err(format!(
                    "{module_context}: owned path {owned_path} also belongs to {owner}"
                ));
            }
        }
        modules_by_id.insert(id, module.clone());
    }

    for layer in REQUIRED_LAYERS {
        if !seen_layers.contains(*layer) {
            return Err(format!("{context}: missing required layer {layer}"));
        }
    }

    for (module_id, module) in &modules_by_id {
        for dependency_id in string_array(module, "depends_on") {
            if dependency_id == *module_id {
                return Err(format!("{module_id}: module must not depend on itself"));
            }
            if !modules_by_id.contains_key(&dependency_id) {
                return Err(format!(
                    "{module_id}: dependency does not exist: {dependency_id}"
                ));
            }
        }
    }

    check_acyclic_dependency_graph(&modules_by_id)
}

fn check_lane_references(
    module: &Value,
    lanes: &BTreeSet<String>,
    field: &str,
    context: &str,
) -> RunnerResult<()> {
    for lane in require_string_array(module, field, context, 1)? {
        if !lanes.contains(&lane) {
            return Err(format!("{context}: unknown {field} entry: {lane}"));
        }
    }
    Ok(())
}

fn check_service_surfaces(module: &Value, context: &str) -> RunnerResult<()> {
    let Some(surfaces) = module.get("service_surfaces") else {
        return Ok(());
    };
    let Some(surfaces) = surfaces.as_array() else {
        return Err(format!(
            "{context}: service_surfaces must be a non-empty array when present"
        ));
    };
    if surfaces.is_empty() {
        return Err(format!(
            "{context}: service_surfaces must be a non-empty array when present"
        ));
    }

    let mut ids = BTreeSet::new();
    for (index, surface) in surfaces.iter().enumerate() {
        let surface_context = format!("{context}#service_surfaces/{index}");
        let id = required_string(surface, "id", &surface_context)?;
        let kind = required_string(surface, "kind", &surface_context)?;
        require_non_empty_string(surface.get("summary"), "summary", &surface_context)?;
        if !ids.insert(id.to_string()) {
            return Err(format!(
                "{surface_context}: duplicate service surface id {id}"
            ));
        }
        if !ALLOWED_SERVICE_SURFACE_KINDS.contains(&kind) {
            return Err(format!(
                "{surface_context}: unknown service surface kind {kind}"
            ));
        }
    }
    Ok(())
}

fn check_lane_test_plan(
    topology: &Value,
    benchmark_lanes: &BTreeSet<String>,
    security_lanes: &BTreeSet<String>,
    context: &str,
) -> RunnerResult<()> {
    let plan = topology
        .get("lane_test_plan")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{context}: lane_test_plan must be defined"))?;

    for (group, lanes) in [("benchmark", benchmark_lanes), ("security", security_lanes)] {
        let group_plan = plan
            .get(group)
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{context}: lane_test_plan.{group} must be an object"))?;

        for lane in lanes {
            let entries = group_plan
                .get(lane)
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    format!("{context}: lane_test_plan.{group}.{lane} must not be empty")
                })?;
            if entries.is_empty() {
                return Err(format!(
                    "{context}: lane_test_plan.{group}.{lane} must not be empty"
                ));
            }
            let mut ids = BTreeSet::new();
            for (index, entry) in entries.iter().enumerate() {
                let entry_context = format!("{context}#lane_test_plan/{group}/{lane}/{index}");
                let id = required_string(entry, "id", &entry_context)?;
                let command = required_string(entry, "command", &entry_context)?;
                let scope = required_string(entry, "scope", &entry_context)?;
                if !ids.insert(id.to_string()) {
                    return Err(format!("{entry_context}: duplicate test plan id {id}"));
                }
                if !ALLOWED_PLAN_SCOPES.contains(&scope) {
                    return Err(format!("{entry_context}: unknown scope {scope}"));
                }
                check_plan_command(command, &entry_context)?;
            }
        }

        for lane in group_plan.keys() {
            if !lanes.contains(lane) {
                return Err(format!(
                    "{context}: lane_test_plan.{group}.{lane} references unknown lane"
                ));
            }
        }
    }

    require_plan_entry(
        topology.pointer("/lane_test_plan/benchmark/control_plane"),
        "central-db-readiness",
        &format!("{context}: lane_test_plan.benchmark.control_plane"),
    )?;
    require_plan_entry(
        topology.pointer("/lane_test_plan/security/data_contract"),
        "central-store-contract",
        &format!("{context}: lane_test_plan.security.data_contract"),
    )?;
    require_plan_entry(
        topology.pointer("/lane_test_plan/security/data_contract"),
        "central-db-readiness",
        &format!("{context}: lane_test_plan.security.data_contract"),
    )
}

fn require_plan_entry(
    entries: Option<&Value>,
    required_id: &str,
    context: &str,
) -> RunnerResult<()> {
    let Some(entries) = entries else {
        return Ok(());
    };
    let has_entry = entries.as_array().is_some_and(|entries| {
        entries
            .iter()
            .any(|entry| string_field(entry, "id") == Some(required_id))
    });
    if !has_entry {
        return Err(format!(
            "{context}: missing required test plan id {required_id}"
        ));
    }
    Ok(())
}

fn check_plan_command(command: &str, context: &str) -> RunnerResult<()> {
    if !ALLOWED_COMMAND_PREFIXES
        .iter()
        .any(|prefix| command.starts_with(prefix))
    {
        return Err(format!("{context}: unsupported command prefix: {command}"));
    }
    if command.contains("..") || command.contains(';') || command.contains("&& rm ") {
        return Err(format!(
            "{context}: command must not contain path traversal or destructive shell chaining"
        ));
    }
    Ok(())
}

fn check_acyclic_dependency_graph(modules: &BTreeMap<String, Value>) -> RunnerResult<()> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for module_id in modules.keys() {
        visit_module(
            module_id,
            modules,
            &mut visiting,
            &mut visited,
            &mut Vec::new(),
        )?;
    }
    Ok(())
}

fn visit_module(
    module_id: &str,
    modules: &BTreeMap<String, Value>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    chain: &mut Vec<String>,
) -> RunnerResult<()> {
    if visited.contains(module_id) {
        return Ok(());
    }
    if visiting.contains(module_id) {
        chain.push(module_id.to_string());
        return Err(format!("dependency cycle detected: {}", chain.join(" -> ")));
    }
    visiting.insert(module_id.to_string());
    chain.push(module_id.to_string());
    if let Some(module) = modules.get(module_id) {
        for dependency in string_array(module, "depends_on") {
            visit_module(&dependency, modules, visiting, visited, chain)?;
        }
    }
    chain.pop();
    visiting.remove(module_id);
    visited.insert(module_id.to_string());
    Ok(())
}

fn check_path_exists(root: &Path, relative_path: &str, context: &str) -> RunnerResult<()> {
    if relative_path.starts_with('/') || relative_path.split('/').any(|part| part == "..") {
        return Err(format!(
            "{context}: owned path must be repository-relative: {relative_path}"
        ));
    }
    if !root.join(relative_path).exists() {
        return Err(format!(
            "{context}: owned path does not exist: {relative_path}"
        ));
    }
    Ok(())
}

fn object_keys(topology: &Value, key: &str) -> BTreeSet<String> {
    topology
        .get(key)
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|object| object.keys())
        .cloned()
        .collect()
}

fn require_non_empty_string(value: Option<&Value>, field: &str, context: &str) -> RunnerResult<()> {
    let Some(value) = value.and_then(Value::as_str) else {
        return Err(format!("{context}: {field} must be a non-empty string"));
    };
    if value.trim().is_empty() {
        return Err(format!("{context}: {field} must be a non-empty string"));
    }
    Ok(())
}

fn required_string<'a>(value: &'a Value, field: &str, context: &str) -> RunnerResult<&'a str> {
    let Some(value) = value.get(field).and_then(Value::as_str) else {
        return Err(format!("{context}: {field} must be a non-empty string"));
    };
    if value.trim().is_empty() {
        return Err(format!("{context}: {field} must be a non-empty string"));
    }
    Ok(value)
}

fn require_string_array(
    value: &Value,
    field: &str,
    context: &str,
    min_length: usize,
) -> RunnerResult<Vec<String>> {
    let entries = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context}: {field} must contain at least {min_length} item(s)"))?;
    if entries.len() < min_length {
        return Err(format!(
            "{context}: {field} must contain at least {min_length} item(s)"
        ));
    }
    let mut strings = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(entry) = entry.as_str() else {
            return Err(format!(
                "{context}: {field}[{index}] must be a non-empty string"
            ));
        };
        if entry.trim().is_empty() {
            return Err(format!(
                "{context}: {field}[{index}] must be a non-empty string"
            ));
        }
        strings.push(entry.to_string());
    }
    Ok(strings)
}

fn value_array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
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
        .map(str::to_string)
        .collect()
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let mut api = module_fixture("api", "control_plane", "config", &["contracts"]);
    api["service_surfaces"] = json!([{
        "id": "central-web-service",
        "kind": "self_host_web",
        "summary": "self-hosted web service surface"
    }]);
    let modules = vec![
        module_fixture("contracts", "contract", "docs", &[]),
        module_fixture("hub", "product_shell", "apps", &["contracts"]),
        api,
        module_fixture("runtime", "runtime_data_plane", "workers", &["contracts"]),
        module_fixture("sdk", "sdk", "sdks", &["contracts"]),
        module_fixture("verification", "verification", "scripts", &["contracts"]),
    ];
    let mut sample = json!({
        "schema_version": SCHEMA_VERSION,
        "version_line": "moxi test",
        "benchmark_lanes": { "ui_startup": "test lane" },
        "security_lanes": { "ui_boundary": "test lane" },
        "lane_test_plan": {
            "benchmark": {
                "ui_startup": [{ "id": "smoke", "command": "make smoke", "scope": "local" }]
            },
            "security": {
                "ui_boundary": [{ "id": "audit", "command": "./scripts/kyuubiki audit-local-paths", "scope": "local" }]
            }
        },
        "modules": modules
    });

    check_topology(root, &sample, "self-test")?;
    sample["modules"][1]["depends_on"] = json!(["missing"]);
    assert_error_contains(root, &sample, "dependency does not exist")?;
    sample["modules"][1]["depends_on"] = json!(["contracts"]);
    sample["modules"][0]["depends_on"] = json!(["hub"]);
    assert_error_contains(root, &sample, "dependency cycle")?;
    sample["modules"][0]["depends_on"] = json!([]);
    sample["modules"][2]["service_surfaces"] =
        json!([{ "id": "bad", "kind": "module", "summary": "wrong" }]);
    assert_error_contains(root, &sample, "unknown service surface kind")
}

fn module_fixture(id: &str, layer: &str, owned_path: &str, depends_on: &[&str]) -> Value {
    json!({
        "id": id,
        "layer": layer,
        "summary": id,
        "owned_paths": [owned_path],
        "depends_on": depends_on,
        "benchmark_lanes": ["ui_startup"],
        "security_lanes": ["ui_boundary"],
        "risk_tags": ["test"]
    })
}

fn assert_error_contains(root: &Path, topology: &Value, expected: &str) -> RunnerResult<()> {
    match check_topology(root, topology, "self-test") {
        Ok(()) => Err(format!("self-test expected failure containing {expected}")),
        Err(error) if error.contains(expected) => Ok(()),
        Err(error) => Err(format!(
            "self-test expected failure containing {expected}, got {error}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{check_plan_command, require_string_array};
    use serde_json::json;

    #[test]
    fn plan_command_rejects_destructive_shell_chaining() {
        assert!(check_plan_command("make test && rm -rf tmp", "ctx").is_err());
    }

    #[test]
    fn string_array_accepts_empty_when_min_is_zero() {
        let value = json!({ "depends_on": [] });
        assert!(require_string_array(&value, "depends_on", "ctx", 0).is_ok());
    }
}
