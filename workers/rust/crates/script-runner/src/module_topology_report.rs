use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const DEFAULT_TOPOLOGY: &str = "config/architecture/module-topology.json";
const DEFAULT_OUT_DIR: &str = "tmp/module-topology";
const TOPOLOGY_SCHEMA: &str = "kyuubiki.module-topology/v1";
const REPORT_SCHEMA: &str = "kyuubiki.module-topology-report/v1";
const BENCHMARK_LANE_ORDER: &[&str] = &[
    "ui_startup",
    "workflow_catalog",
    "control_plane",
    "runtime_solver",
    "mesh",
    "sdk_headless",
    "installer_release",
];
const SECURITY_LANE_ORDER: &[&str] = &[
    "ui_boundary",
    "api_auth",
    "runtime_sandbox",
    "supply_chain",
    "credential_storage",
    "remote_deploy",
    "data_contract",
];

pub(crate) fn run_build_module_topology_report(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(root, args)?;
    let topology = read_json(&options.topology)?;
    if string_field(&topology, "schema_version") != Some(TOPOLOGY_SCHEMA) {
        return Err(format!(
            "unsupported module topology schema: {}",
            string_field(&topology, "schema_version").unwrap_or_default()
        ));
    }
    let report = build_report(&topology);
    let markdown = render_markdown(&report);
    fs::create_dir_all(&options.out_dir)
        .map_err(|error| format!("failed to create {}: {error}", options.out_dir.display()))?;
    write_text(
        &options.out_dir.join("index.json"),
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to render report json: {error}"))?
        ),
    )?;
    write_text(&options.out_dir.join("README.md"), &markdown)?;
    write_text(
        &options.out_dir.join("index.html"),
        &render_html(&report, &markdown),
    )?;
    println!(
        "module topology report written: {} ({} modules)",
        relative_display(root, &options.out_dir),
        report
            .get("module_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    Ok(0)
}

fn build_report(topology: &Value) -> Value {
    let modules = topology
        .get("modules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(module_summary)
        .collect::<Vec<_>>();
    json!({
        "schema_version": REPORT_SCHEMA,
        "generated_from": DEFAULT_TOPOLOGY,
        "version_line": string_field(topology, "version_line").unwrap_or_default(),
        "module_count": modules.len(),
        "benchmark_lanes": lane_index(topology, "benchmark_lanes", "benchmark", BENCHMARK_LANE_ORDER),
        "security_lanes": lane_index(topology, "security_lanes", "security", SECURITY_LANE_ORDER),
        "modules": modules
    })
}

fn module_summary(module: &Value) -> Value {
    json!({
        "id": string_field(module, "id").unwrap_or_default(),
        "layer": string_field(module, "layer").unwrap_or_default(),
        "summary": string_field(module, "summary").unwrap_or_default(),
        "owned_paths": string_array(module, "owned_paths"),
        "depends_on": string_array(module, "depends_on"),
        "service_surfaces": module.get("service_surfaces").and_then(Value::as_array).cloned().unwrap_or_default(),
        "benchmark_lanes": string_array(module, "benchmark_lanes"),
        "security_lanes": string_array(module, "security_lanes"),
        "risk_tags": string_array(module, "risk_tags")
    })
}

fn lane_index(topology: &Value, field: &str, plan_group: &str, lane_order: &[&str]) -> Value {
    let mut lanes = serde_json::Map::new();
    for lane_id in ordered_lanes(topology, field, lane_order) {
        let description = topology
            .get(field)
            .and_then(|lanes| lanes.get(&lane_id))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let modules = topology
            .get("modules")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|module| string_array(module, field).contains(&lane_id))
            .map(lane_module_summary)
            .collect::<Vec<_>>();
        lanes.insert(
            lane_id.clone(),
            json!({
                "description": description,
                "modules": modules,
                "test_plan": topology.pointer(&format!("/lane_test_plan/{plan_group}/{lane_id}")).and_then(Value::as_array).cloned().unwrap_or_default()
            }),
        );
    }
    Value::Object(lanes)
}

fn lane_module_summary(module: &Value) -> Value {
    json!({
        "id": string_field(module, "id").unwrap_or_default(),
        "layer": string_field(module, "layer").unwrap_or_default(),
        "owned_paths": string_array(module, "owned_paths"),
        "service_surfaces": module.get("service_surfaces").and_then(Value::as_array).cloned().unwrap_or_default(),
        "risk_tags": string_array(module, "risk_tags")
    })
}

fn render_markdown(report: &Value) -> String {
    let mut lines = vec![
        "# Module Topology Report".to_string(),
        String::new(),
        format!(
            "- Schema: `{}`",
            string_field(report, "schema_version").unwrap_or_default()
        ),
        format!(
            "- Source: `{}`",
            string_field(report, "generated_from").unwrap_or_default()
        ),
        format!(
            "- Version line: `{}`",
            string_field(report, "version_line").unwrap_or_default()
        ),
        format!(
            "- Modules: `{}`",
            report
                .get("module_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        String::new(),
        "## Modules".to_string(),
        String::new(),
        "| Module | Layer | Depends on | Service surfaces | Benchmark lanes | Security lanes |"
            .to_string(),
        "| --- | --- | --- | --- | --- | --- |".to_string(),
    ];
    for module in report
        .get("modules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "| `{}` | `{}` | {} | {} | {} | {} |",
            string_field(module, "id").unwrap_or_default(),
            string_field(module, "layer").unwrap_or_default(),
            tick_join(&string_array(module, "depends_on")),
            surface_join(module),
            tick_join(&string_array(module, "benchmark_lanes")),
            tick_join(&string_array(module, "security_lanes"))
        ));
    }
    lines.push(String::new());
    render_lane_markdown(
        "Benchmark Lanes",
        report,
        "benchmark_lanes",
        BENCHMARK_LANE_ORDER,
        &mut lines,
    );
    render_lane_markdown(
        "Security Lanes",
        report,
        "security_lanes",
        SECURITY_LANE_ORDER,
        &mut lines,
    );
    format!("{}\n", lines.join("\n").trim_end())
}

fn render_lane_markdown(
    title: &str,
    report: &Value,
    key: &str,
    lane_order: &[&str],
    lines: &mut Vec<String>,
) {
    lines.push(format!("## {title}"));
    lines.push(String::new());
    for lane_id in ordered_report_lanes(report, key, lane_order) {
        let lane = report
            .pointer(&format!("/{key}/{lane_id}"))
            .unwrap_or(&Value::Null);
        lines.push(format!("### `{lane_id}`"));
        lines.push(String::new());
        lines.push(
            string_field(lane, "description")
                .unwrap_or_default()
                .to_string(),
        );
        lines.push(String::new());
        lines.push("| Module | Layer | Risks |".to_string());
        lines.push("| --- | --- | --- |".to_string());
        for module in lane
            .get("modules")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            lines.push(format!(
                "| `{}` | `{}` | {} |",
                string_field(module, "id").unwrap_or_default(),
                string_field(module, "layer").unwrap_or_default(),
                tick_join(&string_array(module, "risk_tags"))
            ));
        }
        lines.push(String::new());
        lines.push("Suggested commands:".to_string());
        lines.push(String::new());
        for entry in lane
            .get("test_plan")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            lines.push(format!(
                "- `{}` ({}, {})",
                string_field(entry, "command").unwrap_or_default(),
                string_field(entry, "scope").unwrap_or_default(),
                string_field(entry, "id").unwrap_or_default()
            ));
        }
        lines.push(String::new());
    }
}

fn render_html(report: &Value, markdown: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Kyuubiki Module Topology</title>
  <style>
    body {{ margin: 0; background: #11161d; color: #e7edf6; font: 15px/1.6 ui-sans-serif, system-ui; }}
    main {{ max-width: 1120px; margin: 0 auto; padding: 48px 24px; }}
    pre {{ white-space: pre-wrap; background: #18212c; border: 1px solid #2d3b4c; border-radius: 16px; padding: 24px; }}
    code {{ color: #9bdcff; }}
  </style>
</head>
<body>
  <main>
    <h1>Kyuubiki Module Topology</h1>
    <p>Generated from <code>{}</code>.</p>
    <pre>{}</pre>
  </main>
</body>
</html>
"#,
        escape_html(string_field(report, "generated_from").unwrap_or_default()),
        escape_html(markdown)
    )
}

fn ordered_lanes(topology: &Value, field: &str, lane_order: &[&str]) -> Vec<String> {
    let lanes = topology
        .get(field)
        .and_then(Value::as_object)
        .map(|object| object.keys().map(String::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    ordered_strings(&lanes, lane_order)
}

fn ordered_report_lanes(report: &Value, key: &str, lane_order: &[&str]) -> Vec<String> {
    let lanes = report
        .get(key)
        .and_then(Value::as_object)
        .map(|object| object.keys().map(String::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    ordered_strings(&lanes, lane_order)
}

fn ordered_strings(values: &[&str], preferred: &[&str]) -> Vec<String> {
    let mut ordered = preferred
        .iter()
        .filter(|item| values.contains(item))
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();
    ordered.extend(
        values
            .iter()
            .filter(|item| !preferred.contains(item))
            .map(|item| (*item).to_string()),
    );
    ordered
}

fn surface_join(module: &Value) -> String {
    let surfaces = module
        .get("service_surfaces")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|surface| {
            format!(
                "`{}`/`{}`",
                string_field(surface, "id").unwrap_or_default(),
                string_field(surface, "kind").unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    if surfaces.is_empty() {
        "-".to_string()
    } else {
        surfaces.join(", ")
    }
}

fn tick_join(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values
            .iter()
            .map(|value| format!("`{value}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("{}: invalid json: {error}", path.display()))
}

fn write_text(path: &Path, text: &str) -> RunnerResult<()> {
    fs::write(path, text).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[derive(Debug)]
struct Options {
    topology: PathBuf,
    out_dir: PathBuf,
}

impl Options {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            topology: root.join(DEFAULT_TOPOLOGY),
            out_dir: root.join(DEFAULT_OUT_DIR),
        };
        let mut index = 0;
        while index < args.len() {
            let arg = args[index]
                .to_str()
                .ok_or_else(|| "non-utf8 argument".to_string())?;
            match arg {
                "--topology" => {
                    let value = next_arg(&args, index, "--topology")?;
                    options.topology = path_arg(root, value);
                    index += 2;
                }
                "--out-dir" => {
                    let value = next_arg(&args, index, "--out-dir")?;
                    options.out_dir = path_arg(root, value);
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

fn path_arg(root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}
