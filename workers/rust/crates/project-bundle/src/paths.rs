use std::path::{Path, PathBuf};

pub(crate) const PROJECT_EXTENSION: &str = "kyuubiki";

fn nonempty<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(trimmed)
    }
}

pub(crate) fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

pub(crate) fn existing_input(value: &str, label: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(nonempty(value, label)?);
    if !candidate.exists() {
        return Err(format!("{label} does not exist: {}", candidate.display()));
    }
    candidate
        .canonicalize()
        .map_err(|error| format!("failed to resolve {label} {}: {error}", candidate.display()))
}

pub(crate) fn existing_bundle(value: &str, label: &str) -> Result<PathBuf, String> {
    let path = existing_input(value, label)?;
    if !path.is_file() || !has_extension(&path, PROJECT_EXTENSION) {
        return Err(format!("{label} must point to a .kyuubiki project bundle"));
    }
    Ok(path)
}

pub(crate) fn existing_directory(value: &str, label: &str) -> Result<PathBuf, String> {
    let path = existing_input(value, label)?;
    if !path.is_dir() {
        return Err(format!("{label} must point to a directory"));
    }
    Ok(path)
}

pub(crate) fn output(value: &str, label: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(nonempty(value, label)?);
    let parent = candidate
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        return Err(format!(
            "{label} parent directory does not exist: {}",
            parent.display()
        ));
    }
    parent.canonicalize().map_err(|error| {
        format!(
            "failed to resolve {label} parent {}: {error}",
            parent.display()
        )
    })?;
    Ok(candidate)
}

pub(crate) fn new_bundle(value: &str) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    let mut candidate = if trimmed.is_empty() {
        unique_default_bundle_path(&default_bundle_root())
    } else {
        let supplied = PathBuf::from(trimmed);
        if !supplied.is_absolute() {
            return Err("new project bundle path must be absolute".to_string());
        }
        supplied
    };
    if candidate.extension().is_none() {
        candidate.set_extension(PROJECT_EXTENSION);
    }
    if !has_extension(&candidate, PROJECT_EXTENSION) {
        return Err("new project bundle path must end with .kyuubiki".to_string());
    }
    if candidate.exists() {
        return Err(format!(
            "refusing to overwrite existing project bundle: {}",
            candidate.display()
        ));
    }
    let parent = candidate
        .parent()
        .ok_or_else(|| format!("new project bundle has no parent: {}", candidate.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    Ok(candidate)
}

pub(crate) fn named_bundle(directory: &str, name: &str) -> Result<PathBuf, String> {
    nonempty(directory, "project directory")?;
    let directory = Path::new(directory);
    if !directory.is_absolute() || !directory.is_dir() {
        return Err("choose an existing absolute project directory".to_string());
    }
    let name = name.trim();
    let name = if name.to_ascii_lowercase().ends_with(".kyuubiki") {
        &name[..name.len() - ".kyuubiki".len()]
    } else {
        name
    };
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
        });
    if name.is_empty()
        || name.len() > 200
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|ch| ch.is_control() || "/\\:*?\"<>|".contains(ch))
        || reserved
    {
        return Err(
            "use a portable project name without path separators or reserved characters"
                .to_string(),
        );
    }
    Ok(directory.join(format!("{name}.kyuubiki")))
}

fn default_bundle_root() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("Documents")
        .join("Kyuubiki Projects")
}

fn unique_default_bundle_path(root: &Path) -> PathBuf {
    let first = root.join("Untitled.kyuubiki");
    if !first.exists() {
        return first;
    }
    for index in 2..10_000 {
        let candidate = root.join(format!("Untitled {index}.kyuubiki"));
        if !candidate.exists() {
            return candidate;
        }
    }
    root.join(format!("Untitled-{}.kyuubiki", uuid::Uuid::new_v4()))
}
