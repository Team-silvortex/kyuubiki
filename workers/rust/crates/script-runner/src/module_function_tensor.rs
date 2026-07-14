use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod markdown;
mod self_test;

type RunnerResult<T> = Result<T, String>;

pub(super) const TENSOR_PATH: &str = "config/architecture/module-function-coverage-tensor.json";
pub(super) const TOPOLOGY_PATH: &str = "config/architecture/module-topology.json";
pub(super) const MATRIX_PATH: &str = "config/architecture/module-function-coverage-matrix.json";
const DEFAULT_OUT: &str = "tmp/module-function-coverage-tensor.json";
pub(super) const SCHEMA_VERSION: &str = "kyuubiki.module-function-coverage-tensor/v1";
const REPORT_SCHEMA_VERSION: &str = "kyuubiki.module-function-coverage-tensor-report/v1";
const ALLOWED_STATUS: &[&str] = &["covered", "partial", "planned", "not_applicable"];
const PARADIGM_ORDER: &[&str] = &[
    "product_surface",
    "runtime_api",
    "solver_execution",
    "workflow_composition",
    "validation",
    "benchmark",
    "security",
    "persistence_provenance",
    "deployment_update",
    "sdk_headless",
];
const DEPTH_AXIS_ORDER: &[&str] = &[
    "required",
    "status",
    "benchmark_evidence",
    "security_evidence",
    "contract_evidence",
    "gap",
];
const GAP_ORDER: &[&str] = &[
    "required_gap",
    "weak",
    "weak_evidence",
    "planned",
    "watch",
    "missing",
    "ok",
    "not_applicable",
];

pub(crate) fn run_check_module_function_tensor(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    if options.self_test {
        self_test::run_self_test()?;
        println!("module function coverage tensor self-test passed");
        return Ok(0);
    }

    let tensor = read_json(root, TENSOR_PATH)?;
    let topology = read_json(root, TOPOLOGY_PATH)?;
    let matrix = read_json(root, MATRIX_PATH)?;
    validate_tensor_config(root, &tensor, &topology, &matrix)?;
    let report = build_tensor_report(&tensor, &topology, &matrix);
    write_report(root, &report, &options.out)?;
    let ok = report.get("ok").and_then(Value::as_bool).unwrap_or(false);
    println!(
        "module function coverage tensor {}: {} module(s), {} paradigm(s), {} gap(s)",
        if ok { "passed" } else { "has gaps" },
        report
            .pointer("/axes/modules")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        report
            .pointer("/axes/paradigms")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        report.get("gap_count").and_then(Value::as_u64).unwrap_or(0)
    );
    Ok(if ok { 0 } else { 1 })
}

fn validate_tensor_config(
    root: &Path,
    tensor: &Value,
    topology: &Value,
    matrix: &Value,
) -> RunnerResult<()> {
    if string_at(tensor, "/schema_version") != Some(SCHEMA_VERSION) {
        return Err(format!("schema_version must be {SCHEMA_VERSION}"));
    }
    if string_at(tensor, "/topology") != Some(TOPOLOGY_PATH) {
        return Err(format!("topology must be {TOPOLOGY_PATH}"));
    }
    if string_at(tensor, "/matrix") != Some(MATRIX_PATH) {
        return Err(format!("matrix must be {MATRIX_PATH}"));
    }
    let depth_axes = tensor
        .get("depth_axes")
        .and_then(Value::as_object)
        .ok_or_else(|| "depth_axes must not be empty".to_string())?;
    if depth_axes.is_empty() {
        return Err("depth_axes must not be empty".to_string());
    }
    for (axis, description) in depth_axes {
        if !description.as_str().is_some_and(|text| !text.is_empty()) {
            return Err(format!("depth axis {axis} must describe itself"));
        }
    }

    let paradigms = object_keys(matrix, "paradigms");
    let benchmark_lanes = object_keys(topology, "benchmark_lanes");
    let security_lanes = object_keys(topology, "security_lanes");
    for paradigm in &paradigms {
        let mapping = tensor
            .pointer(&format!("/paradigm_lanes/{paradigm}"))
            .ok_or_else(|| format!("missing paradigm lane mapping for {paradigm}"))?;
        for lane in string_array(mapping, "benchmark") {
            if !benchmark_lanes.contains(&lane) {
                return Err(format!("{paradigm}: unknown benchmark lane {lane}"));
            }
        }
        for lane in string_array(mapping, "security") {
            if !security_lanes.contains(&lane) {
                return Err(format!("{paradigm}: unknown security lane {lane}"));
            }
        }
    }
    for paradigm in object_keys(tensor, "paradigm_lanes") {
        if !paradigms.contains(&paradigm) {
            return Err(format!("tensor maps unknown paradigm {paradigm}"));
        }
    }
    for (paradigm, entries) in tensor
        .get("paradigm_contract_evidence")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if !paradigms.contains(paradigm) {
            return Err(format!("tensor evidence maps unknown paradigm {paradigm}"));
        }
        validate_contract_evidence_entries(root, entries, paradigm)?;
    }
    Ok(())
}

fn validate_contract_evidence_entries(
    root: &Path,
    entries: &Value,
    paradigm: &str,
) -> RunnerResult<()> {
    let entries = entries
        .as_array()
        .ok_or_else(|| format!("{paradigm}.contract_evidence must be an array"))?;
    let mut seen = BTreeSet::new();
    for (index, entry) in entries.iter().enumerate() {
        let id = string_field(entry, "id")
            .ok_or_else(|| format!("{paradigm}.contract_evidence[{index}] must have id"))?;
        if !seen.insert(id.to_string()) {
            return Err(format!("{paradigm}.contract_evidence duplicate id {id}"));
        }
        let mut combined = String::new();
        for file in string_array(entry, "files") {
            combined.push('\n');
            combined.push_str(&read_text(root, &file)?);
        }
        for required_text in string_array(entry, "required_text") {
            if !combined.contains(&required_text) {
                return Err(format!("{id}: evidence bundle missing {required_text}"));
            }
        }
    }
    Ok(())
}

fn build_tensor_report(tensor: &Value, topology: &Value, matrix: &Value) -> Value {
    let paradigms = ordered_keys(matrix, "paradigms", PARADIGM_ORDER);
    let required_by_module = matrix.get("required_by_module").and_then(Value::as_object);
    let mut module_summary = serde_json::Map::new();
    let mut paradigm_summary = serde_json::Map::new();
    for paradigm in &paradigms {
        paradigm_summary.insert(paradigm.clone(), empty_counts());
    }
    let mut cells = serde_json::Map::new();
    let mut gaps = Vec::new();

    for module in modules(topology) {
        let module_id = string_field(module, "id").unwrap_or_default();
        let required_set: BTreeSet<String> = required_by_module
            .and_then(|map| map.get(module_id))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        let mut module_cells = serde_json::Map::new();
        let mut module_counts = empty_counts_map();
        for paradigm in &paradigms {
            let status = matrix
                .pointer(&format!("/cells/{module_id}/{paradigm}"))
                .and_then(Value::as_str)
                .unwrap_or("not_applicable");
            let required = required_set.contains(paradigm);
            let mapping = tensor
                .pointer(&format!("/paradigm_lanes/{paradigm}"))
                .unwrap_or(&Value::Null);
            let contract_evidence = contract_evidence_for(tensor, paradigm);
            let benchmark_lanes = intersect(
                &string_array(module, "benchmark_lanes"),
                &string_array(mapping, "benchmark"),
            );
            let security_lanes = intersect(
                &string_array(module, "security_lanes"),
                &string_array(mapping, "security"),
            );
            let benchmark_tests = get_lane_tests(topology, "benchmark", &benchmark_lanes);
            let security_tests = get_lane_tests(topology, "security", &security_lanes);
            let evidence_depth = json!({
                "benchmark_lane_count": benchmark_lanes.len(),
                "security_lane_count": security_lanes.len(),
                "contract_evidence_count": contract_evidence.len(),
                "test_command_count": benchmark_tests.len() + security_tests.len()
            });
            let gap = derive_evidence_aware_gap(status, required, &evidence_depth);
            increment(&mut module_counts, gap);
            if let Some(counts) = paradigm_summary.get_mut(paradigm) {
                increment_value_counts(counts, gap);
            }
            if gap != "ok" && gap != "not_applicable" {
                gaps.push(json!({
                    "gap": gap,
                    "module_id": module_id,
                    "paradigm": paradigm,
                    "status": status,
                    "required": required,
                    "benchmark_lanes": benchmark_lanes,
                    "security_lanes": security_lanes
                }));
            }
            module_cells.insert(
                paradigm.clone(),
                json!({
                    "status": status,
                    "required": required,
                    "gap": gap,
                    "benchmark_lanes": benchmark_lanes,
                    "security_lanes": security_lanes,
                    "benchmark_tests": benchmark_tests,
                    "security_tests": security_tests,
                    "contract_evidence": contract_evidence,
                    "evidence_depth": evidence_depth
                }),
            );
        }
        cells.insert(module_id.to_string(), Value::Object(module_cells));
        module_summary.insert(
            module_id.to_string(),
            json!({
                "layer": string_field(module, "layer").unwrap_or_default(),
                "counts": module_counts
            }),
        );
    }

    gaps.sort_by(|left, right| gap_sort_key(left).cmp(&gap_sort_key(right)));
    let blocking_gap_count = gaps
        .iter()
        .filter(|gap| matches!(string_field(gap, "gap"), Some("required_gap" | "missing")))
        .count();
    json!({
        "schema_version": REPORT_SCHEMA_VERSION,
        "source": TENSOR_PATH,
        "topology": TOPOLOGY_PATH,
        "matrix": MATRIX_PATH,
        "ok": blocking_gap_count == 0,
        "axes": {
            "modules": modules(topology).into_iter().filter_map(|module| string_field(module, "id")).collect::<Vec<_>>(),
            "paradigms": paradigms,
            "depth": ordered_keys(tensor, "depth_axes", DEPTH_AXIS_ORDER)
        },
        "module_summary": module_summary,
        "paradigm_summary": paradigm_summary,
        "paradigm_contract_evidence": tensor.get("paradigm_contract_evidence").cloned().unwrap_or_else(|| json!({})),
        "gap_count": gaps.len(),
        "blocking_gap_count": blocking_gap_count,
        "gaps": gaps,
        "cells": cells
    })
}

fn write_report(root: &Path, report: &Value, out_path: &str) -> RunnerResult<()> {
    let absolute = repo_path(root, out_path)?;
    let parent = absolute
        .parent()
        .ok_or_else(|| format!("output path has no parent: {out_path}"))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    fs::write(
        &absolute,
        format!(
            "{}\n",
            serde_json::to_string_pretty(report)
                .map_err(|error| format!("failed to render report json: {error}"))?
        ),
    )
    .map_err(|error| format!("failed to write {}: {error}", absolute.display()))?;
    fs::write(
        parent.join("module-function-coverage-tensor.md"),
        markdown::render_markdown(report),
    )
    .map_err(|error| format!("failed to write markdown report: {error}"))?;
    Ok(())
}

fn derive_gap(status: &str, required: bool) -> &'static str {
    if !ALLOWED_STATUS.contains(&status) {
        return if required { "required_gap" } else { "missing" };
    }
    match (status, required) {
        ("covered", _) => "ok",
        ("partial", true) => "weak",
        ("partial", false) => "watch",
        ("planned", true) => "required_gap",
        ("planned", false) => "planned",
        ("not_applicable", true) => "required_gap",
        ("not_applicable", false) => "not_applicable",
        _ => "missing",
    }
}

fn derive_evidence_aware_gap(status: &str, required: bool, evidence_depth: &Value) -> &'static str {
    let gap = derive_gap(status, required);
    if gap != "ok" || !required {
        return gap;
    }
    let test_count = evidence_depth
        .get("test_command_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let contract_count = evidence_depth
        .get("contract_evidence_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if test_count > 0 || contract_count > 0 {
        "ok"
    } else {
        "weak_evidence"
    }
}

fn get_lane_tests(topology: &Value, lane_kind: &str, lanes: &[String]) -> Vec<Value> {
    let mut tests = Vec::new();
    for lane in lanes {
        let pointer = format!("/lane_test_plan/{lane_kind}/{lane}");
        for test in topology
            .pointer(&pointer)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            tests.push(json!({
                "lane": lane,
                "id": string_field(test, "id").unwrap_or_default(),
                "command": string_field(test, "command").unwrap_or_default(),
                "scope": string_field(test, "scope").unwrap_or_default()
            }));
        }
    }
    tests
}

fn contract_evidence_for(tensor: &Value, paradigm: &str) -> Vec<Value> {
    tensor
        .pointer(&format!("/paradigm_contract_evidence/{paradigm}"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|entry| {
            json!({
                "id": string_field(entry, "id").unwrap_or_default(),
                "files": string_array(entry, "files"),
                "required_text": string_array(entry, "required_text")
            })
        })
        .collect()
}

fn empty_counts() -> Value {
    Value::Object(empty_counts_map())
}

fn empty_counts_map() -> serde_json::Map<String, Value> {
    [
        "ok",
        "weak",
        "watch",
        "planned",
        "weak_evidence",
        "required_gap",
        "missing",
        "not_applicable",
    ]
    .into_iter()
    .map(|key| (key.to_string(), json!(0)))
    .collect()
}

fn increment(counts: &mut serde_json::Map<String, Value>, key: &str) {
    let next = counts.get(key).and_then(Value::as_u64).unwrap_or(0) + 1;
    counts.insert(key.to_string(), json!(next));
}

fn increment_value_counts(counts: &mut Value, key: &str) {
    if let Some(object) = counts.as_object_mut() {
        increment(object, key);
    }
}

fn gap_sort_key(value: &Value) -> (usize, String) {
    let gap = string_field(value, "gap").unwrap_or_default();
    let order = GAP_ORDER.iter().position(|item| *item == gap).unwrap_or(99);
    let label = format!(
        "{}/{}",
        string_field(value, "module_id").unwrap_or_default(),
        string_field(value, "paradigm").unwrap_or_default()
    );
    (order, label)
}

fn intersect(left: &[String], right: &[String]) -> Vec<String> {
    let right: BTreeSet<_> = right.iter().collect();
    left.iter()
        .filter(|item| right.contains(item))
        .cloned()
        .collect()
}

fn object_keys(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|object| object.keys())
        .cloned()
        .collect()
}

fn ordered_keys(value: &Value, key: &str, preferred_order: &[&str]) -> Vec<String> {
    let keys = object_keys(value, key);
    let key_set: BTreeSet<_> = keys.iter().map(String::as_str).collect();
    let mut ordered = preferred_order
        .iter()
        .filter(|item| key_set.contains(**item))
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();
    ordered.extend(
        keys.into_iter()
            .filter(|item| !preferred_order.contains(&item.as_str())),
    );
    ordered
}

fn object_entries<'a>(value: &'a Value, key: &str) -> Vec<(&'a String, &'a Value)> {
    value
        .get(key)
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|object| object.iter())
        .collect()
}

fn modules(topology: &Value) -> Vec<&Value> {
    topology
        .get("modules")
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

fn repo_path(root: &Path, relative_path: &str) -> RunnerResult<PathBuf> {
    if relative_path.is_empty()
        || relative_path.starts_with('/')
        || relative_path.split('/').any(|part| part == "..")
    {
        return Err(format!("path must stay inside repository: {relative_path}"));
    }
    Ok(root.join(relative_path))
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    let path = repo_path(root, relative_path)?;
    fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

#[derive(Debug)]
struct Options {
    out: String,
    self_test: bool,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            out: DEFAULT_OUT.to_string(),
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
                "--out" => {
                    let next = args
                        .get(index + 1)
                        .and_then(|value| value.to_str())
                        .ok_or_else(|| "--out requires a value".to_string())?;
                    options.out = next.to_string();
                    index += 2;
                }
                _ => return Err(format!("unknown argument {arg}")),
            }
        }
        Ok(options)
    }
}
