use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

type RunnerResult<T> = Result<T, String>;

const DEFAULT_RETAIN: usize = 12;

pub(crate) fn run_build_workflow_mesh_regression_index(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(repo_root, args)?;
    fs::create_dir_all(&options.root)
        .map_err(|error| format!("failed to create {}: {error}", options.root.display()))?;
    let discovered = discover_runs(&options.root)?;
    let (kept, removed) = prune_runs(discovered, options.retain)?;
    let payload = json!({
        "schema_version": "kyuubiki.workflow-mesh-regression-index/v1",
        "root": display_path(repo_root, &options.root),
        "generated_at_unix_s": unix_now(),
        "retain": options.retain,
        "retained_runs": kept.iter().map(WorkflowMeshRun::to_json).collect::<Vec<_>>(),
        "pruned_slugs": removed.iter().map(|run| run.slug.clone()).collect::<Vec<_>>(),
    });
    write_json(&options.root.join("index.json"), &payload)?;
    write_text(
        &options.root.join("README.md"),
        &render_readme(repo_root, &options.root, &kept, &removed, options.retain),
    )?;
    write_text(
        &options.root.join("index.html"),
        &render_html(repo_root, &options.root, &kept, &removed, options.retain),
    )?;
    println!("{}", display_path(repo_root, &options.root));
    Ok(0)
}

struct Options {
    retain: usize,
    root: PathBuf,
}

#[derive(Clone)]
struct WorkflowMeshRun {
    dir: PathBuf,
    files: RunFiles,
    generated_at_unix_s: u64,
    status: String,
    tests: Value,
    total_duration_ms: f64,
    total_fail: u64,
    total_pass: u64,
    total_tests: u64,
    slug: String,
}

#[derive(Clone)]
struct RunFiles {
    readme_md: String,
    run_log: String,
    summary_json: String,
}

impl Options {
    fn parse(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let mut root = repo_root.join("tmp/workflow-mesh-regression");
        let mut retain = DEFAULT_RETAIN;
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--root" => root = path_arg(repo_root, &mut iter, "--root")?,
                "--retain" => retain = usize_arg(&mut iter, "--retain")?,
                "--help" | "-h" => {
                    print_usage();
                    return Ok(Self { retain, root });
                }
                other => return Err(format!("unknown option: {other}")),
            }
        }
        Ok(Self { retain, root })
    }
}

impl WorkflowMeshRun {
    fn to_json(&self) -> Value {
        json!({
            "slug": self.slug,
            "generated_at_unix_s": self.generated_at_unix_s,
            "status": self.status,
            "total_tests": self.total_tests,
            "total_pass": self.total_pass,
            "total_fail": self.total_fail,
            "total_duration_ms": self.total_duration_ms,
            "tests": self.tests,
            "files": {
                "summary_json": self.files.summary_json,
                "readme_md": self.files.readme_md,
                "run_log": self.files.run_log,
            },
        })
    }
}

fn discover_runs(root: &Path) -> RunnerResult<Vec<WorkflowMeshRun>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("failed to read {}: {error}", root.display())),
    };
    let mut runs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read dir entry: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", entry.path().display()))?
            .is_dir()
        {
            continue;
        }
        if let Some(run) = discover_run(root, &entry.path())? {
            runs.push(run);
        }
    }
    runs.sort_by(|left, right| right.generated_at_unix_s.cmp(&left.generated_at_unix_s));
    Ok(runs)
}

fn discover_run(root: &Path, run_dir: &Path) -> RunnerResult<Option<WorkflowMeshRun>> {
    let summary_path = run_dir.join("summary.json");
    if !summary_path.is_file() {
        return Ok(None);
    }
    let payload = read_json(&summary_path)?;
    let fallback_generated_at = modified_unix_s(&summary_path)?;
    Ok(Some(WorkflowMeshRun {
        dir: run_dir.to_path_buf(),
        files: RunFiles {
            summary_json: display_path(root, &summary_path),
            readme_md: display_path(root, &run_dir.join("README.md")),
            run_log: display_path(root, &run_dir.join("run.log")),
        },
        generated_at_unix_s: payload
            .get("generated_at_unix_s")
            .and_then(Value::as_u64)
            .unwrap_or(fallback_generated_at),
        status: payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        tests: payload
            .get("tests")
            .cloned()
            .filter(Value::is_array)
            .unwrap_or(json!([])),
        total_duration_ms: payload
            .get("total_duration_ms")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        total_fail: payload
            .get("total_fail")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        total_pass: payload
            .get("total_pass")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        total_tests: payload
            .get("total_tests")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        slug: run_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
    }))
}

fn prune_runs(
    mut runs: Vec<WorkflowMeshRun>,
    retain: usize,
) -> RunnerResult<(Vec<WorkflowMeshRun>, Vec<WorkflowMeshRun>)> {
    if runs.len() <= retain {
        return Ok((runs, Vec::new()));
    }
    let removed = runs.split_off(retain);
    for run in &removed {
        fs::remove_dir_all(&run.dir)
            .map_err(|error| format!("failed to remove {}: {error}", run.dir.display()))?;
    }
    Ok((runs, removed))
}

fn render_readme(
    root: &Path,
    run_root: &Path,
    runs: &[WorkflowMeshRun],
    removed: &[WorkflowMeshRun],
    retain: usize,
) -> String {
    let mut lines = vec![
        "# Workflow Mesh Regression Runs".to_string(),
        String::new(),
        format!("- Root: `{}`", display_path(root, run_root)),
        format!("- Retention window: newest `{retain}` run directories"),
        format!("- Retained runs: `{}`", runs.len()),
        format!("- Pruned this refresh: `{}`", removed.len()),
        String::new(),
    ];
    if runs.is_empty() {
        lines.push("No retained workflow mesh runs were found.".to_string());
    } else {
        lines.push("| Slug | Status | Tests | Pass | Fail | Duration ms |".to_string());
        lines.push("| --- | --- | ---: | ---: | ---: | ---: |".to_string());
        for run in runs {
            lines.push(format!(
                "| `{}` | `{}` | `{}` | `{}` | `{}` | `{:.3}` |",
                run.slug,
                run.status,
                run.total_tests,
                run.total_pass,
                run.total_fail,
                run.total_duration_ms
            ));
        }
    }
    format!("{}\n", lines.join("\n").trim_end())
}

fn render_html(
    root: &Path,
    run_root: &Path,
    runs: &[WorkflowMeshRun],
    removed: &[WorkflowMeshRun],
    retain: usize,
) -> String {
    let cards = if runs.is_empty() {
        "<article class=\"docs-card\"><h2>No retained runs</h2><p class=\"docs-copy\">Run the workflow mesh regression wrapper to populate this index.</p></article>".to_string()
    } else {
        runs.iter().map(render_card).collect::<Vec<_>>().join("\n")
    };
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\" /><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" /><title>Kyuubiki Workflow Mesh Regression Runs</title><link rel=\"stylesheet\" href=\"../../apps/hub-gui/ui/docs/docs.css\" /></head><body><main class=\"docs-shell\"><section class=\"docs-hero\"><div class=\"docs-kicker\">Workflow Mesh Index</div><h1>Retained workflow mesh regression runs</h1><p class=\"docs-copy\">This page tracks the retained local and remote workflow mesh regression outputs using the same artifact layout.</p><div class=\"docs-meta\"><span class=\"docs-chip\">Root: {}</span><span class=\"docs-chip\">Retention window: {}</span><span class=\"docs-chip\">Retained runs: {}</span><span class=\"docs-chip\">Pruned this refresh: {}</span></div><div class=\"docs-links\"><a class=\"docs-link\" href=\"./README.md\">Open README</a><a class=\"docs-link\" href=\"./index.json\">Open JSON index</a><a class=\"docs-link\" href=\"../../docs/testing-and-ci.md\">Open testing guide</a></div></section><section class=\"docs-grid\">{}</section></main></body></html>\n",
        escape_html(&display_path(root, run_root)),
        retain,
        runs.len(),
        removed.len(),
        cards
    )
}

fn render_card(run: &WorkflowMeshRun) -> String {
    format!(
        "<article class=\"docs-card\"><div class=\"docs-kicker\">workflow mesh run</div><h2>{}</h2><p class=\"docs-copy\">Status <code>{}</code> · total tests <code>{}</code> · duration ms <code>{:.3}</code></p><div class=\"docs-meta\"><span class=\"docs-chip\">Pass: {}</span><span class=\"docs-chip\">Fail: {}</span><span class=\"docs-chip\">Generated at unix: {}</span></div><ul class=\"docs-list\"><li><code>{}</code></li><li><code>{}</code></li><li><code>{}</code></li></ul></article>",
        escape_html(&run.slug),
        escape_html(&run.status),
        run.total_tests,
        run.total_duration_ms,
        run.total_pass,
        run.total_fail,
        run.generated_at_unix_s,
        escape_html(&run.files.summary_json),
        escape_html(&run.files.readme_md),
        escape_html(&run.files.run_log)
    )
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

fn usize_arg(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<usize> {
    iter.next()
        .and_then(|value| value.to_string_lossy().parse::<usize>().ok())
        .ok_or_else(|| format!("invalid value for {flag}"))
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
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

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    write_text(
        path,
        &format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
    )
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

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn print_usage() {
    println!(
        "Usage: ./scripts/kyuubiki build-workflow-mesh-regression-index [--root tmp/workflow-mesh-regression] [--retain 12]"
    );
}
