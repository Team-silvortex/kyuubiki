use crate::native_time::utc_timestamp_slug;
use serde_json::{Value, json};
use std::env;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
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
    output_format: String,
    plan_out: Option<PathBuf>,
    profile_filter: Option<String>,
    repeat: String,
    show_shapes: bool,
    solver_preconditioner: String,
    sync_to_remote: Option<String>,
}

pub(crate) fn run_benchmark_profile_plan(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    let options = Options::from_env_and_args(root, args)?;
    if options.execute && options.output_format == "json" {
        return Err("benchmark-profile-plan JSON output is dry-run only".to_string());
    }
    let probes = select_probes(&read_manifest(&options.manifest_path)?, &options);

    if probes.is_empty() {
        let payload = plan_payload(options.execute, Vec::new());
        write_plan_out(root, &options, &payload)?;
        if options.output_format == "json" {
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).expect("plan should serialize")
            );
        } else {
            println!("benchmark profile plan has no matching probes");
        }
        return Ok(0);
    }

    let planned = build_planned_probes(root, &probes, &options)?;
    let payload = plan_payload(options.execute, planned.clone());
    write_plan_out(root, &options, &payload)?;
    if options.output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("plan should serialize")
        );
        return Ok(0);
    }

    println!(
        "benchmark profile plan: {} probe(s), mode={}",
        planned.len(),
        if options.execute {
            "execute"
        } else {
            "dry-run"
        }
    );

    for (planned_probe, probe) in planned.iter().zip(probes.iter()) {
        if let Some(shape) = planned_probe["shape"].as_object() {
            println!(
                "# shape case={} nodes={} elements={} dofs={}",
                planned_probe["case_id"].as_str().unwrap_or_default(),
                shape["node_count"].as_u64().unwrap_or(0),
                shape["element_count"].as_u64().unwrap_or(0),
                shape["dof_count"].as_u64().unwrap_or(0)
            );
        }
        println!("{}", planned_probe["command"].as_str().unwrap_or_default());
        if options.execute {
            let slug = planned_probe["output_slug"].as_str().unwrap_or_default();
            let status = run_probe(root, probe, &options, slug)?;
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
            output_format: env::var("FORMAT").unwrap_or_else(|_| "table".to_string()),
            plan_out: env_non_empty("PLAN_OUT").map(PathBuf::from),
            profile_filter: env_non_empty("PROFILE"),
            repeat: env::var("REPEAT").unwrap_or_else(|_| "1".to_string()),
            show_shapes: env::var("SHAPES").unwrap_or_else(|_| "0".into()) == "1",
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
                "--with-shapes" => options.show_shapes = true,
                "--no-shapes" => options.show_shapes = false,
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
                "--format" => {
                    options.output_format =
                        next_arg(&mut iter, "--format")?.unwrap_or_else(|| "table".to_string());
                }
                "--out" => {
                    let value = next_arg(&mut iter, "--out")?
                        .ok_or_else(|| "--out requires a value".to_string())?;
                    options.plan_out = Some(PathBuf::from(value));
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

fn build_planned_probes(
    root: &Path,
    probes: &[Probe],
    options: &Options,
) -> RunnerResult<Vec<Value>> {
    probes
        .iter()
        .enumerate()
        .map(|(index, probe)| {
            let slug = output_slug(index, probe, options);
            let shape = if options.show_shapes {
                shape_preview(root, probe)?
            } else {
                Value::Null
            };
            Ok(json!({
                "index": index + 1,
                "matrix": probe.matrix,
                "profile": probe.profile_cli,
                "profile_manifest": probe.profile_manifest,
                "case_id": probe.case_id,
                "repeat": options.repeat,
                "solver_preconditioner": options.solver_preconditioner,
                "output_slug": slug,
                "command": command_preview(probe, options, &slug),
                "shape": shape,
            }))
        })
        .collect()
}

fn plan_payload(execute: bool, planned: Vec<Value>) -> Value {
    json!({
        "mode": if execute { "execute" } else { "dry-run" },
        "probe_count": planned.len(),
        "probes": planned,
    })
}

fn write_plan_out(root: &Path, options: &Options, payload: &Value) -> RunnerResult<()> {
    let Some(path) = &options.plan_out else {
        return Ok(());
    };
    let path = resolve_repo_path(root, path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(payload)
        .map_err(|error| format!("plan json error: {error}"))?;
    std::fs::write(&path, format!("{content}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    if options.output_format == "json" {
        eprintln!("benchmark profile plan written to {}", path.display());
    } else {
        println!("benchmark profile plan json: {}", path.display());
    }
    Ok(())
}

fn resolve_repo_path(root: &Path, path: &Path) -> RunnerResult<PathBuf> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(format!(
            "benchmark-profile-plan output must not contain '..': {}",
            path.display()
        ));
    }
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    if !resolved.starts_with(root) {
        return Err(format!(
            "benchmark-profile-plan output must stay inside repo: {}",
            resolved.display()
        ));
    }
    Ok(resolved)
}

fn output_slug(index: usize, probe: &Probe, options: &Options) -> String {
    format!(
        "{}-{:03}-{}-{}-{}",
        options.output_slug_prefix,
        index + 1,
        sanitize_slug(&probe.matrix),
        sanitize_slug(&probe.profile_cli),
        sanitize_slug(&probe.case_id)
    )
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

fn shape_preview(root: &Path, probe: &Probe) -> RunnerResult<Value> {
    let output = Command::new("cargo")
        .current_dir(root.join("workers/rust"))
        .args([
            "run",
            "-q",
            "-p",
            "kyuubiki-benchmark",
            "--",
            "--dry-run-shapes",
            "--profile",
            &probe.profile_cli,
            "--matrix",
            &probe.matrix,
            "--case",
            &probe.case_id,
            "--format",
            "json",
        ])
        .output()
        .map_err(|error| format!("failed to run benchmark shape preview: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "benchmark shape preview failed with status {}",
            output.status
        ));
    }
    let report = serde_json::from_slice::<Value>(&output.stdout)
        .map_err(|error| format!("failed to parse benchmark shape preview: {error}"))?;
    let shape = report["cases"]
        .as_array()
        .and_then(|cases| {
            cases
                .iter()
                .find(|case| case["id"].as_str() == Some(probe.case_id.as_str()))
        })
        .ok_or_else(|| format!("shape preview did not include case {}", probe.case_id))?;
    Ok(json!({
        "node_count": shape["node_count"].as_u64().unwrap_or(0),
        "element_count": shape["element_count"].as_u64().unwrap_or(0),
        "dof_count": shape["dof_count"].as_u64().unwrap_or(0),
    }))
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
Environment / options:\n  PROFILE / --profile\n  MATRIX / --matrix\n  CASE / --case\n  LIMIT / --limit\n  COVERAGE_MANIFEST / --manifest\n  FORMAT=json / --format json\n  PLAN_OUT / --out\n  REPEAT\n  SOLVER_PRECONDITIONER\n  OUTPUT_SLUG\n  SYNC_TO_REMOTE\n  SHAPES=1 / --with-shapes\n  EXECUTE=1 / --execute\n"
    );
}

#[cfg(test)]
#[path = "benchmark_profile_plan_tests.rs"]
mod tests;
