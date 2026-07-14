use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

const CHECKED_EXTENSIONS: &[&str] = &[
    "css", "ex", "exs", "html", "js", "json", "jsx", "md", "mjs", "py", "rs", "service", "sh",
    "swift", "toml", "ts", "tsx", "yaml", "yml", "zsh",
];
const IGNORED_PATH_FRAGMENTS: &[&str] = &[
    "/node_modules/",
    "/target/",
    "/_build/",
    "/.next/",
    "/dist/",
    "/build/",
];
const LOCAL_PATH_PATTERNS: &[&str] = &[
    "/Users/",
    "/Users/Shared/chroot/dev/kyuubiki",
    "/var/folders/",
    "/private/var/",
    "/Volumes/",
    "/home/example-user/",
    "/opt/homebrew/",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    relative_path: String,
    line_number: usize,
    line: String,
}

pub(crate) fn run_audit_local_paths(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner audit-local-paths");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("audit-local-paths does not accept positional arguments".to_string());
    }

    let violations = audit_local_paths(root)?;
    if !violations.is_empty() {
        eprintln!("Local absolute path audit failed:");
        for violation in violations {
            eprintln!(
                "{}:{}: {}",
                violation.relative_path, violation.line_number, violation.line
            );
        }
        return Ok(1);
    }

    println!("Local absolute path audit passed.");
    Ok(0)
}

fn audit_local_paths(root: &Path) -> RunnerResult<Vec<Violation>> {
    let mut violations = Vec::new();
    for relative_path in git_files(root)? {
        if !should_check(&relative_path) {
            continue;
        }
        let absolute_path = root.join(&relative_path);
        if !absolute_path.is_file() {
            continue;
        }
        let contents = fs::read_to_string(&absolute_path)
            .map_err(|error| format!("failed to read {}: {error}", absolute_path.display()))?;
        for (index, line) in contents.lines().enumerate() {
            if contains_local_path(line) {
                violations.push(Violation {
                    relative_path: relative_path.clone(),
                    line_number: index + 1,
                    line: line.trim().to_string(),
                });
            }
        }
    }
    Ok(violations)
}

fn git_files(root: &Path) -> RunnerResult<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to run git ls-files: {error}"))?;
    if !output.status.success() {
        return Err("git ls-files failed".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn should_check(relative_path: &str) -> bool {
    let normalized = format!("/{relative_path}");
    if IGNORED_PATH_FRAGMENTS
        .iter()
        .any(|fragment| normalized.contains(fragment))
    {
        return false;
    }
    PathBuf::from(relative_path)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| CHECKED_EXTENSIONS.contains(&extension))
}

fn contains_local_path(line: &str) -> bool {
    LOCAL_PATH_PATTERNS
        .iter()
        .any(|pattern| line.contains(pattern))
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

#[cfg(test)]
mod tests {
    use super::{contains_local_path, should_check};

    #[test]
    fn checks_text_like_project_files_only() {
        assert!(should_check("docs/book.html"));
        assert!(should_check(
            "workers/rust/crates/script-runner/src/main.rs"
        ));
        assert!(!should_check("apps/frontend/node_modules/pkg/index.js"));
        assert!(!should_check("workers/rust/target/debug/build.rs"));
        assert!(!should_check("assets/logo.png"));
    }

    #[test]
    fn detects_local_machine_paths() {
        assert!(contains_local_path("open /Users/alice/project"));
        assert!(contains_local_path("cache at /var/folders/abc"));
        assert!(contains_local_path("brew at /opt/homebrew/bin"));
        assert!(!contains_local_path("repo-relative docs/book.html"));
    }
}
