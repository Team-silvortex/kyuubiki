use crate::native_time::utc_timestamp_slug;
use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Probe {
    matrix: String,
    profile_manifest: String,
    profile_cli: String,
    case_id: String,
}

struct Options {
    case_filter: Option<String>,
    execute: bool,
    limit: Option<usize>,
    manifest_path: PathBuf,
    matrix_filter: Option<String>,
    output_slug_prefix: String,
    profile_filter: Option<String>,
    repeat: String,
    solver_preconditioner: String,
    sync_to_remote: Option<String>,
}

pub(crate) fn run_benchmark_profile_plan(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    let options = Options::from_env_and_args(root, args)?;
    let probes = select_probes(&read_manifest(&options.manifest_path)?, &options);

    if probes.is_empty() {
        println!("benchmark profile plan has no matching probes");
        return Ok(0);
    }

    println!(
        "benchmark profile plan: {} probe(s), mode={}",
        probes.len(),
        if options.execute {
            "execute"
        } else {
            "dry-run"
        }
    );

    for (index, probe) in probes.iter().enumerate() {
        let slug = format!(
            "{}-{:03}-{}-{}-{}",
            options.output_slug_prefix,
            index + 1,
            sanitize_slug(&probe.matrix),
            sanitize_slug(&probe.profile_cli),
            sanitize_slug(&probe.case_id)
        );
        println!("{}", command_preview(probe, &options, &slug));
        if options.execute {
            let status = run_probe(root, probe, &options, &slug)?;
            if !status.success() {
                return Ok(status.code().unwrap_or(1) as u8);
            }
        }
    }

    Ok(0)
}

impl Options {
    fn from_env_and_args(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            case_filter: env_non_empty("CASE"),
            execute: env::var("EXECUTE").unwrap_or_else(|_| "0".into()) == "1",
            limit: env::var("LIMIT")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|value| *value > 0),
            manifest_path: env_path_or(
                "COVERAGE_MANIFEST",
                root.join("config/benchmark-profile-coverage.json"),
            ),
            matrix_filter: env_non_empty("MATRIX"),
            output_slug_prefix: env::var("OUTPUT_SLUG")
                .unwrap_or_else(|_| format!("benchmark-profile-plan-{}", utc_timestamp_slug())),
            profile_filter: env_non_empty("PROFILE"),
            repeat: env::var("REPEAT").unwrap_or_else(|_| "1".to_string()),
            solver_preconditioner: env::var("SOLVER_PRECONDITIONER")
                .unwrap_or_else(|_| "auto".to_string()),
            sync_to_remote: env_non_empty("SYNC_TO_REMOTE"),
        };

        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            let arg = arg
                .into_string()
                .map_err(|_| "benchmark-profile-plan arguments must be utf-8".to_string())?;
            match arg.as_str() {
                "--execute" => options.execute = true,
                "--dry-run" => options.execute = false,
                "--profile" => options.profile_filter = next_arg(&mut iter, "--profile")?,
                "--matrix" => options.matrix_filter = next_arg(&mut iter, "--matrix")?,
                "--case" => options.case_filter = next_arg(&mut iter, "--case")?,
                "--limit" => {
                    let value = next_arg(&mut iter, "--limit")?
                        .ok_or_else(|| "--limit requires a value".to_string())?;
                    let limit = value
                        .parse::<usize>()
                        .map_err(|_| format!("--limit must be a positive integer: {value}"))?;
                    options.limit = Some(limit.max(1));
                }
                "--manifest" => {
                    let value = next_arg(&mut iter, "--manifest")?
                        .ok_or_else(|| "--manifest requires a value".to_string())?;
                    options.manifest_path = PathBuf::from(value);
                }
                other => {
                    return Err(format!(
                        "unsupported benchmark-profile-plan argument: {other}"
                    ));
                }
            }
        }

        Ok(options)
    }
}

fn next_arg(iter: &mut impl Iterator<Item = OsString>, name: &str) -> RunnerResult<Option<String>> {
    iter.next()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| format!("{name} value must be utf-8"))
        })
        .transpose()
}

fn read_manifest(path: &Path) -> RunnerResult<Value> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn select_probes(manifest: &Value, options: &Options) -> Vec<Probe> {
    let Some(targets) = manifest["targets"].as_array() else {
        return Vec::new();
    };
    let mut probes = Vec::new();
    for target in targets {
        let Some(matrix) = target["matrix"].as_str() else {
            continue;
        };
        let Some(profile_manifest) = target["profile"].as_str() else {
            continue;
        };
        let profile_cli = profile_cli_name(profile_manifest);
        if !matches_filter(&options.matrix_filter, matrix)
            || !matches_profile_filter(&options.profile_filter, profile_manifest, &profile_cli)
        {
            continue;
        }
        let Some(cases) = target["expected_cases"].as_array() else {
            continue;
        };
        for case in cases.iter().filter_map(|entry| entry.as_str()) {
            if !matches_filter(&options.case_filter, case) {
                continue;
            }
            probes.push(Probe {
                matrix: matrix.to_string(),
                profile_manifest: profile_manifest.to_string(),
                profile_cli: profile_cli.clone(),
                case_id: case.to_string(),
            });
            if options.limit == Some(probes.len()) {
                return probes;
            }
        }
    }
    probes
}

fn matches_filter(filter: &Option<String>, value: &str) -> bool {
    filter
        .as_ref()
        .map(|filter| value.contains(filter))
        .unwrap_or(true)
}

fn matches_profile_filter(filter: &Option<String>, manifest: &str, cli: &str) -> bool {
    filter
        .as_ref()
        .map(|filter| manifest.contains(filter) || cli.contains(filter))
        .unwrap_or(true)
}

fn profile_cli_name(profile: &str) -> String {
    match profile {
        "ten_k" => "10k",
        "fifteen_k" => "15k",
        "twenty_k" => "20k",
        "hundred_k" => "100k",
        "two_hundred_k" => "200k",
        "three_hundred_k" => "300k",
        "four_hundred_k" => "400k",
        "five_hundred_k" => "500k",
        other => other,
    }
    .to_string()
}

fn command_preview(probe: &Probe, options: &Options, output_slug: &str) -> String {
    let mut parts = vec![
        format!("PROFILE={}", shell_word(&probe.profile_cli)),
        format!("MATRIX={}", shell_word(&probe.matrix)),
        format!("CASE={}", shell_word(&probe.case_id)),
        format!("REPEAT={}", shell_word(&options.repeat)),
        format!(
            "SOLVER_PRECONDITIONER={}",
            shell_word(&options.solver_preconditioner)
        ),
        format!("OUTPUT_SLUG={}", shell_word(output_slug)),
    ];
    if let Some(sync) = &options.sync_to_remote {
        parts.push(format!("SYNC_TO_REMOTE={}", shell_word(sync)));
    }
    parts.push("./scripts/kyuubiki benchmark-profile-remote".to_string());
    parts.join(" ")
}

fn run_probe(
    root: &Path,
    probe: &Probe,
    options: &Options,
    output_slug: &str,
) -> RunnerResult<std::process::ExitStatus> {
    let mut command = Command::new(root.join("scripts/kyuubiki"));
    command
        .arg("benchmark-profile-remote")
        .env("PROFILE", &probe.profile_cli)
        .env("MATRIX", &probe.matrix)
        .env("CASE", &probe.case_id)
        .env("REPEAT", &options.repeat)
        .env("SOLVER_PRECONDITIONER", &options.solver_preconditioner)
        .env("OUTPUT_SLUG", output_slug);
    if let Some(sync) = &options.sync_to_remote {
        command.env("SYNC_TO_REMOTE", sync);
    }
    command
        .status()
        .map_err(|error| format!("failed to run benchmark-profile-remote: {error}"))
}

fn env_non_empty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_path_or(name: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or(fallback)
}

fn sanitize_slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn shell_word(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki benchmark-profile-plan [--execute]\n\n\
Builds a case-by-case remote benchmark plan from the profile coverage manifest.\n\
Dry-run is the default; use EXECUTE=1 or --execute to run probes sequentially.\n\n\
Environment / options:\n  PROFILE / --profile\n  MATRIX / --matrix\n  CASE / --case\n  LIMIT / --limit\n  COVERAGE_MANIFEST / --manifest\n  REPEAT\n  SOLVER_PRECONDITIONER\n  OUTPUT_SLUG\n  SYNC_TO_REMOTE\n  EXECUTE=1 / --execute\n"
    );
}

#[cfg(test)]
mod tests {
    use super::{Options, profile_cli_name, select_probes};
    use serde_json::json;
    use std::path::PathBuf;

    fn options() -> Options {
        Options {
            case_filter: None,
            execute: false,
            limit: None,
            manifest_path: PathBuf::from("manifest.json"),
            matrix_filter: None,
            output_slug_prefix: "plan".to_string(),
            profile_filter: None,
            repeat: "1".to_string(),
            solver_preconditioner: "auto".to_string(),
            sync_to_remote: None,
        }
    }

    #[test]
    fn profile_names_map_to_cli_tokens() {
        assert_eq!(profile_cli_name("four_hundred_k"), "400k");
        assert_eq!(profile_cli_name("five_hundred_k"), "500k");
        assert_eq!(profile_cli_name("medium"), "medium");
    }

    #[test]
    fn selects_filtered_profile_matrix_cases() {
        let manifest = json!({
            "targets": [
                {
                    "matrix": "mechanical-core",
                    "profile": "five_hundred_k",
                    "expected_cases": ["axial-bar-500k", "truss-roof-500k"]
                },
                {
                    "matrix": "thermal-core",
                    "profile": "five_hundred_k",
                    "expected_cases": ["heat-plane-quad-500k"]
                }
            ]
        });
        let mut options = options();
        options.profile_filter = Some("500k".to_string());
        options.matrix_filter = Some("mechanical-core".to_string());
        options.case_filter = Some("axial".to_string());

        let probes = select_probes(&manifest, &options);

        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].profile_cli, "500k");
        assert_eq!(probes[0].case_id, "axial-bar-500k");
    }
}
