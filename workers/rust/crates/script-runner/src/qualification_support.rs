use serde::{Serialize, de::DeserializeOwned};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) type RunnerResult<T> = Result<T, String>;

#[derive(Default)]
pub(crate) struct QualificationOptions {
    pub(crate) out: Option<String>,
    pub(crate) verify_report: Option<String>,
    pub(crate) self_test: bool,
}

pub(crate) fn parse_options(
    args: Vec<OsString>,
    label: &str,
) -> RunnerResult<QualificationOptions> {
    let mut options = QualificationOptions::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--out" => options.out = Some(required_path(&mut iter, "--out")?),
            "--verify-report" => {
                options.verify_report = Some(required_path(&mut iter, "--verify-report")?)
            }
            "--self-test" => options.self_test = true,
            other => return Err(format!("unknown {label} argument: {other}")),
        }
    }
    if options.out.is_some() && options.verify_report.is_some() {
        return Err("--out and --verify-report cannot be combined".to_string());
    }
    Ok(options)
}

fn required_path(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a repository-relative path"))
}

pub(crate) fn generated_at_unix_ms() -> RunnerResult<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("system clock before epoch: {error}"))
}

pub(crate) fn repo_path(root: &Path, relative: &str) -> RunnerResult<PathBuf> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| component.as_os_str() == "..")
    {
        return Err(format!("path escapes repository: {relative}"));
    }
    Ok(root.join(path))
}

pub(crate) fn read_json<T: DeserializeOwned>(root: &Path, relative: &str) -> RunnerResult<T> {
    let text = fs::read_to_string(repo_path(root, relative)?)
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid JSON {relative}: {error}"))
}

pub(crate) fn write_json<T: Serialize>(root: &Path, relative: &str, value: &T) -> RunnerResult<()> {
    let path = repo_path(root, relative)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode qualification report: {error}"))?;
    fs::write(&path, format!("{rendered}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(crate) fn write_json_compact<T: Serialize>(
    root: &Path,
    relative: &str,
    value: &T,
) -> RunnerResult<()> {
    let path = repo_path(root, relative)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let rendered = serde_json::to_string(value)
        .map_err(|error| format!("failed to encode qualification report: {error}"))?;
    fs::write(&path, format!("{rendered}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(crate) fn combined_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

pub(crate) fn portable_output(root: &Path, output: &Output) -> String {
    portable_text(root, combined_output(output))
}

pub(crate) fn portable_text(root: &Path, rendered: String) -> String {
    let root = root.to_string_lossy();
    if root.is_empty() {
        rendered
    } else {
        rendered.replace(root.as_ref(), "@repo")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn removes_repository_paths_from_retained_output() {
        let output = super::portable_text(
            std::path::Path::new("/private/repo"),
            "Compiling (/private/repo/workers/rust)".to_string(),
        );
        assert_eq!(output, "Compiling (@repo/workers/rust)");
    }

    #[test]
    fn repository_paths_cannot_escape() {
        assert!(super::repo_path(std::path::Path::new("/repo"), "../secret").is_err());
    }
}
