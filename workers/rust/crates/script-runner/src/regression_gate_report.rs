use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const DEFAULT_TMP_ROOT: &str = "tmp";

pub(crate) fn run_build_regression_gate_report(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(repo_root, args)?;
    let catalog_path = options
        .catalog_path
        .clone()
        .unwrap_or_else(|| options.tmp_root.join("regression-lane-catalog.json"));
    let catalog = read_json(&catalog_path)?;
    let lanes = normalize_lanes(
        catalog["lanes"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    let enforced = lanes
        .iter()
        .filter(|lane| lane["gate_scope"].as_str() != Some("advisory"))
        .collect::<Vec<_>>();
    let failing = enforced
        .iter()
        .filter(|lane| lane["gate_status"].as_str() == Some("fail"))
        .count();
    let warning = enforced
        .iter()
        .filter(|lane| lane["gate_status"].as_str() == Some("warn"))
        .count();
    let report = json!({
        "schema_version": "kyuubiki.regression-gate-report/v1",
        "generated_at_unix_s": unix_seconds_now(),
        "catalog_path": display_path(repo_root, &catalog_path),
        "overall_gate_status": catalog["overall_gate_status"].as_str().unwrap_or("unknown"),
        "failing_lane_count": failing,
        "warning_lane_count": warning,
        "lanes": lanes,
    });

    fs::create_dir_all(&options.tmp_root).map_err(|error| {
        format!(
            "failed to create tmp root {}: {error}",
            options.tmp_root.display()
        )
    })?;
    write_json(
        &options.tmp_root.join("regression-gate-report.json"),
        &report,
    )?;
    fs::write(
        options.tmp_root.join("regression-gate-report.md"),
        render_markdown(&report),
    )
    .map_err(|error| format!("failed to write regression-gate-report.md: {error}"))?;

    let status = report["overall_gate_status"].as_str().unwrap_or("unknown");
    println!("{status}");
    if status == "fail" {
        return Ok(2);
    }
    if options.fail_on_warn && status == "warn" {
        return Ok(1);
    }
    Ok(0)
}

struct Options {
    catalog_path: Option<PathBuf>,
    fail_on_warn: bool,
    tmp_root: PathBuf,
}

fn parse_args(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        catalog_path: None,
        fail_on_warn: false,
        tmp_root: repo_root.join(DEFAULT_TMP_ROOT),
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                print_usage();
                return Ok(options);
            }
            "--tmp-root" => options.tmp_root = path_arg(repo_root, &mut iter, "--tmp-root")?,
            "--catalog" => {
                options.catalog_path = Some(path_arg(repo_root, &mut iter, "--catalog")?)
            }
            "--fail-on-warn" => options.fail_on_warn = true,
            other => return Err(format!("unknown option: {other}")),
        }
    }
    Ok(options)
}

fn normalize_lanes(lanes: &[Value]) -> Vec<Value> {
    lanes
        .iter()
        .map(|lane| {
            json!({
                "id": lane["id"].as_str().unwrap_or(""),
                "title": lane["title"].as_str().unwrap_or(""),
                "category": lane["category"].as_str().unwrap_or(""),
                "gate_scope": lane["gate_scope"].as_str().unwrap_or("enforced"),
                "status": lane["status"].as_str().unwrap_or("unknown"),
                "gate_status": lane.pointer("/gate/status").and_then(Value::as_str).or_else(|| lane["status"].as_str()).unwrap_or("unknown"),
                "gate_reasons": lane.pointer("/gate/reasons").and_then(Value::as_array).cloned().unwrap_or_default(),
                "generated_at_unix_s": lane["generated_at_unix_s"].as_i64().unwrap_or(0),
                "links": lane["links"].as_array().cloned().unwrap_or_default(),
            })
        })
        .collect()
}

fn render_markdown(report: &Value) -> String {
    let lanes = report["lanes"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut lines = vec![
        "# Regression Gate Report".to_string(),
        String::new(),
        format!("- Catalog: `{}`", str_at(report, "/catalog_path")),
        format!(
            "- Overall gate status: `{}`",
            str_at(report, "/overall_gate_status")
        ),
        format!(
            "- Generated at unix: `{}`",
            int_at(report, "/generated_at_unix_s")
        ),
        format!(
            "- Failing lane count: `{}`",
            int_at(report, "/failing_lane_count")
        ),
        format!(
            "- Warning lane count: `{}`",
            int_at(report, "/warning_lane_count")
        ),
        String::new(),
        "| Lane | Scope | Gate | Status | Reason count |".to_string(),
        "| --- | --- | --- | --- | ---: |".to_string(),
    ];
    for lane in lanes {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` |",
            str_at(lane, "/id"),
            str_at(lane, "/gate_scope"),
            str_at(lane, "/gate_status"),
            str_at(lane, "/status"),
            array_len(lane.pointer("/gate_reasons")),
        ));
    }
    render_reasons(&mut lines, lanes);
    lines.push(String::new());
    format!("{}\n", lines.join("\n").trim_end())
}

fn render_reasons(lines: &mut Vec<String>, lanes: &[Value]) {
    let lanes_with_reasons = lanes
        .iter()
        .filter(|lane| array_len(lane.pointer("/gate_reasons")) > 0)
        .collect::<Vec<_>>();
    if lanes_with_reasons.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("## Reasons".to_string());
    lines.push(String::new());
    for lane in lanes_with_reasons {
        lines.push(format!("- `{}`", str_at(lane, "/id")));
        if let Some(reasons) = lane["gate_reasons"].as_array() {
            for reason in reasons.iter().filter_map(Value::as_str) {
                lines.push(format!("  {reason}"));
            }
        }
    }
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki build-regression-gate-report [options]\n\n\
Options:\n  --tmp-root <path>       Tmp root containing regression-lane-catalog.json.\n  \
--catalog <path>        Optional explicit regression lane catalog path.\n  \
--fail-on-warn          Exit non-zero when overall gate is warn.\n  \
--help                  Show this message."
    );
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode {}: {error}", path.display()))?;
    fs::write(path, format!("{content}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn path_arg(
    repo_root: &Path,
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> RunnerResult<PathBuf> {
    let value = iter
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))?;
    Ok(if value.is_absolute() {
        value
    } else {
        repo_root.join(value)
    })
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn str_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}

fn int_at(value: &Value, pointer: &str) -> i64 {
    value.pointer(pointer).and_then(Value::as_i64).unwrap_or(0)
}

fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}

fn unix_seconds_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
