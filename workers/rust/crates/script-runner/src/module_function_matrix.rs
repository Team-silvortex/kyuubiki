use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const MATRIX_PATH: &str = "config/architecture/module-function-coverage-matrix.json";
const TOPOLOGY_PATH: &str = "config/architecture/module-topology.json";
const DEFAULT_OUT: &str = "tmp/module-function-matrix-report.json";
const SCHEMA_VERSION: &str = "kyuubiki.module-function-coverage-matrix/v1";
const ALLOWED_STATUS: &[&str] = &["covered", "partial", "planned", "not_applicable"];
const BLOCKING_STATUS: &[&str] = &["planned", "not_applicable"];
const PREFERRED_PARADIGM_ORDER: &[&str] = &[
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

pub(crate) fn run_check_module_function_matrix(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    if options.self_test {
        run_self_test()?;
        println!("module function matrix self-test passed");
        return Ok(0);
    }

    let matrix = read_json(root, MATRIX_PATH)?;
    let topology = read_json(root, TOPOLOGY_PATH)?;
    let findings = validate_matrix(&matrix, &topology)?;
    let report = build_report(&matrix, &topology, &findings)?;
    write_report(root, &report, &matrix, &options.out)?;

    let ok = !findings.iter().any(|finding| {
        finding
            .get("severity")
            .and_then(Value::as_str)
            .is_some_and(|severity| severity == "fail")
    });
    println!(
        "module function matrix {}: {} module(s), {} paradigm(s)",
        if ok { "passed" } else { "failed" },
        report
            .get("module_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        report
            .get("paradigm_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    Ok(if ok { 0 } else { 1 })
}

fn validate_matrix(matrix: &Value, topology: &Value) -> RunnerResult<Vec<Value>> {
    if string_at(matrix, "/schema_version") != Some(SCHEMA_VERSION) {
        return Err(format!("schema_version must be {SCHEMA_VERSION}"));
    }
    if string_at(matrix, "/topology") != Some(TOPOLOGY_PATH) {
        return Err(format!("topology must be {TOPOLOGY_PATH}"));
    }
    let paradigms = object_keys(matrix, "paradigms");
    if paradigms.is_empty() {
        return Err("paradigms must not be empty".to_string());
    }

    let module_ids: BTreeSet<String> = modules(topology)
        .into_iter()
        .filter_map(|module| string_field(module, "id").map(str::to_string))
        .collect();
    let matrix_module_ids = object_keys(matrix, "cells");
    for module_id in &module_ids {
        if !matrix_module_ids.contains(module_id) {
            return Err(format!("missing matrix row for module {module_id}"));
        }
    }
    for module_id in &matrix_module_ids {
        if !module_ids.contains(module_id) {
            return Err(format!("matrix row references unknown module {module_id}"));
        }
    }

    let mut findings = Vec::new();
    let required_by_module = matrix
        .get("required_by_module")
        .and_then(Value::as_object)
        .ok_or_else(|| "required_by_module must be an object".to_string())?;
    for (module_id, required_paradigms) in required_by_module {
        if !module_ids.contains(module_id) {
            return Err(format!(
                "required_by_module references unknown module {module_id}"
            ));
        }
        let required_paradigms = required_paradigms
            .as_array()
            .ok_or_else(|| format!("{module_id}: required paradigms must be an array"))?;
        for paradigm_value in required_paradigms {
            let paradigm = paradigm_value
                .as_str()
                .ok_or_else(|| format!("{module_id}: required paradigm must be a string"))?;
            if !paradigms.contains(paradigm) {
                return Err(format!("{module_id}: unknown required paradigm {paradigm}"));
            }
            let status = matrix
                .pointer(&format!("/cells/{module_id}/{paradigm}"))
                .and_then(Value::as_str);
            match status {
                Some(status) if ALLOWED_STATUS.contains(&status) => {
                    if BLOCKING_STATUS.contains(&status) {
                        findings.push(finding("warn", module_id, paradigm, status));
                    }
                }
                Some(status) => findings.push(finding("fail", module_id, paradigm, status)),
                None => findings.push(finding("fail", module_id, paradigm, "missing")),
            }
        }
    }

    let cells = matrix
        .get("cells")
        .and_then(Value::as_object)
        .ok_or_else(|| "cells must be an object".to_string())?;
    for (module_id, cells) in cells {
        let Some(cells) = cells.as_object() else {
            return Err(format!("{module_id}: cells row must be an object"));
        };
        for (paradigm, status) in cells {
            if !paradigms.contains(paradigm) {
                return Err(format!("{module_id}: unknown paradigm {paradigm}"));
            }
            let Some(status) = status.as_str() else {
                return Err(format!("{module_id}/{paradigm}: status must be a string"));
            };
            if !ALLOWED_STATUS.contains(&status) {
                return Err(format!("{module_id}/{paradigm}: unknown status {status}"));
            }
        }
    }

    Ok(findings)
}

fn build_report(matrix: &Value, topology: &Value, findings: &[Value]) -> RunnerResult<Value> {
    let paradigms = ordered_paradigms(matrix);
    let mut rows = Vec::new();
    for module in modules(topology) {
        let module_id = string_field(module, "id").unwrap_or_default();
        let layer = string_field(module, "layer").unwrap_or_default();
        let mut cells = BTreeMap::new();
        for paradigm in &paradigms {
            let status = matrix
                .pointer(&format!("/cells/{module_id}/{paradigm}"))
                .and_then(Value::as_str)
                .unwrap_or("not_applicable");
            cells.insert(paradigm.clone(), Value::String(status.to_string()));
        }
        let covered = count_status(&cells, "covered");
        let partial = count_status(&cells, "partial");
        let planned = count_status(&cells, "planned");
        rows.push(json!({
            "module_id": module_id,
            "layer": layer,
            "cells": cells,
            "covered": covered,
            "partial": partial,
            "planned": planned
        }));
    }

    Ok(json!({
        "schema_version": "kyuubiki.module-function-matrix-report/v1",
        "source": MATRIX_PATH,
        "topology": TOPOLOGY_PATH,
        "ok": !findings.iter().any(|finding| string_field(finding, "severity") == Some("fail")),
        "paradigm_count": paradigms.len(),
        "module_count": rows.len(),
        "findings": findings,
        "rows": rows
    }))
}

fn write_report(root: &Path, report: &Value, matrix: &Value, out_path: &str) -> RunnerResult<()> {
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
        parent.join("module-function-matrix.md"),
        render_markdown(report, matrix),
    )
    .map_err(|error| format!("failed to write markdown report: {error}"))?;
    Ok(())
}

fn render_markdown(report: &Value, matrix: &Value) -> String {
    let paradigms = ordered_paradigms(matrix);
    let mut lines = vec![
        "# Module Function Coverage Matrix".to_string(),
        String::new(),
        format!("- Source: `{MATRIX_PATH}`"),
        format!(
            "- Modules: `{}`",
            report
                .get("module_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Paradigms: `{}`",
            report
                .get("paradigm_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Status: `{}`",
            if report.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                "pass"
            } else {
                "fail"
            }
        ),
        String::new(),
        "| Module | Layer | Covered | Partial | Planned |".to_string(),
        "| --- | --- | ---: | ---: | ---: |".to_string(),
    ];
    for row in report
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` |",
            string_field(row, "module_id").unwrap_or_default(),
            string_field(row, "layer").unwrap_or_default(),
            row.get("covered").and_then(Value::as_u64).unwrap_or(0),
            row.get("partial").and_then(Value::as_u64).unwrap_or(0),
            row.get("planned").and_then(Value::as_u64).unwrap_or(0)
        ));
    }
    lines.push(String::new());
    lines.push("## Matrix".to_string());
    lines.push(String::new());
    lines.push(format!(
        "| Module | {} |",
        paradigms
            .iter()
            .map(|item| format!("`{item}`"))
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    lines.push(format!(
        "| --- | {} |",
        paradigms
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    for row in report
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let cells = row.get("cells").and_then(Value::as_object);
        let values = paradigms
            .iter()
            .map(|item| {
                format!(
                    "`{}`",
                    cells
                        .and_then(|cells| cells.get(item))
                        .and_then(Value::as_str)
                        .unwrap_or("not_applicable")
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!(
            "| `{}` | {values} |",
            string_field(row, "module_id").unwrap_or_default()
        ));
    }
    if let Some(findings) = report.get("findings").and_then(Value::as_array)
        && !findings.is_empty()
    {
        lines.extend([String::new(), "## Findings".to_string(), String::new()]);
        for finding in findings {
            lines.push(format!(
                "- `{}`: `{}` / `{}` is `{}`",
                string_field(finding, "severity").unwrap_or_default(),
                string_field(finding, "module_id").unwrap_or_default(),
                string_field(finding, "paradigm").unwrap_or_default(),
                string_field(finding, "status").unwrap_or_default()
            ));
        }
    }
    format!("{}\n", lines.join("\n").trim_end())
}

fn finding(severity: &str, module_id: &str, paradigm: &str, status: &str) -> Value {
    json!({
        "severity": severity,
        "module_id": module_id,
        "paradigm": paradigm,
        "status": status
    })
}

fn count_status(cells: &BTreeMap<String, Value>, expected: &str) -> usize {
    cells
        .values()
        .filter(|status| status.as_str() == Some(expected))
        .count()
}

fn object_keys(value: &Value, key: &str) -> BTreeSet<String> {
    value
        .get(key)
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|object| object.keys())
        .cloned()
        .collect()
}

fn ordered_paradigms(matrix: &Value) -> Vec<String> {
    let keys = object_keys(matrix, "paradigms");
    let mut ordered = Vec::new();
    for key in PREFERRED_PARADIGM_ORDER {
        if keys.contains(*key) {
            ordered.push((*key).to_string());
        }
    }
    for key in keys {
        if !ordered.contains(&key) {
            ordered.push(key);
        }
    }
    ordered
}

fn modules(topology: &Value) -> Vec<&Value> {
    topology
        .get("modules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect()
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn repo_path(root: &Path, relative_path: &str) -> RunnerResult<PathBuf> {
    let path = root.join(relative_path);
    if relative_path.is_empty()
        || relative_path.starts_with('/')
        || relative_path.split('/').any(|part| part == "..")
    {
        return Err(format!("path must stay inside repository: {relative_path}"));
    }
    Ok(path)
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(repo_path(root, relative_path)?)
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
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

fn run_self_test() -> RunnerResult<()> {
    let topology = json!({ "modules": [{ "id": "a", "layer": "contract" }] });
    let mut matrix = json!({
        "schema_version": SCHEMA_VERSION,
        "topology": TOPOLOGY_PATH,
        "paradigms": { "validation": "v" },
        "required_by_module": { "a": ["validation"] },
        "cells": { "a": { "validation": "covered" } }
    });
    if !validate_matrix(&matrix, &topology)?.is_empty() {
        return Err("self-test expected covered matrix to have no findings".to_string());
    }
    matrix["cells"]["a"]["validation"] = json!("planned");
    let findings = validate_matrix(&matrix, &topology)?;
    if string_field(&findings[0], "severity") != Some("warn") {
        return Err("self-test expected planned required cell to warn".to_string());
    }
    matrix["cells"]["a"]["validation"] = json!("missing");
    match validate_matrix(&matrix, &topology) {
        Err(error) if error.contains("unknown status") => Ok(()),
        Ok(_) => Err("self-test expected unknown status failure".to_string()),
        Err(error) => Err(format!(
            "self-test expected unknown status failure, got {error}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{Options, render_markdown};
    use serde_json::json;
    use std::ffi::OsString;

    #[test]
    fn parse_out_option() {
        let options = Options::parse(vec![
            OsString::from("--out"),
            OsString::from("tmp/out.json"),
        ])
        .unwrap();
        assert_eq!(options.out, "tmp/out.json");
    }

    #[test]
    fn markdown_renders_matrix_heading() {
        let report = json!({
            "ok": true,
            "module_count": 0,
            "paradigm_count": 1,
            "rows": [],
            "findings": []
        });
        let matrix = json!({ "paradigms": { "validation": "v" } });
        assert!(render_markdown(&report, &matrix).contains("## Matrix"));
    }
}
