use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

const CONTRACT_PATH: &str = "config/dependency-audit-lockfiles.json";
const SCHEMA: &str = "kyuubiki.dependency-audit-lockfiles/v1";
const NPM_ARGS: &[&str] = &["audit", "--omit=dev", "--package-lock-only", "--json"];
const CARGO_ARGS: &[&str] = &["audit"];
const HEX_ARGS: &[&str] = &["hex.audit"];

type RunnerResult<T> = Result<T, String>;

#[derive(Clone)]
struct AuditContract {
    npm: Vec<String>,
    cargo: Vec<String>,
    hex: Vec<String>,
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
    results.extend(contract.cargo.iter().map(|cwd| {
        let result = run(root, "cargo", CARGO_ARGS, cwd);
        let summary = result
            .stdout
            .lines()
            .find(|line| line.contains("allowed warnings found"))
            .map(str::trim)
            .unwrap_or("0 vulnerability(s)")
            .to_string();
        AuditResult { summary, ..result }
    }));
    results.extend(contract.hex.iter().map(|cwd| {
        let result = run(root, "mix", HEX_ARGS, cwd);
        let summary = if result.status == 0 {
            "0 advisory/retired package(s)"
        } else {
            "Hex advisories found"
        }
        .to_string();
        AuditResult { summary, ..result }
    }));
    results
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
    Ok(AuditContract {
        npm: string_array(&value, "npm")?,
        cargo: string_array(&value, "cargo")?,
        hex: string_array(&value, "hex")?,
    })
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
    expect_eq(&HEX_ARGS.to_vec(), &["hex.audit"], "hex audit args")?;
    if summarize_npm_audit(r#"{"metadata":{"vulnerabilities":{"total":0}}}"#)
        != "0 vulnerability(s)"
    {
        return Err("self-test expected npm summary total".to_string());
    }
    if summarize_npm_audit("not json") != "unable to parse npm audit JSON" {
        return Err("self-test expected bad npm JSON summary".to_string());
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

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{format_npm_audit_failure, summarize_npm_audit};

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
}
