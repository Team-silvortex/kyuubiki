use serde::Serialize;
use serde_json::json;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const DEFAULT_ROOT: &str = "workers/rust/crates";
const DEFAULT_MAX_LINES: usize = 800;
const IGNORED_DIRS: &[&str] = &[".git", "target", "node_modules", ".next", "dist", "build"];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    root: PathBuf,
    max_lines: usize,
    json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FileLineCount {
    file: String,
    lines: usize,
}

pub(crate) fn run_rust_line_audit(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    let result = audit(repo_root, &options)?;

    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "root": options.root.to_string_lossy(),
                "maxLines": options.max_lines,
                "checkedFiles": result.files.len(),
                "maximum": result.maximum(),
                "violations": result.violations,
            }))
            .expect("rust line audit result should serialize")
        );
    } else if result.violations.is_empty() {
        let maximum = result.maximum();
        println!(
            "Rust line-count audit passed: {} files, max {}/{} lines ({}).",
            result.files.len(),
            maximum.as_ref().map_or(0, |entry| entry.lines),
            options.max_lines,
            maximum
                .as_ref()
                .map_or_else(|| "none".to_string(), |entry| entry.file.clone())
        );
    } else {
        eprintln!(
            "Rust line-count audit failed: {} file(s) exceed {} lines.",
            result.violations.len(),
            options.max_lines
        );
        for violation in &result.violations {
            eprintln!("- {} {}", violation.lines, violation.file);
        }
    }

    Ok(if result.violations.is_empty() { 0 } else { 1 })
}

struct AuditResult {
    files: Vec<FileLineCount>,
    violations: Vec<FileLineCount>,
}

impl AuditResult {
    fn maximum(&self) -> Option<FileLineCount> {
        self.files.first().cloned()
    }
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            root: PathBuf::from(DEFAULT_ROOT),
            max_lines: DEFAULT_MAX_LINES,
            json: false,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            let arg = arg
                .into_string()
                .map_err(|_| "rust-line-audit arguments must be utf-8".to_string())?;
            match arg.as_str() {
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                "--json" => options.json = true,
                "--root" => {
                    options.root = PathBuf::from(next_arg(&mut iter, "--root")?);
                }
                "--max" => {
                    let value = next_arg(&mut iter, "--max")?;
                    options.max_lines = value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid --max value: {value}"))?;
                    if options.max_lines == 0 {
                        return Err(format!("Invalid --max value: {value}"));
                    }
                }
                other => return Err(format!("Unknown argument: {other}")),
            }
        }
        Ok(options)
    }
}

fn audit(repo_root: &Path, options: &Options) -> RunnerResult<AuditResult> {
    let mut files = Vec::new();
    collect_rust_files(repo_root, &options.root, &mut files)?;
    files.sort_by(|left, right| {
        right
            .lines
            .cmp(&left.lines)
            .then_with(|| left.file.cmp(&right.file))
    });
    let violations = files
        .iter()
        .filter(|entry| entry.lines > options.max_lines)
        .cloned()
        .collect();
    Ok(AuditResult { files, violations })
}

fn collect_rust_files(
    repo_root: &Path,
    relative_root: &Path,
    files: &mut Vec<FileLineCount>,
) -> RunnerResult<()> {
    let absolute_root = repo_root.join(relative_root);
    for entry in fs::read_dir(&absolute_root)
        .map_err(|error| format!("failed to read {}: {error}", absolute_root.display()))?
    {
        let entry = entry.map_err(|error| format!("failed to read rust source entry: {error}"))?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        if IGNORED_DIRS.contains(&file_name.as_str()) {
            continue;
        }
        let next_relative = relative_root.join(&file_name);
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("failed to stat {}: {error}", path.display()))?;
        if metadata.is_dir() {
            collect_rust_files(repo_root, &next_relative, files)?;
        } else if metadata.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("rs")
        {
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            files.push(FileLineCount {
                file: next_relative
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
                lines: line_count(&text),
            });
        }
    }
    Ok(())
}

fn line_count(text: &str) -> usize {
    if text.is_empty() {
        0
    } else if text.ends_with('\n') {
        text.split('\n').count() - 1
    } else {
        text.split('\n').count()
    }
}

fn next_arg(iter: &mut impl Iterator<Item = OsString>, name: &str) -> RunnerResult<String> {
    iter.next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(|| format!("{name} requires a value"))
}

fn print_usage() {
    println!(
        "Usage:\n  kyuubiki-script-runner rust-line-audit [--root workers/rust/crates] [--max 800] [--json]"
    );
}

#[cfg(test)]
mod tests {
    use super::{Options, line_count};
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn counts_lines_like_existing_node_audit() {
        assert_eq!(line_count(""), 0);
        assert_eq!(line_count("one"), 1);
        assert_eq!(line_count("one\n"), 1);
        assert_eq!(line_count("one\ntwo"), 2);
    }

    #[test]
    fn parses_audit_options() {
        let options = Options::parse(vec![
            OsString::from("--root"),
            OsString::from("workers/rust"),
            OsString::from("--max"),
            OsString::from("500"),
            OsString::from("--json"),
        ])
        .expect("options should parse");

        assert_eq!(options.root, PathBuf::from("workers/rust"));
        assert_eq!(options.max_lines, 500);
        assert!(options.json);
    }
}
