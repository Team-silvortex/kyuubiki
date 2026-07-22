use serde_json::json;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_build_workflow_mesh_regression_summary(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(repo_root, args)?;
    let output_dir = options
        .output_dir
        .clone()
        .unwrap_or_else(|| options.log_path.parent().unwrap_or(repo_root).to_path_buf());
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;
    let log_text = fs::read_to_string(&options.log_path)
        .map_err(|error| format!("failed to read {}: {error}", options.log_path.display()))?;
    let parsed = parse_log(&log_text);
    let log_mtime_unix_s = modified_unix_s(&options.log_path)?;
    let log_path_label = display_path(repo_root, &options.log_path);
    let qualification_path = output_dir.join("agent-task-ir-qualification.json");
    let qualification_label = qualification_path
        .is_file()
        .then(|| display_path(repo_root, &qualification_path));
    let status = if parsed.completed && parsed.total_fail == 0 {
        "passed"
    } else {
        "failed"
    };
    let payload = json!({
        "schema_version": "kyuubiki.workflow-mesh-regression-summary/v1",
        "generated_at_unix_s": unix_now(),
        "log_path": log_path_label,
        "completed": parsed.completed,
        "status": status,
        "total_tests": parsed.tests.len(),
        "total_pass": parsed.total_pass,
        "total_fail": parsed.total_fail,
        "total_duration_ms": parsed.total_duration_ms,
        "log_mtime_unix_s": log_mtime_unix_s,
        "tests": parsed.tests.iter().map(TestCase::to_json).collect::<Vec<_>>(),
        "artifacts": {
            "agent_solver_qualification": qualification_label
        }
    });
    write_text(
        &output_dir.join("summary.json"),
        &format!("{}\n", serde_json::to_string_pretty(&payload).unwrap()),
    )?;
    write_text(
        &output_dir.join("README.md"),
        &render_readme(
            &parsed,
            status,
            &log_path_label,
            qualification_label.as_deref(),
        ),
    )?;
    println!("{}", display_path(repo_root, &output_dir));
    Ok(0)
}

struct Options {
    log_path: PathBuf,
    output_dir: Option<PathBuf>,
}

struct ParsedLog {
    completed: bool,
    tests: Vec<TestCase>,
    total_duration_ms: f64,
    total_fail: u64,
    total_pass: u64,
}

#[derive(Clone)]
struct TestCase {
    duration_ms: Option<f64>,
    fail: u64,
    pass: u64,
    status: String,
    subtest: Option<String>,
    test_file: String,
}

impl Options {
    fn parse(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let mut log_path = repo_root.join("tmp/workflow-mesh-regression/latest/run.log");
        let mut output_dir = None;
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--log" => log_path = path_arg(repo_root, &mut iter, "--log")?,
                "--output-dir" => {
                    output_dir = Some(path_arg(repo_root, &mut iter, "--output-dir")?)
                }
                "--help" | "-h" => {
                    print_usage();
                    return Ok(Self {
                        log_path,
                        output_dir,
                    });
                }
                other => return Err(format!("unknown option: {other}")),
            }
        }
        Ok(Self {
            log_path,
            output_dir,
        })
    }
}

impl TestCase {
    fn new(test_file: String) -> Self {
        Self {
            duration_ms: None,
            fail: 0,
            pass: 0,
            status: "running".to_string(),
            subtest: None,
            test_file,
        }
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "test_file": self.test_file,
            "subtest": self.subtest,
            "pass": self.pass,
            "fail": self.fail,
            "duration_ms": self.duration_ms,
            "status": self.status,
        })
    }
}

fn parse_log(log_text: &str) -> ParsedLog {
    let mut tests = Vec::new();
    let mut current: Option<TestCase> = None;
    let mut total_pass = 0;
    let mut total_fail = 0;
    let mut completed = false;

    for line in log_text.lines() {
        if let Some(test_file) = line.strip_prefix("==> running ") {
            if let Some(test) = current.take() {
                tests.push(test);
            }
            current = Some(TestCase::new(test_file.to_string()));
            continue;
        }

        if line.trim() == "workflow mesh regression completed" {
            completed = true;
            continue;
        }

        let Some(test) = current.as_mut() else {
            continue;
        };

        if let Some(subtest) = line.strip_prefix("# Subtest: ") {
            test.subtest = Some(subtest.to_string());
        } else if let Some((name, duration)) = parse_spec_line(line, "✔ ") {
            test.subtest = Some(name);
            test.pass = 1;
            test.fail = 0;
            test.duration_ms = Some(duration);
            test.status = "passed".to_string();
            total_pass += 1;
        } else if let Some((name, duration)) = parse_spec_line(line, "✖ ") {
            test.subtest = Some(name);
            test.pass = 0;
            test.fail = 1;
            test.duration_ms = Some(duration);
            test.status = "failed".to_string();
            total_fail += 1;
        } else if let Some(value) =
            parse_count(line, "# pass ").or_else(|| parse_count(line, "ℹ pass "))
        {
            test.pass = value;
            if test.status != "passed" {
                total_pass += value;
            }
        } else if let Some(value) =
            parse_count(line, "# fail ").or_else(|| parse_count(line, "ℹ fail "))
        {
            test.fail = value;
            if test.status != "failed" {
                total_fail += value;
            }
            if value > 0 {
                test.status = "failed".to_string();
            }
        } else if let Some(duration) = parse_duration(line, "# duration_ms ")
            .or_else(|| parse_duration(line, "ℹ duration_ms "))
        {
            test.duration_ms = Some(duration);
            if test.status != "failed" {
                test.status = if test.pass > 0 { "passed" } else { "completed" }.to_string();
            }
            tests.push(current.take().unwrap());
        }
    }
    if let Some(test) = current {
        tests.push(test);
    }
    let total_duration_ms = tests.iter().filter_map(|test| test.duration_ms).sum();
    ParsedLog {
        completed,
        tests,
        total_duration_ms,
        total_fail,
        total_pass,
    }
}

fn parse_spec_line(line: &str, marker: &str) -> Option<(String, f64)> {
    let body = line.strip_prefix(marker)?;
    let (name, duration) = body.rsplit_once(" (")?;
    let duration = duration.strip_suffix("ms)")?.parse().ok()?;
    Some((name.to_string(), duration))
}

fn parse_count(line: &str, prefix: &str) -> Option<u64> {
    line.strip_prefix(prefix)?.parse().ok()
}

fn parse_duration(line: &str, prefix: &str) -> Option<f64> {
    line.strip_prefix(prefix)?.parse().ok()
}

fn render_readme(
    parsed: &ParsedLog,
    status: &str,
    log_path: &str,
    qualification_path: Option<&str>,
) -> String {
    let mut lines = vec![
        "# Workflow Mesh Regression".to_string(),
        String::new(),
        format!("- Status: `{status}`"),
        format!("- Completed marker: `{}`", parsed.completed),
        format!("- Total tests: `{}`", parsed.tests.len()),
        format!("- Total pass: `{}`", parsed.total_pass),
        format!("- Total fail: `{}`", parsed.total_fail),
        format!("- Total duration ms: `{:.3}`", parsed.total_duration_ms),
        format!("- Log: `{log_path}`"),
        format!(
            "- Agent solver qualification: {}",
            qualification_path
                .map(|path| format!("`{path}`"))
                .unwrap_or_else(|| "`not generated`".to_string())
        ),
        String::new(),
        "## Cases".to_string(),
        String::new(),
    ];
    for test in &parsed.tests {
        lines.push(format!("- `{}`", test.test_file));
        lines.push(format!("  Status: `{}`", test.status));
        lines.push(format!(
            "  Subtest: {}",
            test.subtest
                .as_ref()
                .map(|subtest| format!("`{subtest}`"))
                .unwrap_or_else(|| "`n/a`".to_string())
        ));
        lines.push(format!(
            "  Pass/fail: `{}` / `{}`, duration ms: `{:.3}`",
            test.pass,
            test.fail,
            test.duration_ms.unwrap_or(0.0)
        ));
    }
    format!("{}\n", lines.join("\n").trim_end())
}

fn path_arg(
    repo_root: &Path,
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> RunnerResult<PathBuf> {
    let value = iter
        .next()
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))?;
    let path = PathBuf::from(value);
    Ok(if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    })
}

fn modified_unix_s(path: &Path) -> RunnerResult<u64> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("failed to stat {}: {error}", path.display()))?
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("invalid mtime for {}: {error}", path.display()))
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn write_text(path: &Path, content: &str) -> RunnerResult<()> {
    fs::write(path, content).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn print_usage() {
    println!(
        "Usage: ./scripts/kyuubiki build-workflow-mesh-regression-summary [--log tmp/workflow-mesh-regression/latest/run.log] [--output-dir tmp/workflow-mesh-regression/latest]"
    );
}
