use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", ".next", "dist", "build"];
const SCAN_ROOTS: &[&str] = &[
    "README.md",
    "docs",
    "apps",
    "deploy",
    "releases",
    "workers",
    "sdks",
    "scripts",
];
const ALLOWED_EXTENSIONS: &[&str] = &[
    "md", "html", "json", "js", "mjs", "ts", "tsx", "zsh", "cmd", "toml", "ex", "exs",
];

type RunnerResult<T> = Result<T, String>;

pub(crate) fn search_inventory(
    root: &Path,
    expected: &str,
    codename: &str,
) -> RunnerResult<Vec<Value>> {
    let minor = version_minor_line(expected);
    let patterns = [
        ("exact_version", expected.to_string()),
        ("minor_line", minor.clone()),
        ("display_version", version_display(codename, expected)),
        ("display_minor_line", version_display(codename, &minor)),
    ];
    let mut files = Vec::new();
    for scan_root in SCAN_ROOTS {
        let absolute = root.join(scan_root);
        if !absolute.exists() {
            continue;
        }
        if absolute.is_dir() {
            walk(root, scan_root, &mut files)?;
        } else {
            files.push((*scan_root).to_string());
        }
    }
    files.sort();
    let mut inventory = Vec::new();
    for file in files {
        if file.ends_with("package-lock.json") || !is_allowed_inventory_file(&file) {
            continue;
        }
        let text = read_text(root, &file)?;
        let hits = patterns
            .iter()
            .filter_map(|(label, value)| {
                let count = text.matches(value).count();
                (count > 0).then(|| json!({ "label": label, "value": value, "count": count }))
            })
            .collect::<Vec<_>>();
        if !hits.is_empty() {
            inventory.push(json!({ "file": file, "hits": hits }));
        }
    }
    Ok(inventory)
}

pub(crate) fn next_version_candidates(
    root: &Path,
    expected: &str,
    next: Option<&str>,
    codename: &str,
) -> RunnerResult<Vec<Value>> {
    let Some(next) = next else {
        return Ok(Vec::new());
    };
    let current_display = version_display(codename, expected);
    let current_minor = version_minor_line(expected);
    let next_minor = version_minor_line(next);
    Ok(search_inventory(root, expected, codename)?
        .into_iter()
        .filter(|entry| field(entry, "file") != format!("releases/snapshots/{expected}.json"))
        .map(|entry| {
            json!({
                "file": field(&entry, "file"),
                "hits": entry.get("hits").cloned().unwrap_or_else(|| json!([])),
                "suggested_replacements": [
                    { "from": expected, "to": next },
                    { "from": current_display, "to": version_display(codename, next) },
                    { "from": current_minor, "to": next_minor },
                ],
            })
        })
        .collect())
}

fn walk(root: &Path, relative: &str, results: &mut Vec<String>) -> RunnerResult<()> {
    let mut entries = fs::read_dir(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        let next = format!("{relative}/{name}");
        if entry.path().is_dir() {
            walk(root, &next, results)?;
        } else {
            results.push(next);
        }
    }
    Ok(())
}

fn is_allowed_inventory_file(relative: &str) -> bool {
    let path = PathBuf::from(relative);
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ALLOWED_EXTENSIONS.contains(&ext))
        || relative.ends_with("README")
}

fn read_text(root: &Path, relative: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn version_minor_line(version: &str) -> String {
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() < 2 {
        format!("{version}.x")
    } else {
        format!("{}.{}.x", parts[0], parts[1])
    }
}

fn version_display(codename: &str, version: &str) -> String {
    format!("{codename} {version}")
}
