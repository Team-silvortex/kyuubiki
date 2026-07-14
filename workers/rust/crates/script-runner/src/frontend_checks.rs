use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const MAX_FRONTEND_LINES: usize = 600;
const ALLOWED_SENSITIVE_STORAGE_LINE: &str = "src/lib/workbench/helpers.ts:const rawSecrets = window.sessionStorage.getItem(WORKBENCH_SECRETS_KEY);";

pub(crate) fn run_frontend_file_lines(
    frontend_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner frontend-file-lines");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("frontend-file-lines does not accept positional arguments".to_string());
    }

    let mut violations = Vec::new();
    for file_path in frontend_source_files(frontend_root)? {
        let contents = read_text(&file_path)?;
        let lines = if contents.is_empty() {
            0
        } else {
            contents.split('\n').count()
        };
        if lines > MAX_FRONTEND_LINES {
            violations.push((relative_path(frontend_root, &file_path), lines));
        }
    }

    if !violations.is_empty() {
        violations.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        eprintln!("File line-count guard failed. Default limit is {MAX_FRONTEND_LINES} lines.\n");
        eprintln!("Existing oversized files must be split before they grow further.\n");
        for (relative, lines) in violations {
            eprintln!("{relative}: {lines} lines (limit {MAX_FRONTEND_LINES})");
        }
        return Ok(1);
    }

    println!("File line-count guard passed. Default limit {MAX_FRONTEND_LINES}");
    Ok(0)
}

pub(crate) fn run_frontend_storage_security(
    frontend_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner frontend-storage-security");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("frontend-storage-security does not accept positional arguments".to_string());
    }

    let mut violations = Vec::new();
    for file_path in frontend_source_files(frontend_root)? {
        let relative = relative_path(frontend_root, &file_path);
        for (index, line) in read_text(&file_path)?.lines().enumerate() {
            if !contains_storage_call(line) || !contains_sensitive_word(line) {
                continue;
            }
            let stable_key = format!("{relative}:{}", line.trim());
            if stable_key == ALLOWED_SENSITIVE_STORAGE_LINE {
                continue;
            }
            violations.push(format!("{}:{}: {}", relative, index + 1, line.trim()));
        }
    }

    if !violations.is_empty() {
        eprintln!("Storage security guard failed.");
        eprintln!(
            "Do not read or write token, password, credential, API key, or secret-shaped values through browser storage."
        );
        eprintln!(
            "Use in-memory workbench secrets or a platform credential vault boundary instead.\n"
        );
        for violation in violations {
            eprintln!("{violation}");
        }
        return Ok(1);
    }

    println!("Storage security guard passed. Sensitive browser storage calls were not found.");
    Ok(0)
}

fn frontend_source_files(frontend_root: &Path) -> RunnerResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_source_files(&frontend_root.join("src"), &mut files)?;
    Ok(files)
}

fn collect_source_files(dir: &Path, files: &mut Vec<PathBuf>) -> RunnerResult<()> {
    for entry in
        fs::read_dir(dir).map_err(|error| format!("failed to read {}: {error}", dir.display()))?
    {
        let entry = entry.map_err(|error| format!("failed to read directory entry: {error}"))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("failed to stat {}: {error}", path.display()))?;
        if metadata.is_dir() {
            collect_source_files(&path, files)?;
        } else if metadata.is_file() && is_frontend_source(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_frontend_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "ts" || extension == "tsx")
}

fn read_text(path: &Path) -> RunnerResult<String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

fn contains_storage_call(line: &str) -> bool {
    [
        "localStorage.setItem",
        "localStorage.getItem",
        "sessionStorage.setItem",
        "sessionStorage.getItem",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

fn contains_sensitive_word(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "api-key",
        "api_key",
        "apikey",
        "authorization",
        "bearer",
        "credential",
        "password",
        "passwd",
        "secret",
        "session-key",
        "session_key",
        "sessionkey",
        "token",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::{
        ALLOWED_SENSITIVE_STORAGE_LINE, contains_sensitive_word, contains_storage_call,
        is_frontend_source,
    };
    use std::path::Path;

    #[test]
    fn frontend_source_filter_accepts_ts_and_tsx_only() {
        assert!(is_frontend_source(Path::new("src/app/page.tsx")));
        assert!(is_frontend_source(Path::new("src/lib/helpers.ts")));
        assert!(!is_frontend_source(Path::new("src/app/style.css")));
        assert!(!is_frontend_source(Path::new("src/app/page.mjs")));
    }

    #[test]
    fn storage_security_matches_browser_storage_and_secret_words() {
        assert!(contains_storage_call(
            "window.localStorage.setItem(KEY, token);"
        ));
        assert!(contains_storage_call("window.sessionStorage.getItem(KEY);"));
        assert!(contains_sensitive_word("const token = value;"));
        assert!(contains_sensitive_word("assistantApiKey"));
        assert!(!contains_sensitive_word("const theme = value;"));
    }

    #[test]
    fn legacy_scrub_line_stays_explicitly_allowlisted() {
        assert_eq!(
            ALLOWED_SENSITIVE_STORAGE_LINE,
            "src/lib/workbench/helpers.ts:const rawSecrets = window.sessionStorage.getItem(WORKBENCH_SECRETS_KEY);"
        );
    }
}
