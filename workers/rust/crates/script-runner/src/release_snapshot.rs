use serde_json::json;
use std::collections::BTreeSet;
use std::ffi::OsString;

const RELEASE_FRONTEND_CHECKS: &[&str] = &[
    "npm run typecheck",
    "npm run build",
    "npm run check:workflow-preflight",
];
const RELEASE_REPO_CHECKS: &[&str] = &[
    "git diff --check",
    "make audit-project-organization",
    "make operator-package-preflight",
    "make check-operator-package-dynamic-smoke-contract",
    "make operator-package-dynamic-smoke",
    "make architecture-check",
];

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_create_release_snapshot(args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner create-release-snapshot --self-test");
        return Ok(0);
    }
    if args.iter().any(|arg| arg == "--self-test") {
        return run_self_test();
    }
    Err(
        "create-release-snapshot native runner currently supports --self-test; use the retained .mjs parity script for scaffold writes"
            .to_string(),
    )
}

fn run_self_test() -> RunnerResult<u8> {
    let mut issues = Vec::new();
    require_contains(
        RELEASE_FRONTEND_CHECKS,
        "npm run check:workflow-preflight",
        &mut issues,
    );
    for required in [
        "make audit-project-organization",
        "make operator-package-preflight",
        "make check-operator-package-dynamic-smoke-contract",
        "make operator-package-dynamic-smoke",
        "make architecture-check",
    ] {
        require_contains(RELEASE_REPO_CHECKS, required, &mut issues);
    }
    if unique_count(RELEASE_REPO_CHECKS) != RELEASE_REPO_CHECKS.len() {
        issues.push("release repo checks must not contain duplicates".to_string());
    }
    let source_issues = collect_source_version_issues(
        "1.20.0",
        &[
            VersionRow::new("ok.json", "version", "1.20.0"),
            VersionRow::new("brand.json", "releaseVersion", "1.20.0"),
            VersionRow::new("stale.json", "version", "1.17.8"),
        ],
    );
    let expected = vec![json!({
        "path": "stale.json",
        "field": "version",
        "actual": "1.17.8",
        "expected": "1.20.0",
    })];
    if source_issues != expected {
        issues.push("source version issue fixture did not match legacy shape".to_string());
    }
    if !issues.is_empty() {
        eprintln!("release snapshot self-test failed:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return Ok(1);
    }
    println!("release snapshot self-test passed");
    Ok(0)
}

fn require_contains(values: &[&str], expected: &str, issues: &mut Vec<String>) {
    if !values.contains(&expected) {
        issues.push(format!("missing required release check {expected}"));
    }
}

fn unique_count(values: &[&str]) -> usize {
    values.iter().copied().collect::<BTreeSet<_>>().len()
}

#[derive(Clone, Copy)]
struct VersionRow {
    path: &'static str,
    field: &'static str,
    actual: &'static str,
}

impl VersionRow {
    const fn new(path: &'static str, field: &'static str, actual: &'static str) -> Self {
        Self {
            path,
            field,
            actual,
        }
    }
}

fn collect_source_version_issues(expected: &str, rows: &[VersionRow]) -> Vec<serde_json::Value> {
    rows.iter()
        .filter(|row| row.actual != expected)
        .map(|row| {
            json!({
                "path": row.path,
                "field": row.field,
                "actual": row.actual,
                "expected": expected,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{VersionRow, collect_source_version_issues, unique_count};
    use serde_json::json;

    #[test]
    fn source_version_issues_match_legacy_fixture_shape() {
        assert_eq!(
            collect_source_version_issues(
                "1.20.0",
                &[
                    VersionRow::new("ok.json", "version", "1.20.0"),
                    VersionRow::new("stale.json", "version", "1.19.0"),
                ],
            ),
            vec![json!({
                "path": "stale.json",
                "field": "version",
                "actual": "1.19.0",
                "expected": "1.20.0",
            })]
        );
    }

    #[test]
    fn duplicate_detector_counts_unique_values() {
        assert_eq!(unique_count(&["a", "b", "a"]), 2);
    }
}
