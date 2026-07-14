use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

const TOOLCHAINS_PATH: &str = "config/toolchains.json";
const MIX_PATH: &str = "apps/web/mix.exs";
const CONFIG_PATH: &str = "apps/web/config/config.exs";
const SCHEMA_VERSION: &str = "kyuubiki.elixir-self-host-preflight/v1";

type RunnerResult<T> = Result<T, String>;

struct Options {
    json: bool,
    static_only: bool,
}

pub(crate) fn run_check_elixir_self_host(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner check-elixir-self-host [--static-only] [--json]");
        return Ok(0);
    }
    let options = parse_args(args)?;
    let report = build_report(root, &options)?;
    let issues = string_array(&report, "issues");
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to encode report: {error}"))?
        );
    } else if issues.is_empty() {
        println!("elixir self-host preflight ok");
        if !options.static_only {
            println!(
                "Elixir {}, Mix {}, OTP {}",
                report
                    .pointer("/detected/elixir")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                report
                    .pointer("/detected/mix")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                report
                    .pointer("/detected/otp")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            );
        }
    } else {
        eprintln!("elixir self-host preflight failed:");
        for issue in &issues {
            eprintln!("- {issue}");
        }
    }
    Ok(if issues.is_empty() { 0 } else { 1 })
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        json: false,
        static_only: false,
    };
    for arg in args {
        match arg.to_string_lossy().as_ref() {
            "--json" => options.json = true,
            "--static-only" => options.static_only = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

fn build_report(root: &Path, options: &Options) -> RunnerResult<Value> {
    let contract = read_json(root, TOOLCHAINS_PATH)?;
    let elixir = contract
        .get("elixir")
        .ok_or_else(|| format!("{TOOLCHAINS_PATH}: missing elixir contract"))?;
    let mut issues = Vec::<String>::new();
    let mut report = json!({
        "schema_version": SCHEMA_VERSION,
        "mode": if options.static_only { "static" } else { "runtime" },
        "contract": {
            "elixir_constraint": field(elixir, "constraint"),
            "elixir_minimum": field(elixir, "minimum"),
            "otp_minimum": field(elixir, "otp_minimum"),
            "container_base": field(elixir, "container_base"),
            "required_env": string_array(elixir, "self_host_required_env"),
            "optional_env": string_array(elixir, "self_host_optional_env"),
        },
        "detected": {},
        "env": {},
        "checks": [],
    });

    let mix = read_text(root, MIX_PATH)?;
    let constraint = field(elixir, "constraint");
    if !mix.contains(&format!("elixir: \"{constraint}\"")) {
        issues.push(format!("{MIX_PATH} does not declare {constraint}"));
    }
    let config = read_text(root, CONFIG_PATH)?;
    fill_env_report(
        &mut report,
        &config,
        &string_array(elixir, "self_host_required_env"),
        true,
        &mut issues,
    );
    fill_env_report(
        &mut report,
        &config,
        &string_array(elixir, "self_host_optional_env"),
        false,
        &mut issues,
    );
    if !options.static_only {
        let elixir_version = run_version(root, "elixir", &["--version"]);
        let mix_version = run_version(root, "mix", &["--version"]);
        if !elixir_version.ok {
            issues.push(format!(
                "elixir --version failed: {}",
                elixir_version.failure_detail()
            ));
        }
        if !mix_version.ok {
            issues.push(format!(
                "mix --version failed: {}",
                mix_version.failure_detail()
            ));
        }
        let combined = format!("{}\n{}", elixir_version.stdout, mix_version.stdout);
        let detected_elixir = parse_labeled_version(&combined, "Elixir");
        let detected_mix = parse_labeled_version(&combined, "Mix");
        let detected_otp = parse_otp_version(&combined);
        set_detected(&mut report, "elixir", &detected_elixir);
        set_detected(&mut report, "mix", &detected_mix);
        set_detected(&mut report, "otp", &detected_otp);
        push_version_issue(
            &mut issues,
            "Elixir",
            detected_elixir.as_deref(),
            field(elixir, "minimum"),
        );
        push_version_issue(
            &mut issues,
            "Mix",
            detected_mix.as_deref(),
            field(elixir, "minimum"),
        );
        push_version_issue(
            &mut issues,
            "OTP",
            detected_otp.as_deref(),
            field(elixir, "otp_minimum"),
        );
    }
    report["status"] = Value::from(if issues.is_empty() { "ok" } else { "fail" });
    report["issues"] = Value::Array(issues.into_iter().map(Value::from).collect());
    Ok(report)
}

fn fill_env_report(
    report: &mut Value,
    config: &str,
    keys: &[String],
    required: bool,
    issues: &mut Vec<String>,
) {
    let env = report
        .get_mut("env")
        .and_then(Value::as_object_mut)
        .expect("env report is an object");
    for key in keys {
        let referenced = config.contains(key);
        env.insert(
            key.clone(),
            json!({
                "referenced_by_config": referenced,
                "value_present": std::env::var_os(key).is_some(),
            }),
        );
        if required && !referenced {
            issues.push(format!("{CONFIG_PATH} does not reference {key}"));
        }
    }
}

struct VersionOutput {
    ok: bool,
    status: Option<i32>,
    stdout: String,
    stderr: String,
    error: Option<String>,
}

impl VersionOutput {
    fn failure_detail(&self) -> String {
        self.error
            .clone()
            .or_else(|| {
                let stderr = self.stderr.trim();
                if stderr.is_empty() {
                    None
                } else {
                    Some(stderr.to_string())
                }
            })
            .or_else(|| self.status.map(|status| status.to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }
}

fn run_version(root: &Path, command: &str, args: &[&str]) -> VersionOutput {
    match Command::new(command).args(args).current_dir(root).output() {
        Ok(output) => VersionOutput {
            ok: output.status.success(),
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            error: None,
        },
        Err(error) => VersionOutput {
            ok: false,
            status: None,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(error.to_string()),
        },
    }
}

fn parse_labeled_version(text: &str, label: &str) -> Option<String> {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|pair| {
            if pair[0] == label && looks_like_version(pair[1]) {
                Some(pair[1].to_string())
            } else {
                None
            }
        })
}

fn parse_otp_version(text: &str) -> Option<String> {
    text.split_whitespace()
        .find_map(|token| {
            token
                .strip_prefix("Erlang/OTP")
                .filter(|version| !version.is_empty())
                .map(str::to_string)
                .or_else(|| if token == "Erlang/OTP" { None } else { None })
        })
        .or_else(|| {
            let marker = "Erlang/OTP ";
            text.find(marker).and_then(|index| {
                text[index + marker.len()..]
                    .split_whitespace()
                    .next()
                    .filter(|value| looks_like_version(value))
                    .map(str::to_string)
            })
        })
}

fn looks_like_version(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
}

fn push_version_issue(issues: &mut Vec<String>, label: &str, actual: Option<&str>, expected: &str) {
    match actual {
        None => issues.push(format!("{label}: unable to detect version")),
        Some(actual) if compare_versions(actual, expected).is_lt() => {
            issues.push(format!("{label}: {actual} is below required {expected}"));
        }
        _ => {}
    }
}

fn compare_versions(actual: &str, expected: &str) -> std::cmp::Ordering {
    let left = version_parts(actual);
    let right = version_parts(expected);
    let length = left.len().max(right.len());
    for index in 0..length {
        let a = left.get(index).copied().unwrap_or(0);
        let b = right.get(index).copied().unwrap_or(0);
        if a != b {
            return a.cmp(&b);
        }
    }
    std::cmp::Ordering::Equal
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .split(['.', '-'])
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn set_detected(report: &mut Value, key: &str, value: &Option<String>) {
    if let Some(value) = value {
        if let Some(detected) = report.get_mut("detected").and_then(Value::as_object_mut) {
            detected.insert(key.to_string(), Value::from(value.clone()));
        }
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

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{compare_versions, parse_labeled_version, parse_otp_version, version_parts};

    #[test]
    fn version_parts_accept_dash_suffixes() {
        assert_eq!(version_parts("1.20.1-otp-28"), vec![1, 20, 1, 28]);
    }

    #[test]
    fn compare_versions_pads_missing_parts() {
        assert!(compare_versions("1.20", "1.20.0").is_eq());
        assert!(compare_versions("28.1", "28.0").is_gt());
    }

    #[test]
    fn parses_elixir_mix_and_otp_versions() {
        let text = "Erlang/OTP 28 [erts]\nElixir 1.20.1\nMix 1.20.1";
        assert_eq!(
            parse_labeled_version(text, "Elixir").as_deref(),
            Some("1.20.1")
        );
        assert_eq!(
            parse_labeled_version(text, "Mix").as_deref(),
            Some("1.20.1")
        );
        assert_eq!(parse_otp_version(text).as_deref(), Some("28"));
    }
}
