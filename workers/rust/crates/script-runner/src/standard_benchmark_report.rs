use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const DEFAULT_PROFILE: &str = "10k";
const DEFAULT_REPORTS_DIR: &str = "workers/rust/benchmarks/reports";
const DEFAULT_MATRICES: &[&str] = &["mechanical-core", "thermal-core", "compound-core"];

pub(crate) fn run_build_standard_benchmark_report(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(repo_root, args)?;
    let sections = options
        .matrices
        .iter()
        .map(|matrix| load_section(matrix, &options))
        .collect::<RunnerResult<Vec<_>>>()?;
    let report = render_report(repo_root, &options, &sections);
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(&options.output, report)
        .map_err(|error| format!("failed to write {}: {error}", options.output.display()))?;
    println!("{}", display_path(repo_root, &options.output));
    Ok(0)
}

struct Options {
    matrices: Vec<String>,
    output: PathBuf,
    profile: String,
    reports_dir: PathBuf,
}

struct Section {
    body: String,
    matrix: String,
    report_path: PathBuf,
}

fn parse_args(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut profile = DEFAULT_PROFILE.to_string();
    let mut reports_dir = repo_root.join(DEFAULT_REPORTS_DIR);
    let mut output = None;
    let mut matrices = DEFAULT_MATRICES
        .iter()
        .map(|matrix| (*matrix).to_string())
        .collect::<Vec<_>>();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                print_usage();
                return Ok(Options {
                    matrices,
                    output: repo_root
                        .join(DEFAULT_REPORTS_DIR)
                        .join("standard-10k-compare.md"),
                    profile,
                    reports_dir,
                });
            }
            "--profile" => profile = value_arg(&mut iter, "--profile")?,
            "--reports-dir" => reports_dir = path_arg(repo_root, &mut iter, "--reports-dir")?,
            "--output" => output = Some(path_arg(repo_root, &mut iter, "--output")?),
            "--matrices" => matrices = parse_matrices(&value_arg(&mut iter, "--matrices")?),
            other => return Err(format!("unknown option: {other}")),
        }
    }
    let output =
        output.unwrap_or_else(|| reports_dir.join(format!("standard-{profile}-compare.md")));
    Ok(Options {
        matrices,
        output,
        profile,
        reports_dir,
    })
}

fn load_section(matrix: &str, options: &Options) -> RunnerResult<Section> {
    let report_path = options
        .reports_dir
        .join(format!("{matrix}-{}-compare.md", options.profile));
    let content = fs::read_to_string(&report_path)
        .map_err(|error| format!("failed to read {}: {error}", report_path.display()))?;
    let lines = content.lines().collect::<Vec<_>>();
    let body_start = lines.iter().position(|line| line.starts_with("- Profile:"));
    let body = body_start
        .map(|index| lines[index..].join("\n"))
        .unwrap_or_else(|| content.trim().to_string())
        .trim()
        .to_string();
    Ok(Section {
        body,
        matrix: matrix.to_string(),
        report_path,
    })
}

fn render_report(repo_root: &Path, options: &Options, sections: &[Section]) -> String {
    let mut lines = vec![
        "# Kyuubiki Standard Benchmark Comparison".to_string(),
        String::new(),
        format!("- Profile: `{}`", options.profile),
        format!(
            "- Matrices: {}",
            options
                .matrices
                .iter()
                .map(|matrix| format!("`{matrix}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        format!(
            "- Reports directory: `{}`",
            display_path(repo_root, &options.reports_dir)
        ),
        String::new(),
        "## Included reports".to_string(),
        String::new(),
    ];
    for section in sections {
        lines.push(format!(
            "- `{}`: `{}`",
            section.matrix,
            display_path(repo_root, &section.report_path)
        ));
    }
    lines.push(String::new());
    for section in sections {
        lines.push(format!("## {}", section.matrix));
        lines.push(String::new());
        lines.push(section.body.clone());
        lines.push(String::new());
    }
    format!("{}\n", lines.join("\n").trim_end())
}

fn parse_matrices(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|matrix| !matrix.is_empty())
        .map(str::to_string)
        .collect()
}

fn value_arg(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))
}

fn path_arg(
    repo_root: &Path,
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> RunnerResult<PathBuf> {
    let value = PathBuf::from(value_arg(iter, flag)?);
    Ok(if value.is_absolute() {
        value
    } else {
        repo_root.join(value)
    })
}

fn display_path(root: &Path, path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(root) {
        return relative.to_string_lossy().replace('\\', "/");
    }
    relative_path(root, path)
        .unwrap_or_else(|| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
}

fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    let from_parts = normalized_components(from)?;
    let to_parts = normalized_components(to)?;
    let shared = from_parts
        .iter()
        .zip(to_parts.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut result = PathBuf::new();
    for _ in shared..from_parts.len() {
        result.push("..");
    }
    for part in &to_parts[shared..] {
        result.push(part);
    }
    Some(if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    })
}

fn normalized_components(path: &Path) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::RootDir => parts.push(String::new()),
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => return None,
            Component::Prefix(_) => return None,
        }
    }
    Some(parts)
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki build-standard-benchmark-report [options]\n\n\
Options:\n  --profile <name>       Benchmark profile, default 10k.\n  \
--reports-dir <path>   Directory containing matrix compare reports.\n  \
--output <path>        Output Markdown report.\n  \
--matrices <csv>       Comma-separated matrix list.\n  \
--help                 Show this message."
    );
}
