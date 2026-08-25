use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

mod inventory;
mod report;

use inventory::{next_version_candidates, search_inventory};
use report::print_human_report;

const DEFAULT_CODENAME: &str = "moxi";
const VERSION_POLICY_PATH: &str = "config/version-line-policy.json";
const VERSION_POLICY_SCHEMA: &str = "kyuubiki.version-line-policy/v1";
const EXACT_VERSION_FILES: &[&str] = &[
    "assets/brand/brand.json",
    "apps/frontend/package.json",
    "apps/frontend/public/brand.json",
    "apps/hub-gui/package.json",
    "apps/hub-gui/src-tauri/Cargo.toml",
    "apps/hub-gui/src-tauri/tauri.conf.json",
    "apps/hub-gui/ui/assets/brand.json",
    "apps/workbench-gui/package.json",
    "apps/workbench-gui/src-tauri/Cargo.toml",
    "apps/workbench-gui/src-tauri/tauri.conf.json",
    "apps/workbench-gui/ui/assets/brand.json",
    "apps/installer-gui/package.json",
    "apps/installer-gui/src-tauri/Cargo.toml",
    "apps/installer-gui/src-tauri/tauri.conf.json",
    "apps/installer-gui/ui/assets/brand.json",
    "apps/web/mix.exs",
    "workers/rust/Cargo.toml",
];
const PACKAGE_LOCK_FILES: &[&str] = &[
    "apps/frontend/package-lock.json",
    "apps/hub-gui/package-lock.json",
    "apps/workbench-gui/package-lock.json",
    "apps/installer-gui/package-lock.json",
];
const ARCHITECTURE_VERSION_LINE_FILES: &[&str] = &[
    "config/architecture/module-topology.json",
    "config/architecture/module-function-coverage-matrix.json",
];

type RunnerResult<T> = Result<T, String>;

struct Options {
    expected: Option<String>,
    next: Option<String>,
    codename: String,
    json: bool,
    self_test: bool,
}

pub(crate) fn run_audit_version_line(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = parse_args(args)?;
    if options.self_test {
        return run_self_test();
    }
    let expected = options.expected.unwrap_or(current_release_version(root)?);
    let development = current_development_version(root)?;
    let exact_checks = exact_checks(root, &expected, &options.codename)?;
    let failed = exact_checks
        .iter()
        .filter(|check| check.get("ok").and_then(Value::as_bool) != Some(true))
        .count();
    let report = json!({
        "codename": options.codename,
        "expected": expected,
        "current_development_version": development,
        "next_version": options.next,
        "exact_checks": exact_checks,
        "reference_inventory": search_inventory(root, &expected, &options.codename)?,
        "next_candidates": next_version_candidates(root, &expected, options.next.as_deref(), &options.codename)?,
    });
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to encode version line report: {error}"))?
        );
    } else {
        print_human_report(&report);
    }
    Ok(if failed == 0 { 0 } else { 1 })
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        expected: None,
        next: None,
        codename: DEFAULT_CODENAME.to_string(),
        json: false,
        self_test: false,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner audit-version-line [--expected 2.0.0] [--next 2.0.1] [--codename moxi] [--json] [--self-test]"
                );
                return Ok(options);
            }
            "--json" => options.json = true,
            "--self-test" => options.self_test = true,
            "--expected" => options.expected = Some(required_value(&mut iter, "--expected")?),
            "--next" => options.next = Some(required_value(&mut iter, "--next")?),
            "--codename" => options.codename = required_value(&mut iter, "--codename")?,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

fn required_value(iter: &mut impl Iterator<Item = OsString>, name: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{name} requires a value"))
}

fn current_release_version(root: &Path) -> RunnerResult<String> {
    Ok(field(&read_json(root, "releases/index.json")?, "current_version").to_string())
}

fn exact_checks(root: &Path, expected: &str, codename: &str) -> RunnerResult<Vec<Value>> {
    let minor = version_minor_line(expected);
    let development_version = current_development_version(root)?;
    let development_minor = version_minor_line(&development_version);
    let mut checks = Vec::new();
    for file in EXACT_VERSION_FILES {
        if file.ends_with("package.json") || file.ends_with("tauri.conf.json") {
            checks.push(check(
                "version",
                file,
                "version",
                expected,
                json_field(root, file, "version")?,
            ));
        } else if file.ends_with("brand.json") {
            checks.push(check(
                "releaseVersion",
                file,
                "releaseVersion",
                expected,
                json_field(root, file, "releaseVersion")?,
            ));
        } else if file.ends_with("Cargo.toml") {
            checks.push(check(
                "version",
                file,
                "version",
                expected,
                cargo_version(root, file)?,
            ));
        } else if file.ends_with("mix.exs") {
            checks.push(check(
                "version",
                file,
                "version",
                expected,
                mix_version(root, file)?,
            ));
        } else {
            checks.push(check(
                "minor_line",
                file,
                "version",
                &minor,
                json_field(root, file, "version")?,
            ));
        }
    }
    for file in PACKAGE_LOCK_FILES {
        let lock = read_json(root, file)?;
        checks.push(check(
            "version",
            file,
            "version",
            expected,
            string_value(&lock, "version"),
        ));
        checks.push(check(
            "root_package_version",
            file,
            "packages[\"\"].version",
            expected,
            lock.get("packages")
                .and_then(|packages| packages.get(""))
                .and_then(|package| package.get("version"))
                .cloned()
                .unwrap_or(Value::Null),
        ));
    }
    let channels = read_json(root, "deploy/update-channels.json")?;
    let contract = read_json(root, "deploy/installation-integrity-contract.json")?;
    let release_index = read_json(root, "releases/index.json")?;
    checks.push(check(
        "shipping_version",
        "deploy/update-channels.json",
        "shipping_version",
        expected,
        string_value(&channels, "shipping_version"),
    ));
    checks.push(check(
        "stable_channel_version",
        "deploy/update-channels.json",
        "channels[stable].version",
        expected,
        stable_channel_version(&channels),
    ));
    checks.push(check(
        "shipping_version",
        "deploy/installation-integrity-contract.json",
        "shipping_version",
        expected,
        string_value(&contract, "shipping_version"),
    ));
    checks.push(check(
        "shipping_version",
        "deploy/install-update-disk-hygiene.json",
        "shipping_version",
        expected,
        json_field(
            root,
            "deploy/install-update-disk-hygiene.json",
            "shipping_version",
        )?,
    ));
    checks.push(check(
        "required_version",
        "deploy/installation-integrity-contract.json",
        "visible_rules[required packaged version].value",
        expected,
        required_version_rule(&contract),
    ));
    checks.push(check(
        "current_version",
        "releases/index.json",
        "current_version",
        expected,
        string_value(&release_index, "current_version"),
    ));
    checks.push(check(
        "current_development_version",
        "docs/book-manifest.json",
        "current_development_version",
        &development_version,
        json_field(
            root,
            "docs/book-manifest.json",
            "current_development_version",
        )?,
    ));
    checks.push(check(
        "current_minor_line",
        "docs/ui-automation-contract.json",
        "version",
        &development_minor,
        json_field(root, "docs/ui-automation-contract.json", "version")?,
    ));
    let architecture_line = version_display(codename, &development_minor);
    for file in ARCHITECTURE_VERSION_LINE_FILES {
        checks.push(check(
            "architecture_version_line",
            file,
            "version_line",
            &architecture_line,
            json_field(root, file, "version_line")?,
        ));
    }
    let formal_release_target =
        json_field(root, "docs/book-manifest.json", "formal_release_target")?;
    let transition_line = format!(
        "{architecture_line} -> {}",
        formal_release_target.as_str().unwrap_or_default()
    );
    checks.push(check(
        "architecture_transition_line",
        "config/architecture/module-function-coverage-tensor.json",
        "version_line",
        &transition_line,
        json_field(
            root,
            "config/architecture/module-function-coverage-tensor.json",
            "version_line",
        )?,
    ));
    checks.push(check(
        "shipping_version",
        "releases/update-catalog.json",
        "shipping_version",
        expected,
        json_field(root, "releases/update-catalog.json", "shipping_version")?,
    ));
    add_language_pack_checks(root, expected, &mut checks)?;
    checks.extend(markdown_fact_checks(root, &development_version, codename)?);
    checks.extend(version_policy_checks(root, &development_version, codename)?);
    checks.push(check(
        "release_current_snapshot",
        "releases/index.json",
        "snapshots[status=current]",
        expected,
        current_snapshot_versions(&release_index),
    ));
    if let Some(snapshot) = current_snapshot_path(&release_index, expected) {
        let current = read_json(root, &format!("releases/{snapshot}"))?;
        checks.push(check_value(
            "current_snapshot_placeholders",
            &format!("releases/{snapshot}"),
            "summary/product_surfaces/workflow_builder/operator_sdk",
            json!(false),
            json!(contains_todo(&json!({
                "summary": current.get("summary").unwrap_or(&Value::Null),
                "product_surfaces": current.get("product_surfaces").unwrap_or(&Value::Null),
                "workflow_builder": current.get("workflow_builder").unwrap_or(&Value::Null),
                "operator_sdk": current.get("operator_sdk").unwrap_or(&Value::Null),
            }))),
        ));
    }
    let catalog = read_json(root, "releases/update-catalog.json")?;
    checks.push(check(
        "catalog_current_version",
        "releases/update-catalog.json",
        "versions[status=current]",
        expected,
        catalog_current_versions(&catalog),
    ));
    checks.push(check_value(
        "current_catalog_placeholders",
        "releases/update-catalog.json",
        "channels[status=current]/versions[status=current]",
        json!(false),
        json!(current_catalog_payloads(&catalog).iter().any(contains_todo)),
    ));
    Ok(checks)
}

fn version_policy_checks(
    root: &Path,
    development_version: &str,
    codename: &str,
) -> RunnerResult<Vec<Value>> {
    let policy = read_json(root, VERSION_POLICY_PATH)?;
    let cadence = policy.get("cadence").unwrap_or(&Value::Null);
    let active = policy.get("active_line").unwrap_or(&Value::Null);
    let minor = cadence.get("minor_range").unwrap_or(&Value::Null);
    let patch = cadence.get("patch_range").unwrap_or(&Value::Null);
    let next = policy.get("next_line").unwrap_or(&Value::Null);
    let formal_target = json_field(root, "docs/book-manifest.json", "formal_release_target")?;
    Ok(vec![
        check(
            "version_policy",
            VERSION_POLICY_PATH,
            "schema_version",
            VERSION_POLICY_SCHEMA,
            string_value(&policy, "schema_version"),
        ),
        check(
            "version_policy",
            VERSION_POLICY_PATH,
            "active_line.codename",
            codename,
            string_value(active, "codename"),
        ),
        check_value(
            "version_policy",
            VERSION_POLICY_PATH,
            "active_line.major",
            json!(2),
            active.get("major").cloned().unwrap_or(Value::Null),
        ),
        range_check("minor_range", minor, 0, 20, 21),
        range_check("patch_range", patch, 0, 9, 10),
        check(
            "version_policy",
            VERSION_POLICY_PATH,
            "active_line.first_version",
            "2.0.0",
            string_value(active, "first_version"),
        ),
        check(
            "version_policy",
            VERSION_POLICY_PATH,
            "active_line.final_version",
            "2.20.9",
            string_value(active, "final_version"),
        ),
        check_value(
            "version_policy",
            "docs/book-manifest.json",
            "current_development_version within moxi range",
            json!(true),
            json!(version_allowed(development_version, 2, 20, 9)),
        ),
        check(
            "version_policy",
            VERSION_POLICY_PATH,
            "next_line.codename",
            "daji",
            string_value(next, "codename"),
        ),
        check(
            "version_policy",
            VERSION_POLICY_PATH,
            "next_line.first_version",
            "3.0.0",
            string_value(next, "first_version"),
        ),
        check(
            "version_policy",
            VERSION_POLICY_PATH,
            "next_line.public_launch_channel",
            "reddit",
            string_value(next, "public_launch_channel"),
        ),
        check(
            "version_policy",
            "docs/book-manifest.json",
            "formal_release_target",
            "daji 3.0.0",
            formal_target,
        ),
    ])
}

fn range_check(field_name: &str, range: &Value, first: u64, last: u64, count: u64) -> Value {
    check_value(
        "version_policy",
        VERSION_POLICY_PATH,
        field_name,
        json!({ "first": first, "last": last, "count": count }),
        json!({
            "first": range.get("first").and_then(Value::as_u64),
            "last": range.get("last").and_then(Value::as_u64),
            "count": range.get("count").and_then(Value::as_u64),
        }),
    )
}

fn version_allowed(version: &str, major: u64, last_minor: u64, last_patch: u64) -> bool {
    parse_version(version).is_some_and(|(actual_major, minor, patch)| {
        actual_major == major && minor <= last_minor && patch <= last_patch
    })
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let parsed = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(parsed)
}

fn current_development_version(root: &Path) -> RunnerResult<String> {
    Ok(field(
        &read_json(root, "docs/book-manifest.json")?,
        "current_development_version",
    )
    .to_string())
}

fn add_language_pack_checks(
    root: &Path,
    expected: &str,
    checks: &mut Vec<Value>,
) -> RunnerResult<()> {
    let catalog = read_json(root, "language-packs/catalog.json")?;
    checks.push(check(
        "language_pack_catalog_shipping_version",
        "language-packs/catalog.json",
        "shipping_version",
        expected,
        string_value(&catalog, "shipping_version"),
    ));
    for entry in array(&catalog, "packs") {
        let pack_path = format!("language-packs/{}", field(entry, "path"));
        let pack = read_json(root, &pack_path)?;
        checks.push(check(
            "language_pack_version",
            &pack_path,
            "version",
            expected,
            string_value(&pack, "version"),
        ));
        checks.push(check(
            "language_pack_target_app_version",
            &pack_path,
            "targetAppVersion",
            expected,
            string_value(&pack, "targetAppVersion"),
        ));
    }
    Ok(())
}

fn markdown_fact_checks(root: &Path, expected: &str, codename: &str) -> RunnerResult<Vec<Value>> {
    let minor = version_minor_line(expected);
    let display = version_display(codename, expected);
    let display_minor = version_display(codename, &minor);
    let facts = [
        (
            "docs/version-line.md",
            "current development point",
            format!("current development point: `{display}`"),
        ),
        (
            "docs/version-line.md",
            "current documentation target",
            format!("current documentation target: `{display_minor}` line"),
        ),
        (
            "docs/current-line.md",
            "current development point",
            format!("current development point in this line is `{display}`"),
        ),
        (
            "docs/installer-remote-control.md",
            "runtime control line",
            format!("`{display_minor}` line"),
        ),
    ];
    facts
        .into_iter()
        .map(|(file, field_name, expected_text)| {
            let actual = if read_text(root, file)?.contains(&expected_text) {
                Value::String(expected_text.clone())
            } else {
                Value::Null
            };
            Ok(check_value(
                "text_includes",
                file,
                field_name,
                Value::String(expected_text),
                actual,
            ))
        })
        .collect()
}

fn run_self_test() -> RunnerResult<u8> {
    let checks = self_test_markdown_checks("2.0.0", DEFAULT_CODENAME);
    let failed = checks
        .iter()
        .filter(|check| check.get("actual") != check.get("expected"))
        .count();
    if failed != checks.len() {
        eprintln!("Version line audit self-test failed: stale Markdown facts were not rejected");
        return Ok(1);
    }
    println!("version line audit self-test passed");
    Ok(0)
}

fn self_test_markdown_checks(expected: &str, codename: &str) -> Vec<Value> {
    let minor = version_minor_line(expected);
    let display = version_display(codename, expected);
    let display_minor = version_display(codename, &minor);
    [
        ("docs/version-line.md", "current development point", format!("current development point: `{display}`"), "current development point: `moxi 1.15.0`\ncurrent documentation target: `moxi 1.15.x` pre-`2.x` line"),
        ("docs/version-line.md", "current documentation target", format!("current documentation target: `{display_minor}` line"), "current development point: `moxi 1.15.0`\ncurrent documentation target: `moxi 1.15.x` pre-`2.x` line"),
        ("docs/current-line.md", "current development point", format!("current development point in this line is `{display}`"), "The current development point in this line is `moxi 1.15.0`."),
        ("docs/installer-remote-control.md", "runtime control line", format!("`{display_minor}` line"), "remote runtime control surface in the `moxi 1.15.x` preparation line."),
    ]
    .into_iter()
    .map(|(file, field_name, expected_text, text)| {
        let actual = if text.contains(&expected_text) { Value::String(expected_text.clone()) } else { Value::Null };
        check_value("text_includes", file, field_name, Value::String(expected_text), actual)
    })
    .collect()
}

fn check(kind: &str, file: &str, field_name: &str, expected: &str, actual: Value) -> Value {
    check_value(
        kind,
        file,
        field_name,
        Value::String(expected.to_string()),
        actual,
    )
}

fn check_value(kind: &str, file: &str, field_name: &str, expected: Value, actual: Value) -> Value {
    let ok = actual == expected;
    json!({ "kind": kind, "file": file, "field": field_name, "expected": expected, "actual": actual, "ok": ok })
}

fn read_json(root: &Path, relative: &str) -> RunnerResult<Value> {
    serde_json::from_str(&read_text(root, relative)?)
        .map_err(|error| format!("{relative}: invalid json: {error}"))
}

fn read_text(root: &Path, relative: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

fn json_field(root: &Path, relative: &str, key: &str) -> RunnerResult<Value> {
    Ok(read_json(root, relative)?
        .get(key)
        .cloned()
        .unwrap_or(Value::Null))
}

fn cargo_version(root: &Path, relative: &str) -> RunnerResult<Value> {
    for line in read_text(root, relative)?.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("version") {
            if let Some(version) = value.split('"').nth(1) {
                return Ok(Value::String(version.to_string()));
            }
        }
    }
    Ok(Value::Null)
}

fn mix_version(root: &Path, relative: &str) -> RunnerResult<Value> {
    for line in read_text(root, relative)?.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version:") {
            if let Some(version) = last_quoted_value(trimmed) {
                return Ok(Value::String(version.to_string()));
            }
        }
    }
    Ok(Value::Null)
}

fn last_quoted_value(text: &str) -> Option<&str> {
    let parts = text.split('"').collect::<Vec<_>>();
    (parts.len() >= 3).then(|| parts[parts.len() - 2])
}

fn stable_channel_version(channels: &Value) -> Value {
    let default = field(channels, "default_channel");
    let id = if default.is_empty() {
        "stable"
    } else {
        default
    };
    array(channels, "channels")
        .into_iter()
        .find(|channel| field(channel, "id") == id)
        .and_then(|channel| channel.get("version").cloned())
        .unwrap_or(Value::Null)
}

fn required_version_rule(contract: &Value) -> Value {
    array(contract, "visible_rules")
        .into_iter()
        .find(|rule| {
            matches!(
                field(rule, "label"),
                "required development version"
                    | "required packaged version"
                    | "required shipping version"
            )
        })
        .and_then(|rule| rule.get("value").cloned())
        .unwrap_or(Value::Null)
}

fn current_snapshot_versions(release_index: &Value) -> Value {
    let current = array(release_index, "snapshots")
        .into_iter()
        .filter(|snapshot| field(snapshot, "status") == "current")
        .map(|snapshot| field(snapshot, "version").to_string())
        .collect::<Vec<_>>();
    Value::String(if current.len() == 1 {
        current[0].clone()
    } else {
        current.join(",")
    })
}

fn current_snapshot_path(release_index: &Value, expected: &str) -> Option<String> {
    array(release_index, "snapshots")
        .into_iter()
        .find(|snapshot| {
            field(snapshot, "status") == "current" && field(snapshot, "version") == expected
        })
        .map(|snapshot| field(snapshot, "snapshot_path").to_string())
        .filter(|value| !value.is_empty())
}

fn catalog_current_versions(catalog: &Value) -> Value {
    let current = array(catalog, "versions")
        .into_iter()
        .filter(|version| field(version, "status") == "current")
        .map(|version| field(version, "version").to_string())
        .collect::<Vec<_>>();
    Value::String(if current.len() == 1 {
        current[0].clone()
    } else {
        current.join(",")
    })
}

fn current_catalog_payloads(catalog: &Value) -> Vec<Value> {
    array(catalog, "channels")
        .into_iter()
        .chain(array(catalog, "versions"))
        .filter(|item| field(item, "status") == "current")
        .cloned()
        .collect()
}

fn contains_todo(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains("TODO:"),
        Value::Array(items) => items.iter().any(contains_todo),
        Value::Object(object) => object.values().any(contains_todo),
        _ => false,
    }
}

fn string_value(value: &Value, key: &str) -> Value {
    value.get(key).cloned().unwrap_or(Value::Null)
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::{contains_todo, last_quoted_value, version_allowed, version_minor_line};
    use serde_json::json;

    #[test]
    fn minor_line_keeps_major_minor() {
        assert_eq!(version_minor_line("1.20.0"), "1.20.x");
        assert_eq!(version_minor_line("2"), "2.x");
    }

    #[test]
    fn todo_scan_recurses() {
        assert!(contains_todo(&json!({"nested": ["TODO: fill"]})));
        assert!(!contains_todo(&json!({"nested": ["done"]})));
    }

    #[test]
    fn mix_version_uses_the_environment_default() {
        assert_eq!(
            last_quoted_value(r#"version: System.get_env("KYUUBIKI_RELEASE_VERSION", "2.17.0"),"#),
            Some("2.17.0")
        );
    }

    #[test]
    fn moxi_version_space_ends_at_2_20_9() {
        assert!(version_allowed("2.0.0", 2, 20, 9));
        assert!(version_allowed("2.11.7", 2, 20, 9));
        assert!(version_allowed("2.20.9", 2, 20, 9));
        assert!(!version_allowed("2.20.10", 2, 20, 9));
        assert!(!version_allowed("2.21.0", 2, 20, 9));
        assert!(!version_allowed("3.0.0", 2, 20, 9));
    }
}
