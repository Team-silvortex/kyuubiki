use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

const CONTRACT_PATH: &str = "config/dependency-audit-lockfiles.json";
const SCHEMA: &str = "kyuubiki.dependency-audit-lockfiles/v1";
const NPM_ARGS: &[&str] = &["audit", "--omit=dev", "--package-lock-only", "--json"];
const CARGO_ARGS: &[&str] = &["audit"];
const CARGO_NO_FETCH_ARGS: &[&str] = &["audit", "--no-fetch"];
const HEX_ARGS: &[&str] = &["hex.audit"];
const CARGO_ADVISORY_CACHE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

type RunnerResult<T> = Result<T, String>;

#[derive(Clone)]
struct AuditContract {
    npm: Vec<String>,
    cargo: Vec<String>,
    hex: Vec<String>,
    hex_advisory_mitigations: Vec<HexAdvisoryMitigation>,
}

#[derive(Clone)]
struct HexAdvisoryMitigation {
    id: String,
    package: String,
    locked_version: String,
    status: String,
    evidence: Vec<String>,
}

#[derive(Clone, Debug)]
struct AuditResult {
    command: String,
    cwd: String,
    status: i32,
    stdout: String,
    stderr: String,
    summary: String,
}

pub(crate) fn run_audit_dependencies(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let contract = load_contract(root)?;
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(&contract)?;
        println!("dependency audit self-test passed");
        return Ok(0);
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner audit-dependencies [--self-test]");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("audit-dependencies only accepts --self-test".to_string());
    }
    let results = audit_all(root, &contract);
    for result in &results {
        print_result(result);
    }
    let failures = results.iter().filter(|result| result.status != 0).count();
    if failures > 0 {
        eprintln!("dependency audit failed: {failures} lane(s) failed");
        return Ok(1);
    }
    println!("dependency audit passed");
    Ok(0)
}

fn audit_all(root: &Path, contract: &AuditContract) -> Vec<AuditResult> {
    let mut results = Vec::new();
    results.extend(contract.npm.iter().map(|cwd| {
        let result = run(root, "npm", NPM_ARGS, cwd);
        AuditResult {
            summary: summarize_npm_audit(&result.stdout),
            ..result
        }
    }));
    results.extend(audit_cargo_lanes(root, &contract.cargo));
    results.extend(contract.hex.iter().map(|cwd| {
        let mut result = run(root, "mix", HEX_ARGS, cwd);
        let output = format!("{}\n{}", result.stdout, result.stderr);
        let (status, summary) =
            classify_hex_audit(result.status, &output, &contract.hex_advisory_mitigations);
        result.status = status;
        AuditResult { summary, ..result }
    }));
    results
}

fn audit_cargo_lanes(root: &Path, dirs: &[String]) -> Vec<AuditResult> {
    let Some((first, rest)) = dirs.split_first() else {
        return Vec::new();
    };
    let refreshed = run(root, "cargo", CARGO_ARGS, first);
    let first_result = if is_cargo_fetch_failure(&refreshed) && cargo_cache_is_fresh(root) {
        let mut cached = run(root, "cargo", CARGO_NO_FETCH_ARGS, first);
        cached.summary = cargo_audit_summary(&cached, true);
        cached
    } else {
        AuditResult {
            summary: cargo_audit_summary(&refreshed, false),
            ..refreshed
        }
    };
    let mut results = vec![first_result];
    results.extend(rest.iter().map(|cwd| {
        let result = run(root, "cargo", CARGO_NO_FETCH_ARGS, cwd);
        AuditResult {
            summary: cargo_audit_summary(&result, true),
            ..result
        }
    }));
    results
}

fn cargo_audit_summary(result: &AuditResult, cached: bool) -> String {
    let allowed = result
        .stdout
        .lines()
        .find(|line| line.contains("allowed warnings found"))
        .map(str::trim);
    let summary = allowed.unwrap_or(if result.status == 0 {
        "0 vulnerability(s)"
    } else {
        "audit incomplete or vulnerabilities reported"
    });
    if cached {
        format!("{summary}; advisory DB cache <= 24h")
    } else {
        summary.to_string()
    }
}

fn is_cargo_fetch_failure(result: &AuditResult) -> bool {
    let output = format!("{}\n{}", result.stdout, result.stderr);
    output.contains("couldn't fetch advisory database")
        || output.contains("failed to prepare fetch")
}

fn cargo_cache_is_fresh(root: &Path) -> bool {
    cargo_advisory_db(root)
        .map(|path| path.join(".git/FETCH_HEAD"))
        .and_then(|path| fs::metadata(path).ok())
        .and_then(|metadata| metadata.modified().ok())
        .is_some_and(|modified| cache_timestamp_is_fresh(modified, SystemTime::now()))
}

fn cargo_advisory_db(root: &Path) -> Option<PathBuf> {
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))?;
    let cargo_home = if cargo_home.is_absolute() {
        cargo_home
    } else {
        root.join(cargo_home)
    };
    Some(cargo_home.join("advisory-db"))
}

fn cache_timestamp_is_fresh(modified: SystemTime, now: SystemTime) -> bool {
    now.duration_since(modified)
        .is_ok_and(|age| age <= CARGO_ADVISORY_CACHE_MAX_AGE)
}

fn run(root: &Path, command: &str, args: &[&str], cwd: &str) -> AuditResult {
    let output = Command::new(command)
        .args(args)
        .current_dir(root.join(cwd))
        .output();
    match output {
        Ok(output) => AuditResult {
            command: std::iter::once(command)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .join(" "),
            cwd: cwd.to_string(),
            status: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            summary: String::new(),
        },
        Err(error) => AuditResult {
            command: std::iter::once(command)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .join(" "),
            cwd: cwd.to_string(),
            status: 1,
            stdout: String::new(),
            stderr: error.to_string(),
            summary: String::new(),
        },
    }
}

fn print_result(result: &AuditResult) {
    let marker = if result.status == 0 { "ok" } else { "failed" };
    println!("[{marker}] {}: {}", result.cwd, result.command);
    println!("      {}", result.summary);
    if result.status != 0 {
        let stdout = if result.command.starts_with("npm audit") {
            format_npm_audit_failure(&result.stdout)
        } else {
            result.stdout.clone()
        };
        let detail = [result.stderr.as_str(), stdout.as_str()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!("{detail}");
    }
}

fn load_contract(root: &Path) -> RunnerResult<AuditContract> {
    let text = fs::read_to_string(root.join(CONTRACT_PATH))
        .map_err(|error| format!("failed to read {CONTRACT_PATH}: {error}"))?;
    let value = serde_json::from_str::<Value>(&text)
        .map_err(|error| format!("{CONTRACT_PATH}: invalid json: {error}"))?;
    if field(&value, "schema") != SCHEMA {
        return Err(format!("{CONTRACT_PATH}: unexpected schema"));
    }
    let contract = AuditContract {
        npm: string_array(&value, "npm")?,
        cargo: string_array(&value, "cargo")?,
        hex: string_array(&value, "hex")?,
        hex_advisory_mitigations: parse_hex_mitigations(&value)?,
    };
    validate_hex_mitigations(root, &contract.hex_advisory_mitigations)?;
    Ok(contract)
}

fn parse_hex_mitigations(value: &Value) -> RunnerResult<Vec<HexAdvisoryMitigation>> {
    value
        .get("hex_advisory_mitigations")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{CONTRACT_PATH}: hex_advisory_mitigations must be an array"))?
        .iter()
        .map(|entry| {
            Ok(HexAdvisoryMitigation {
                id: required_field(entry, "id")?,
                package: required_field(entry, "package")?,
                locked_version: required_field(entry, "locked_version")?,
                status: required_field(entry, "status")?,
                evidence: string_array(entry, "evidence")?,
            })
        })
        .collect()
}

fn validate_hex_mitigations(
    root: &Path,
    mitigations: &[HexAdvisoryMitigation],
) -> RunnerResult<()> {
    let mix_lock = fs::read_to_string(root.join("apps/web/mix.lock"))
        .map_err(|error| format!("failed to read apps/web/mix.lock: {error}"))?;
    let mut ids = BTreeSet::new();
    for mitigation in mitigations {
        if !ids.insert(mitigation.id.as_str()) {
            return Err(format!(
                "{CONTRACT_PATH}: duplicate advisory {}",
                mitigation.id
            ));
        }
        if !matches!(mitigation.status.as_str(), "mitigated" | "not_reachable") {
            return Err(format!(
                "{CONTRACT_PATH}: invalid mitigation status for {}",
                mitigation.id
            ));
        }
        let lock_prefix = format!(
            "\"{}\": {{:hex, :{}, \"{}\"",
            mitigation.package, mitigation.package, mitigation.locked_version
        );
        if !mix_lock.contains(&lock_prefix) {
            return Err(format!(
                "{CONTRACT_PATH}: {} does not match apps/web/mix.lock",
                mitigation.id
            ));
        }
        if mitigation.evidence.is_empty() {
            return Err(format!(
                "{CONTRACT_PATH}: {} has no evidence",
                mitigation.id
            ));
        }
        for evidence in &mitigation.evidence {
            if Path::new(evidence).is_absolute() || evidence.contains("..") {
                return Err(format!("{CONTRACT_PATH}: unsafe evidence path {evidence}"));
            }
            if !root.join(evidence).is_file() {
                return Err(format!("{CONTRACT_PATH}: missing evidence {evidence}"));
            }
        }
    }
    Ok(())
}

fn parse_hex_advisory_ids(output: &str) -> BTreeSet<String> {
    let mut advisories = BTreeSet::new();
    for (offset, _) in output.match_indices("CVE-") {
        let id = output[offset..]
            .chars()
            .take_while(|character| {
                character.is_ascii_digit()
                    || *character == '-'
                    || *character == 'C'
                    || *character == 'V'
                    || *character == 'E'
            })
            .collect::<String>();
        if id.matches('-').count() == 2 {
            advisories.insert(id);
        }
    }
    advisories
}

fn classify_hex_audit(
    command_status: i32,
    output: &str,
    mitigations: &[HexAdvisoryMitigation],
) -> (i32, String) {
    let advisories = parse_hex_advisory_ids(output);
    let mitigated = mitigations
        .iter()
        .map(|mitigation| mitigation.id.as_str())
        .collect::<BTreeSet<_>>();
    let unknown = advisories
        .iter()
        .filter(|advisory| !mitigated.contains(advisory.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !unknown.is_empty() {
        return (
            1,
            format!("unmitigated Hex advisories: {}", unknown.join(", ")),
        );
    }
    if !advisories.is_empty() {
        return (
            0,
            format!(
                "{} explicitly mitigated Hex advisory/advisories: {}",
                advisories.len(),
                advisories.into_iter().collect::<Vec<_>>().join(", ")
            ),
        );
    }
    if command_status == 0 {
        if mitigations.is_empty() {
            (
                0,
                "0 retired package(s); 0 tracked advisory mitigation(s)".to_string(),
            )
        } else {
            let tracked = mitigations
                .iter()
                .map(|mitigation| mitigation.id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            (
                0,
                format!(
                    "0 retired package(s); {} tracked lock-bound advisory mitigation(s): {tracked}",
                    mitigations.len()
                ),
            )
        }
    } else {
        (command_status, "Hex audit command failed".to_string())
    }
}

fn summarize_npm_audit(output: &str) -> String {
    serde_json::from_str::<Value>(output)
        .ok()
        .and_then(|value| value.pointer("/metadata/vulnerabilities/total").cloned())
        .map(|value| format!("{value} vulnerability(s)"))
        .unwrap_or_else(|| "unable to parse npm audit JSON".to_string())
}

fn format_npm_audit_failure(output: &str) -> String {
    let Ok(parsed) = serde_json::from_str::<Value>(output) else {
        return output.to_string();
    };
    let Some(vulnerabilities) = parsed.get("vulnerabilities").and_then(Value::as_object) else {
        return output.to_string();
    };
    if vulnerabilities.is_empty() {
        return output.to_string();
    }
    vulnerabilities
        .values()
        .map(|vulnerability| {
            let name = field(vulnerability, "name");
            let severity = field(vulnerability, "severity");
            let direct = if vulnerability
                .get("isDirect")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "direct"
            } else {
                "transitive"
            };
            let via = format_via(vulnerability.get("via"));
            format!("- {name} ({severity}, {direct}): {via}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_via(value: Option<&Value>) -> String {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(|entry| match entry {
                Value::String(text) => text.clone(),
                Value::Object(_) => [field(entry, "title"), field(entry, "url")]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join(" "),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join("; "),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => "unknown".to_string(),
    }
}

fn run_self_test(contract: &AuditContract) -> RunnerResult<()> {
    expect_eq(
        &contract.npm,
        &[
            "apps/frontend",
            "apps/hub-gui",
            "apps/installer-gui",
            "apps/workbench-gui",
        ],
        "npm audit dirs",
    )?;
    expect_eq(
        &contract.cargo,
        &[
            "workers/rust",
            "sdks/rust",
            "apps/hub-gui/src-tauri",
            "apps/installer-gui/src-tauri",
            "apps/workbench-gui/src-tauri",
        ],
        "cargo audit dirs",
    )?;
    expect_eq(&contract.hex, &["apps/web"], "hex audit dirs")?;
    expect_eq(
        &contract
            .hex_advisory_mitigations
            .iter()
            .map(|mitigation| mitigation.id.as_str())
            .collect::<Vec<_>>(),
        &["CVE-2026-43966", "CVE-2026-43971"],
        "Hex mitigated advisories",
    )?;
    expect_eq(
        &contract
            .hex_advisory_mitigations
            .iter()
            .map(|mitigation| mitigation.locked_version.as_str())
            .collect::<Vec<_>>(),
        &["2.19.0", "2.19.0"],
        "Hex mitigation lock versions",
    )?;
    expect_eq(
        &lockfiles(&contract.npm, "package-lock.json"),
        &[
            "apps/frontend/package-lock.json",
            "apps/hub-gui/package-lock.json",
            "apps/installer-gui/package-lock.json",
            "apps/workbench-gui/package-lock.json",
        ],
        "npm lockfiles",
    )?;
    expect_eq(
        &lockfiles(&contract.cargo, "Cargo.lock"),
        &[
            "workers/rust/Cargo.lock",
            "sdks/rust/Cargo.lock",
            "apps/hub-gui/src-tauri/Cargo.lock",
            "apps/installer-gui/src-tauri/Cargo.lock",
            "apps/workbench-gui/src-tauri/Cargo.lock",
        ],
        "cargo lockfiles",
    )?;
    expect_eq(
        &lockfiles(&contract.hex, "mix.lock"),
        &["apps/web/mix.lock"],
        "hex lockfiles",
    )?;
    expect_eq(
        &NPM_ARGS.to_vec(),
        &["audit", "--omit=dev", "--package-lock-only", "--json"],
        "npm audit args",
    )?;
    expect_eq(&CARGO_ARGS.to_vec(), &["audit"], "cargo audit args")?;
    expect_eq(
        &CARGO_NO_FETCH_ARGS.to_vec(),
        &["audit", "--no-fetch"],
        "cached cargo audit args",
    )?;
    expect_eq(&HEX_ARGS.to_vec(), &["hex.audit"], "hex audit args")?;
    if summarize_npm_audit(r#"{"metadata":{"vulnerabilities":{"total":0}}}"#)
        != "0 vulnerability(s)"
    {
        return Err("self-test expected npm summary total".to_string());
    }
    if summarize_npm_audit("not json") != "unable to parse npm audit JSON" {
        return Err("self-test expected bad npm JSON summary".to_string());
    }
    let parsed = parse_hex_advisory_ids(
        "cowlib 2.19.0 - EEF-CVE-2026-43971\naka: CVE-2026-43971\ncowlib 2.19.0 - EEF-CVE-2026-43966",
    );
    if parsed.into_iter().collect::<Vec<_>>()
        != ["CVE-2026-43966".to_string(), "CVE-2026-43971".to_string()]
    {
        return Err("self-test expected Hex advisory identifiers".to_string());
    }
    let formatted = format_npm_audit_failure(
        r#"{"vulnerabilities":{"next":{"name":"next","severity":"critical","isDirect":true,"via":[{"title":"Middleware bypass","url":"https://example.test/advisory"},"postcss"]}}}"#,
    );
    if formatted
        != "- next (critical, direct): Middleware bypass https://example.test/advisory; postcss"
    {
        return Err("self-test expected formatted npm advisory".to_string());
    }
    Ok(())
}

fn lockfiles(dirs: &[String], suffix: &str) -> Vec<String> {
    dirs.iter().map(|dir| format!("{dir}/{suffix}")).collect()
}

fn expect_eq<T>(actual: &[T], expected: &[&str], label: &str) -> RunnerResult<()>
where
    T: AsRef<str>,
{
    let actual = actual.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("self-test {label} drifted"))
    }
}

fn string_array(value: &Value, key: &str) -> RunnerResult<Vec<String>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{CONTRACT_PATH}: {key} must be an array"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{CONTRACT_PATH}: {key} entries must be strings"))
        })
        .collect()
}

fn required_field(value: &Value, key: &str) -> RunnerResult<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|field| !field.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{CONTRACT_PATH}: missing {key}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        AuditResult, HexAdvisoryMitigation, cache_timestamp_is_fresh, classify_hex_audit,
        format_npm_audit_failure, is_cargo_fetch_failure, parse_hex_advisory_ids,
        summarize_npm_audit,
    };
    use std::time::{Duration, SystemTime};

    fn mitigation(id: &str) -> HexAdvisoryMitigation {
        HexAdvisoryMitigation {
            id: id.to_string(),
            package: "cowlib".to_string(),
            locked_version: "2.19.0".to_string(),
            status: "mitigated".to_string(),
            evidence: vec!["evidence".to_string()],
        }
    }

    #[test]
    fn npm_summary_handles_json_and_bad_json() {
        assert_eq!(
            summarize_npm_audit(r#"{"metadata":{"vulnerabilities":{"total":0}}}"#),
            "0 vulnerability(s)"
        );
        assert_eq!(
            summarize_npm_audit("oops"),
            "unable to parse npm audit JSON"
        );
    }

    #[test]
    fn npm_failure_formats_vulnerabilities() {
        let formatted = format_npm_audit_failure(
            r#"{"vulnerabilities":{"next":{"name":"next","severity":"critical","isDirect":true,"via":[{"title":"Middleware bypass","url":"https://example.test/advisory"},"postcss"]}}}"#,
        );
        assert_eq!(
            formatted,
            "- next (critical, direct): Middleware bypass https://example.test/advisory; postcss"
        );
    }

    #[test]
    fn hex_advisory_parser_deduplicates_alias_lines() {
        let parsed = parse_hex_advisory_ids(
            "cowlib - EEF-CVE-2026-43971\naka: CVE-2026-43971, GHSA-test\ncowlib - EEF-CVE-2026-43966",
        );
        assert_eq!(
            parsed.into_iter().collect::<Vec<_>>(),
            ["CVE-2026-43966", "CVE-2026-43971"]
        );
    }

    #[test]
    fn hex_audit_only_accepts_explicit_mitigations() {
        let mitigations = [mitigation("CVE-2026-43966")];
        let (status, summary) = classify_hex_audit(
            0,
            "cowlib - EEF-CVE-2026-43966\naka: CVE-2026-43966",
            &mitigations,
        );
        assert_eq!(status, 0);
        assert!(summary.contains("explicitly mitigated"));

        let (status, summary) = classify_hex_audit(
            0,
            "cowlib - EEF-CVE-2026-43966\nother - CVE-2099-00001",
            &mitigations,
        );
        assert_eq!(status, 1);
        assert_eq!(summary, "unmitigated Hex advisories: CVE-2099-00001");
    }

    #[test]
    fn hex_audit_preserves_command_failure_without_advisories() {
        let (status, summary) = classify_hex_audit(7, "network unavailable", &[]);
        assert_eq!(status, 7);
        assert_eq!(summary, "Hex audit command failed");
    }

    #[test]
    fn hex_audit_surfaces_lock_bound_mitigations_without_retirements() {
        let mitigations = [mitigation("CVE-2026-43966"), mitigation("CVE-2026-43971")];
        let (status, summary) = classify_hex_audit(0, "No retired packages found", &mitigations);

        assert_eq!(status, 0);
        assert_eq!(
            summary,
            "0 retired package(s); 2 tracked lock-bound advisory mitigation(s): CVE-2026-43966, CVE-2026-43971"
        );
    }

    #[test]
    fn cargo_fetch_failures_are_distinct_from_vulnerability_results() {
        let fetch_failure = AuditResult {
            command: "cargo audit".to_string(),
            cwd: "workers/rust".to_string(),
            status: 1,
            stdout: String::new(),
            stderr: "error: couldn't fetch advisory database: failed to prepare fetch".to_string(),
            summary: String::new(),
        };
        assert!(is_cargo_fetch_failure(&fetch_failure));
        let vulnerability = AuditResult {
            stderr: "1 vulnerability found".to_string(),
            ..fetch_failure
        };
        assert!(!is_cargo_fetch_failure(&vulnerability));
    }

    #[test]
    fn cargo_advisory_cache_must_be_recent_and_not_future_dated() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100_000);
        assert!(cache_timestamp_is_fresh(
            now - Duration::from_secs(23 * 60 * 60),
            now
        ));
        assert!(!cache_timestamp_is_fresh(
            now - Duration::from_secs(25 * 60 * 60),
            now
        ));
        assert!(!cache_timestamp_is_fresh(now + Duration::from_secs(1), now));
    }
}
