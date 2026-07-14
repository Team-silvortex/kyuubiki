use serde_json::Value;
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_MAX_LINES: usize = 600;
const LOCKFILE_CONTRACT: &str = "config/dependency-audit-lockfiles.json";
const INSTALLER_TEST_INDEX: &str = "workers/rust/crates/installer/src/tests.rs";
const CHECKED_EXTENSIONS: &[&str] = &[
    "css", "ex", "exs", "html", "js", "json", "jsx", "md", "mk", "mjs", "py", "rs", "sh", "swift",
    "ts", "tsx", "zsh",
];

type RunnerResult<T> = Result<T, String>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Violation {
    relative_path: String,
    lines: usize,
    limit: String,
    custom_message: Option<String>,
    missing_debt_file: bool,
    debt_tracked: bool,
}

pub(crate) fn run_audit_project_organization(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test()?;
        println!("project organization audit self-test passed");
        return Ok(0);
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner audit-project-organization [--self-test]");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("audit-project-organization only accepts --self-test".to_string());
    }
    let max_lines = std::env::var("MAX_LINES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_LINES);
    let violations = audit(root, max_lines)?;
    if violations.is_empty() {
        println!(
            "Project organization audit passed. Default source limit {max_lines}; tracked debt 0."
        );
        return Ok(0);
    }
    eprintln!(
        "Project organization audit failed. Default source limit is {max_lines} lines.\n\n\
Split new oversized files, or lower existing tracked debt after refactoring.\n"
    );
    for violation in format_violations(&violations) {
        eprintln!("{violation}");
    }
    Ok(1)
}

fn audit(root: &Path, max_lines: usize) -> RunnerResult<Vec<Violation>> {
    let mut violations = Vec::new();
    for relative_path in project_files(root)? {
        if !should_check(&relative_path) {
            continue;
        }
        let lines = line_count_like_node(&read_text(root, &relative_path)?);
        if lines > max_lines {
            violations.push(Violation {
                relative_path,
                lines,
                limit: max_lines.to_string(),
                custom_message: None,
                missing_debt_file: false,
                debt_tracked: false,
            });
        }
    }
    violations.extend(installer_test_index_violations(
        INSTALLER_TEST_INDEX,
        &read_text_optional(root, INSTALLER_TEST_INDEX)?.unwrap_or_default(),
        &|module_path| root.join(module_path).exists(),
    ));
    violations.extend(required_audit_lockfile_violations(
        root,
        &required_audit_lockfiles(root)?,
        &|relative_path| root.join(relative_path).exists(),
        &|relative_path| git_check_ignored(root, relative_path),
    )?);
    Ok(violations)
}

fn project_files(root: &Path) -> RunnerResult<Vec<String>> {
    let mut files = BTreeSet::new();
    for args in [
        vec!["ls-files"],
        vec!["ls-files", "--others", "--exclude-standard"],
    ] {
        for file in git_lines(root, &args)? {
            if root.join(&file).exists() {
                files.insert(file);
            }
        }
    }
    Ok(files.into_iter().collect())
}

fn required_audit_lockfiles(root: &Path) -> RunnerResult<Vec<String>> {
    let contract = read_json(root, LOCKFILE_CONTRACT)?;
    if field(&contract, "schema") != "kyuubiki.dependency-audit-lockfiles/v1" {
        return Err(format!("{LOCKFILE_CONTRACT}: unexpected schema"));
    }
    let mut files = Vec::new();
    for key in ["npm", "cargo"] {
        let Some(entries) = contract.get(key).and_then(Value::as_array) else {
            return Err(format!("{LOCKFILE_CONTRACT}: {key} must be an array"));
        };
        let suffix = if key == "npm" {
            "package-lock.json"
        } else {
            "Cargo.lock"
        };
        files.extend(
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(|dir| format!("{dir}/{suffix}")),
        );
    }
    Ok(files)
}

fn required_audit_lockfile_violations(
    root: &Path,
    paths: &[String],
    exists: &dyn Fn(&str) -> bool,
    ignored: &dyn Fn(&str) -> RunnerResult<bool>,
) -> RunnerResult<Vec<Violation>> {
    let mut violations = Vec::new();
    for relative_path in paths {
        if !exists(relative_path) {
            violations.push(custom_violation(
                relative_path,
                "dependency-audit-lockfile-required",
                "dependency audit lockfile is required for reproducible security checks",
            ));
            continue;
        }
        if ignored(relative_path)? {
            violations.push(custom_violation(
                relative_path,
                "dependency-audit-lockfile-not-ignored",
                "dependency audit lockfile must not be ignored by git",
            ));
        }
    }
    let _ = root;
    Ok(violations)
}

fn installer_test_index_violations(
    relative_path: &str,
    contents: &str,
    module_exists: &dyn Fn(&str) -> bool,
) -> Vec<Violation> {
    if contents.is_empty() {
        return Vec::new();
    }
    for (index, line) in contents.split('\n').enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let Some(module_name) = module_declaration(trimmed) else {
            return vec![custom_line_violation(
                relative_path,
                index + 1,
                "module-index-only",
                "installer tests.rs should only declare test modules; put tests in workers/rust/crates/installer/src/tests/",
            )];
        };
        let module_path = format!("workers/rust/crates/installer/src/tests/{module_name}.rs");
        if !module_exists(&module_path) {
            return vec![custom_line_violation(
                relative_path,
                index + 1,
                "module-file-required",
                &format!("installer test module {module_name} is missing {module_path}"),
            )];
        }
    }
    Vec::new()
}

fn module_declaration(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("mod ")?;
    let name = rest.strip_suffix(';')?;
    if name.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    }) {
        Some(name)
    } else {
        None
    }
}

fn should_check(relative_path: &str) -> bool {
    let extension = Path::new(relative_path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    CHECKED_EXTENSIONS.contains(&extension) && !is_ignored_path(relative_path)
}

fn is_ignored_path(relative_path: &str) -> bool {
    relative_path.contains("/gen/schemas/")
        || relative_path.starts_with("gen/schemas/")
        || relative_path.ends_with("/package-lock.json")
        || relative_path == "package-lock.json"
        || relative_path.starts_with("assets/icons/")
        || relative_path == "releases/update-catalog.json"
        || relative_path.starts_with("apps/frontend/public/")
        || is_app_ui_asset(relative_path)
        || is_app_tauri_icon(relative_path)
        || (relative_path.starts_with("workers/rust/benchmarks/")
            && relative_path.ends_with(".json"))
}

fn is_app_ui_asset(relative_path: &str) -> bool {
    let parts = relative_path.split('/').collect::<Vec<_>>();
    parts.len() >= 5 && parts[0] == "apps" && parts[2] == "ui" && parts[3] == "assets"
}

fn is_app_tauri_icon(relative_path: &str) -> bool {
    let parts = relative_path.split('/').collect::<Vec<_>>();
    parts.len() >= 5 && parts[0] == "apps" && parts[2] == "src-tauri" && parts[3] == "icons"
}

fn format_violations(violations: &[Violation]) -> Vec<String> {
    let mut sorted = violations.to_vec();
    sorted.sort_by(|left, right| right.lines.cmp(&left.lines));
    sorted
        .iter()
        .map(|violation| {
            if violation.missing_debt_file {
                return format!(
                    "{}: debt guard references a missing file",
                    violation.relative_path
                );
            }
            if let Some(message) = &violation.custom_message {
                return format!("{}: {message}", violation.relative_path);
            }
            format!(
                "{}: {} lines (limit {})",
                violation.relative_path, violation.lines, violation.limit
            )
        })
        .collect()
}

fn custom_violation(relative_path: &str, limit: &str, message: &str) -> Violation {
    custom_line_violation(relative_path, 0, limit, message)
}

fn custom_line_violation(
    relative_path: &str,
    lines: usize,
    limit: &str,
    message: &str,
) -> Violation {
    Violation {
        relative_path: relative_path.to_string(),
        lines,
        limit: limit.to_string(),
        custom_message: Some(message.to_string()),
        missing_debt_file: false,
        debt_tracked: false,
    }
}

fn git_lines(root: &Path, args: &[&str]) -> RunnerResult<Vec<String>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("git {} failed: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn git_check_ignored(root: &Path, relative_path: &str) -> RunnerResult<bool> {
    let output = Command::new("git")
        .args(["check-ignore", "-q", relative_path])
        .current_dir(root)
        .output()
        .map_err(|error| format!("git check-ignore failed for {relative_path}: {error}"))?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "git check-ignore failed for {relative_path}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn read_text_optional(root: &Path, relative_path: &str) -> RunnerResult<Option<String>> {
    let path = root.join(relative_path);
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn line_count_like_node(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.split('\n').count()
    }
}

fn run_self_test() -> RunnerResult<()> {
    let modules = HashSet::<String>::from([
        "workers/rust/crates/installer/src/tests/control_update.rs".to_string(),
        "workers/rust/crates/installer/src/tests/security_integrity.rs".to_string(),
    ]);
    let module_exists = |path: &str| modules.contains(path);
    if !installer_test_index_violations(
        INSTALLER_TEST_INDEX,
        "mod control_update;\nmod security_integrity;\n",
        &module_exists,
    )
    .is_empty()
    {
        return Err("self-test expected valid installer module index".to_string());
    }
    assert_limit(
        installer_test_index_violations(
            INSTALLER_TEST_INDEX,
            "#[test]\nfn inline_test() {}\n",
            &module_exists,
        ),
        "module-index-only",
    )?;
    assert_limit(
        installer_test_index_violations(
            INSTALLER_TEST_INDEX,
            "mod missing_module;\n",
            &module_exists,
        ),
        "module-file-required",
    )?;
    let expected_lockfiles = [
        "apps/frontend/package-lock.json",
        "apps/hub-gui/package-lock.json",
        "apps/installer-gui/package-lock.json",
        "apps/workbench-gui/package-lock.json",
        "workers/rust/Cargo.lock",
        "sdks/rust/Cargo.lock",
        "apps/hub-gui/src-tauri/Cargo.lock",
        "apps/installer-gui/src-tauri/Cargo.lock",
        "apps/workbench-gui/src-tauri/Cargo.lock",
    ];
    let root = PathBuf::from(".");
    let missing = required_audit_lockfile_violations(
        &root,
        &["missing.lock".to_string()],
        &|_| false,
        &|_| Ok(false),
    )?;
    assert_limit(missing, "dependency-audit-lockfile-required")?;
    let ignored = required_audit_lockfile_violations(
        &root,
        &["ignored.lock".to_string()],
        &|_| true,
        &|_| Ok(true),
    )?;
    assert_limit(ignored, "dependency-audit-lockfile-not-ignored")?;
    let ok =
        required_audit_lockfile_violations(&root, &["ok.lock".to_string()], &|_| true, &|_| {
            Ok(false)
        })?;
    if !ok.is_empty() {
        return Err("self-test expected ok lockfile to pass".to_string());
    }
    if expected_lockfiles.len() != 9 {
        return Err("self-test lockfile fixture drifted".to_string());
    }
    Ok(())
}

fn assert_limit(violations: Vec<Violation>, expected: &str) -> RunnerResult<()> {
    if violations.first().map(|violation| violation.limit.as_str()) == Some(expected) {
        Ok(())
    } else {
        Err(format!("self-test expected violation limit {expected}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{installer_test_index_violations, line_count_like_node, module_declaration};

    #[test]
    fn line_count_matches_node_split_shape() {
        assert_eq!(line_count_like_node(""), 0);
        assert_eq!(line_count_like_node("one"), 1);
        assert_eq!(line_count_like_node("one\n"), 2);
    }

    #[test]
    fn module_declaration_accepts_simple_test_modules() {
        assert_eq!(
            module_declaration("mod control_update;"),
            Some("control_update")
        );
        assert_eq!(module_declaration("#[test]"), None);
    }

    #[test]
    fn installer_index_rejects_inline_tests() {
        let violations = installer_test_index_violations(
            "workers/rust/crates/installer/src/tests.rs",
            "#[test]\nfn inline_test() {}\n",
            &|_| true,
        );
        assert_eq!(violations[0].limit, "module-index-only");
    }
}
