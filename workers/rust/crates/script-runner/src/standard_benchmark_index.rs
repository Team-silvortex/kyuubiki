use serde_json::json;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

type RunnerResult<T> = Result<T, String>;

const DEFAULT_RETAIN: usize = 12;
const REPORT_FILES: [&str; 6] = [
    "standard-10k-compare.md",
    "standard-15k-compare.md",
    "standard-20k-compare.md",
    "standard-100k-compare.md",
    "standard-200k-compare.md",
    "standard-300k-compare.md",
];

pub(crate) fn run_build_standard_benchmark_index(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(repo_root, args)?;
    fs::create_dir_all(&options.root)
        .map_err(|error| format!("failed to create {}: {error}", options.root.display()))?;
    let discovered = discover_runs(&options.root)?;
    let (mut kept, removed) = prune_runs(discovered, options.retain)?;
    for run in &mut kept {
        run.headline = read_headline(&options.root.join(&run.merged_report));
    }

    let payload = json!({
        "schema_version": "kyuubiki.standard-benchmark-index/v1",
        "root": display_path(repo_root, &options.root),
        "generated_at_unix_s": unix_now(),
        "retain": options.retain,
        "retained_runs": kept.iter().map(StandardRun::to_json).collect::<Vec<_>>(),
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
struct StandardRun {
    dir: PathBuf,
    generated_at_unix_s: u64,
    headline: String,
    matrix_reports: Vec<String>,
    merged_report: String,
    profile: String,
    slug: String,
}

impl Options {
    fn parse(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let mut root = repo_root.join("tmp/standard-benchmark");
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

impl StandardRun {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "slug": self.slug,
            "dir": self.dir,
            "profile": self.profile,
            "merged_report": self.merged_report,
            "matrix_reports": self.matrix_reports,
            "generated_at_unix_s": self.generated_at_unix_s,
            "headline": self.headline,
        })
    }
}

fn discover_runs(root: &Path) -> RunnerResult<Vec<StandardRun>> {
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

fn discover_run(root: &Path, run_dir: &Path) -> RunnerResult<Option<StandardRun>> {
    let Some(report) = REPORT_FILES
        .iter()
        .map(|file| run_dir.join(file))
        .find(|candidate| candidate.is_file())
    else {
        return Ok(None);
    };
    let generated_at_unix_s = modified_unix_s(&report)?;
    let profile = report
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .trim_start_matches("standard-")
        .trim_end_matches("-compare.md")
        .to_string();
    let matrix_reports = ["mechanical-core", "thermal-core", "compound-core"]
        .iter()
        .map(|matrix| format!("{matrix}-{profile}-compare.md"))
        .collect();
    Ok(Some(StandardRun {
        dir: run_dir.to_path_buf(),
        generated_at_unix_s,
        headline: String::new(),
        matrix_reports,
        merged_report: display_path(root, &report),
        profile,
        slug: run_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
    }))
}

fn prune_runs(
    mut runs: Vec<StandardRun>,
    retain: usize,
) -> RunnerResult<(Vec<StandardRun>, Vec<StandardRun>)> {
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

fn read_headline(report_path: &Path) -> String {
    fs::read_to_string(report_path)
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("| `") && !line.contains("| --- |"))
                .map(str::to_string)
        })
        .unwrap_or_default()
}

fn render_readme(
    root: &Path,
    run_root: &Path,
    runs: &[StandardRun],
    removed: &[StandardRun],
    retain: usize,
) -> String {
    let mut lines = vec![
        "# Standard Benchmark Runs".to_string(),
        String::new(),
        format!("- Root: `{}`", display_path(root, run_root)),
        format!("- Retention window: newest `{retain}` run directories"),
        format!("- Retained runs: `{}`", runs.len()),
        format!("- Pruned this refresh: `{}`", removed.len()),
        String::new(),
    ];
    if runs.is_empty() {
        lines.push("No retained standard benchmark runs were found.".to_string());
    } else {
        lines.push("| Slug | Profile | Generated (unix) | Merged report |".to_string());
        lines.push("| --- | --- | ---: | --- |".to_string());
        for run in runs {
            lines.push(format!(
                "| `{}` | `{}` | `{}` | `{}` |",
                run.slug, run.profile, run.generated_at_unix_s, run.merged_report
            ));
        }
    }
    format!("{}\n", lines.join("\n").trim_end())
}

fn render_html(
    root: &Path,
    run_root: &Path,
    runs: &[StandardRun],
    removed: &[StandardRun],
    retain: usize,
) -> String {
    let cards = if runs.is_empty() {
        "<article class=\"docs-card\"><h2>No retained runs</h2><p class=\"docs-copy\">Run the standard benchmark regression wrapper to populate this index.</p></article>".to_string()
    } else {
        runs.iter().map(render_card).collect::<Vec<_>>().join("\n")
    };
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\" /><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" /><title>Kyuubiki Standard Benchmark Runs</title><link rel=\"stylesheet\" href=\"../../apps/hub-gui/ui/docs/docs.css\" /></head><body><main class=\"docs-shell\"><section class=\"docs-hero\"><div class=\"docs-kicker\">Standard Benchmark Index</div><h1>Retained nightly benchmark runs</h1><p class=\"docs-copy\">This page is generated from the local standard benchmark artifact directory and mirrors the same retained run window used by the remote regression wrapper.</p><div class=\"docs-meta\"><span class=\"docs-chip\">Root: {}</span><span class=\"docs-chip\">Retention window: {}</span><span class=\"docs-chip\">Retained runs: {}</span><span class=\"docs-chip\">Pruned this refresh: {}</span></div><div class=\"docs-links\"><a class=\"docs-link\" href=\"./README.md\">Open README</a><a class=\"docs-link\" href=\"./index.json\">Open JSON index</a><a class=\"docs-link\" href=\"../../docs/testing-and-ci.md\">Open testing guide</a></div></section><section class=\"docs-grid\">{}</section></main></body></html>\n",
        escape_html(&display_path(root, run_root)),
        retain,
        runs.len(),
        removed.len(),
        cards
    )
}

fn render_card(run: &StandardRun) -> String {
    let reports = run
        .matrix_reports
        .iter()
        .map(|report| format!("<li><code>{}</code></li>", escape_html(report)))
        .collect::<Vec<_>>()
        .join("\n");
    let headline = if run.headline.is_empty() {
        String::new()
    } else {
        format!(
            "<p class=\"docs-copy\"><strong>First reported row:</strong> <code>{}</code></p>",
            escape_html(&run.headline)
        )
    };
    format!(
        "<article class=\"docs-card\"><div class=\"docs-kicker\">standard run</div><h2>{}</h2><p class=\"docs-copy\">Profile <code>{}</code> · generated at unix <code>{}</code></p><div class=\"docs-meta\"><span class=\"docs-chip\">Merged report: <code>{}</code></span></div>{}<h3>Per-matrix reports</h3><ul class=\"docs-list\">{}</ul></article>",
        escape_html(&run.slug),
        escape_html(&run.profile),
        run.generated_at_unix_s,
        escape_html(&run.merged_report),
        headline,
        reports
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

fn write_json(path: &Path, value: &serde_json::Value) -> RunnerResult<()> {
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
        "Usage: ./scripts/kyuubiki build-standard-benchmark-index [--root tmp/standard-benchmark] [--retain 12]"
    );
}
