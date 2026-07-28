use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const INVENTORIES: &[(&str, &str, &str)] = &[
    ("docs", "docs/README.md", "docs/README.md"),
    (
        "apps/hub-gui/ui/docs",
        "apps/hub-gui/ui/docs/README.md",
        "apps/hub-gui/ui/docs/README.md",
    ),
];

pub(crate) fn run_check_doc_inventory(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner check-doc-inventory");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-doc-inventory does not accept arguments".to_string());
    }

    let issues = inventory_issues(root)?;
    if !issues.is_empty() {
        eprintln!("documentation inventory check failed:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return Ok(1);
    }

    println!("documentation inventory ok");
    Ok(0)
}

fn inventory_issues(root: &Path) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    for (directory, index_path, label) in INVENTORIES {
        let index = read_text(root, index_path)?;
        let mut files = fs::read_dir(root.join(directory))
            .map_err(|error| format!("failed to read {directory}: {error}"))?
            .map(|entry| {
                entry
                    .map_err(|error| format!("failed to inspect {directory}: {error}"))
                    .map(|entry| entry.path())
            })
            .collect::<Result<Vec<_>, _>>()?;
        files.sort();

        for path in files {
            if !path.is_file() || path.file_name().is_some_and(|name| name == "README.md") {
                continue;
            }
            let supported = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension, "md" | "html" | "json"));
            if !supported {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("{directory}: documentation filename is not UTF-8"))?;
            if !index.contains(file_name) {
                issues.push(format!(
                    "{label}: missing inventory entry for {}/{}",
                    Path::new(directory)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(directory),
                    file_name
                ));
            }
        }
    }
    Ok(issues)
}

fn read_text(root: &Path, relative: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::inventory_issues;
    use std::path::PathBuf;

    #[test]
    fn retained_documentation_inventory_is_complete() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        let issues = inventory_issues(&root).expect("documentation inventory should load");
        assert!(issues.is_empty(), "{}", issues.join("\n"));
    }
}
