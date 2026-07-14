use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_check_make_modules(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner check-make-modules");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-make-modules does not accept positional arguments".to_string());
    }

    let issues = check_make_modules(root)?;
    if !issues.is_empty() {
        eprintln!("make module check failed:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return Ok(1);
    }

    let include_count = include_paths(&read_text(&root.join("Makefile"))?).len();
    println!("make module check passed: {include_count} included module(s)");
    Ok(0)
}

fn check_make_modules(root: &Path) -> RunnerResult<Vec<String>> {
    let makefile = read_text(&root.join("Makefile"))?;
    let includes = include_paths(&makefile);
    let mut issues = Vec::new();

    if includes.first().map(String::as_str) != Some("make/help.mk") {
        issues.push(
            "Makefile: make/help.mk must be the first include so plain `make` shows help"
                .to_string(),
        );
    }

    for include_path in &includes {
        if !include_path.starts_with("make/") || !include_path.ends_with(".mk") {
            issues.push(format!(
                "Makefile: include {include_path} must point at make/*.mk"
            ));
            continue;
        }
        if !root.join(include_path).is_file() {
            issues.push(format!(
                "Makefile: included module does not exist: {include_path}"
            ));
        }
        if include_path == "make/targets.mk" {
            issues.push(
                "Makefile: make/targets.mk is retired; use narrow modules instead".to_string(),
            );
        }
    }

    for (index, line) in makefile.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if is_variable_assignment(trimmed) || trimmed.starts_with("include ") {
            continue;
        }
        issues.push(format!(
            "Makefile:{}: root Makefile should only define shared variables and includes",
            index + 1
        ));
    }

    for module in make_modules(root)? {
        let include_path = format!("make/{module}");
        if !includes.contains(&include_path) {
            issues.push(format!(
                "{include_path}: module is not included by root Makefile"
            ));
        }
    }

    Ok(issues)
}

fn include_paths(makefile: &str) -> Vec<String> {
    makefile
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("include "))
        .map(str::trim)
        .map(str::to_string)
        .collect()
}

fn make_modules(root: &Path) -> RunnerResult<Vec<String>> {
    let mut modules = Vec::new();
    let make_dir = root.join("make");
    for entry in fs::read_dir(&make_dir)
        .map_err(|error| format!("failed to read {}: {error}", make_dir.display()))?
    {
        let entry = entry.map_err(|error| format!("failed to read make module entry: {error}"))?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("mk") {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
            modules.push(name.to_string());
        }
    }
    modules.sort();
    Ok(modules)
}

fn is_variable_assignment(line: &str) -> bool {
    let Some(operator_index) = line.find('=') else {
        return false;
    };
    let left = line[..operator_index].trim_end();
    if left.is_empty() {
        return false;
    }
    let name = left.trim_end_matches([':', '?', '+']).trim_end();
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}

fn read_text(path: &Path) -> RunnerResult<String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

#[cfg(test)]
mod tests {
    use super::{include_paths, is_variable_assignment};

    #[test]
    fn extracts_trimmed_make_includes() {
        let includes = include_paths(
            "ROOT := .\ninclude make/help.mk\n include nope.mk\ninclude make/checks.mk\n",
        );

        assert_eq!(includes, vec!["make/help.mk", "nope.mk", "make/checks.mk"]);
    }

    #[test]
    fn recognizes_shared_make_variable_assignments() {
        assert!(is_variable_assignment("ROOT_DIR := $(CURDIR)"));
        assert!(is_variable_assignment("FOO ?= bar"));
        assert!(is_variable_assignment("BAR += baz"));
        assert!(!is_variable_assignment("check:"));
        assert!(!is_variable_assignment("\t@echo hello"));
    }
}
