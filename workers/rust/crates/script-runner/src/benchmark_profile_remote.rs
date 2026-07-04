use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

struct Options {
    case_filter: Option<String>,
    local_json_path: PathBuf,
    local_md_path: PathBuf,
    local_output_dir: PathBuf,
    matrix: String,
    profile: String,
    remote_dir: String,
    remote_host: String,
    remote_json_path: String,
    repeat: String,
    rustup_toolchain: String,
    solver_preconditioner: String,
    sync_to_remote: bool,
}

pub(crate) fn run_benchmark_profile_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("benchmark-profile-remote does not accept positional arguments".into());
    }
    let options = Options::from_env(root);
    std::fs::create_dir_all(&options.local_output_dir).map_err(|error| {
        format!(
            "failed to create {}: {error}",
            options.local_output_dir.display()
        )
    })?;

    if options.sync_to_remote {
        sync_benchmark_sources(root, &options)?;
    }

    let status = run_status(
        "ssh",
        [
            OsString::from(&options.remote_host),
            OsString::from(remote_command(&options)),
        ],
        root,
    )?;
    if status != 0 {
        return Ok(status);
    }

    let scp_status = run_status(
        "scp",
        [
            OsString::from(format!(
                "{}:{}",
                options.remote_host, options.remote_json_path
            )),
            options.local_json_path.clone().into_os_string(),
        ],
        root,
    )?;
    if scp_status != 0 {
        return Ok(scp_status);
    }

    write_markdown_summary(&options.local_json_path, &options.local_md_path)?;
    println!(
        "remote benchmark profile completed on {}",
        options.remote_host
    );
    println!("json: {}", options.local_json_path.display());
    println!("summary: {}", options.local_md_path.display());
    Ok(0)
}

impl Options {
    fn from_env(root: &Path) -> Self {
        let profile = env::var("PROFILE").unwrap_or_else(|_| "200k".to_string());
        let matrix = env::var("MATRIX").unwrap_or_else(|_| "thermal-core".to_string());
        let case_filter = env::var("CASE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let output_name = output_name(&matrix, &profile, case_filter.as_deref());
        let output_slug = env::var("OUTPUT_SLUG").unwrap_or_else(|_| {
            format!(
                "benchmark-profile-{}",
                timestamp_slug().unwrap_or_else(|| "manual".to_string())
            )
        });
        let remote_dir = env::var("KYUUBIKI_LAB_BENCH_DIR")
            .unwrap_or_else(|_| "/tmp/kyuubiki-server-test".to_string());
        let remote_output_dir = env::var("REMOTE_OUTPUT_DIR")
            .unwrap_or_else(|_| format!("/tmp/kyuubiki-benchmark-profile/{output_slug}"));
        let remote_json_path = if remote_output_dir.starts_with('/') {
            format!("{remote_output_dir}/{output_name}.json")
        } else {
            format!("{remote_dir}/{remote_output_dir}/{output_name}.json")
        };
        let local_output_dir = env_path_or(
            "LOCAL_OUTPUT_DIR",
            root.join("tmp/benchmark-profile").join(&output_slug),
        );
        Self {
            case_filter,
            local_json_path: local_output_dir.join(format!("{output_name}.json")),
            local_md_path: local_output_dir.join("README.md"),
            local_output_dir,
            matrix,
            profile,
            remote_dir,
            remote_host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".into()),
            remote_json_path,
            repeat: env::var("REPEAT").unwrap_or_else(|_| "3".to_string()),
            rustup_toolchain: env::var("RUSTUP_TOOLCHAIN_OVERRIDE")
                .unwrap_or_else(|_| "stable".to_string()),
            solver_preconditioner: env::var("SOLVER_PRECONDITIONER")
                .unwrap_or_else(|_| "jacobi".to_string()),
            sync_to_remote: env::var("SYNC_TO_REMOTE").unwrap_or_else(|_| "1".into()) == "1",
        }
    }
}

fn sync_benchmark_sources(root: &Path, options: &Options) -> RunnerResult<()> {
    ensure_remote_sync_dirs(root, options)?;
    let status = rsync(
        root,
        &[root.join("workers/rust/")],
        &format!(
            "{}:{}/workers/rust/",
            options.remote_host, options.remote_dir
        ),
    )?;
    if status != 0 {
        return Err(format!("rsync failed with status {status}"));
    }
    Ok(())
}

fn ensure_remote_sync_dirs(root: &Path, options: &Options) -> RunnerResult<()> {
    let status = run_status(
        "ssh",
        [
            OsString::from(&options.remote_host),
            OsString::from(format!(
                "mkdir -p {}",
                shell_escape(&format!("{}/workers", options.remote_dir))
            )),
        ],
        root,
    )?;
    if status != 0 {
        return Err(format!("remote mkdir failed with status {status}"));
    }
    Ok(())
}

fn remote_command(options: &Options) -> String {
    let case_arg = options
        .case_filter
        .as_deref()
        .map(|case| format!(" --case {}", shell_escape(case)))
        .unwrap_or_default();
    format!(
        "set -euo pipefail; mkdir -p {}; cd {}/workers/rust; RUSTUP_TOOLCHAIN={} cargo run --release -q -p kyuubiki-benchmark -- --profile {} --matrix {} --repeat {} --format json --solver-preconditioner {} --progress{} > {}",
        shell_escape(&dirname(&options.remote_json_path)),
        shell_escape(&options.remote_dir),
        shell_escape(&options.rustup_toolchain),
        shell_escape(&options.profile),
        shell_escape(&options.matrix),
        shell_escape(&options.repeat),
        shell_escape(&options.solver_preconditioner),
        case_arg,
        shell_escape(&options.remote_json_path)
    )
}

fn write_markdown_summary(json_path: &Path, md_path: &Path) -> RunnerResult<()> {
    let content = std::fs::read_to_string(json_path)
        .map_err(|error| format!("failed to read {}: {error}", json_path.display()))?;
    let report: Value = serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", json_path.display()))?;
    let cases = report["cases"].as_array().ok_or_else(|| {
        format!(
            "benchmark profile report is missing cases array: {}",
            json_path.display()
        )
    })?;
    let mut output = File::create(md_path)
        .map_err(|error| format!("failed to create {}: {error}", md_path.display()))?;
    writeln!(output, "# Benchmark profile smoke\n")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Profile: `{}`", string_field(&report, "profile"))
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Matrix: `{}`", string_field(&report, "matrix"))
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Repeat: `{}`", number_field(&report, "repeat"))
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Case count: `{}`\n", cases.len())
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(
        output,
        "| Case | Nodes | Elements | Median ms | Peak RSS MiB | Solver | Iterations | Residual |"
    )
    .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "|---|---:|---:|---:|---:|---|---:|---:|")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    for entry in cases {
        let peak_rss = entry["peak_rss_kib"]
            .as_f64()
            .map(|value| format!("{:.1}", value / 1024.0))
            .unwrap_or_else(|| "--".to_string());
        let solver = string_field(entry, "solver_preconditioner");
        let iterations = number_field(entry, "solver_iterations");
        let residual = entry["solver_residual_norm"]
            .as_f64()
            .map(|value| format!("{value:.3e}"))
            .unwrap_or_else(|| "--".to_string());
        writeln!(
            output,
            "| `{}` | {} | {} | {:.3} | {} | `{}` | {} | {} |",
            string_field(entry, "id"),
            number_field(entry, "node_count"),
            number_field(entry, "element_count"),
            entry["median_ms"].as_f64().unwrap_or(0.0),
            peak_rss,
            solver,
            iterations,
            residual
        )
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    }
    write_solver_comparison(&mut output, cases)?;
    Ok(())
}

fn write_solver_comparison(output: &mut File, cases: &[Value]) -> RunnerResult<()> {
    let pairs = solver_pairs(cases);
    if pairs.is_empty() {
        return Ok(());
    }

    writeln!(output, "\n## Solver Strategy Comparison\n")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(
        output,
        "| Base Case | Reference | Candidate | Median Delta | Solve Delta | Iteration Delta | Peak RSS Delta |"
    )
    .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "|---|---|---|---:|---:|---:|---:|")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    for (base, reference, candidate) in pairs {
        writeln!(
            output,
            "| `{}` | `{}` | `{}` | {} | {} | {} | {} |",
            base,
            string_field(reference, "solver_preconditioner"),
            string_field(candidate, "solver_preconditioner"),
            delta_pct(
                reference["median_ms"].as_f64(),
                candidate["median_ms"].as_f64()
            ),
            delta_pct(
                stage_elapsed_ms(reference, "solve_system"),
                stage_elapsed_ms(candidate, "solve_system")
            ),
            delta_pct(
                reference["solver_iterations"].as_f64(),
                candidate["solver_iterations"].as_f64()
            ),
            delta_pct(
                reference["peak_rss_kib"].as_f64(),
                candidate["peak_rss_kib"].as_f64()
            )
        )
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    }
    Ok(())
}

fn solver_pairs<'a>(cases: &'a [Value]) -> Vec<(String, &'a Value, &'a Value)> {
    cases
        .iter()
        .filter_map(|reference| {
            let id = string_field(reference, "id");
            let base = id.strip_suffix("#jacobi")?.to_string();
            let candidate_id = format!("{base}#symmetric-gauss-seidel");
            cases
                .iter()
                .find(|case| string_field(case, "id") == candidate_id)
                .map(|candidate| (base, reference, candidate))
        })
        .collect()
}

fn delta_pct(reference: Option<f64>, candidate: Option<f64>) -> String {
    let (Some(reference), Some(candidate)) = (reference, candidate) else {
        return "--".to_string();
    };
    if reference.abs() <= f64::EPSILON {
        return "--".to_string();
    }
    format!("{:+.2}%", ((candidate - reference) / reference) * 100.0)
}

fn stage_elapsed_ms(value: &Value, label: &str) -> Option<f64> {
    value["memory_stages"]
        .as_array()?
        .iter()
        .find(|stage| stage["label"].as_str() == Some(label))
        .and_then(|stage| stage["elapsed_ms"].as_f64())
}

fn rsync(root: &Path, sources: &[PathBuf], destination: &str) -> RunnerResult<u8> {
    run_status(
        "rsync",
        [OsString::from("-az"), OsString::from("--exclude=target/")]
            .into_iter()
            .chain(sources.iter().map(|path| path.clone().into_os_string()))
            .chain([OsString::from(destination)]),
        root,
    )
}

fn run_status<I>(program: &str, args: I, cwd: &Path) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    Ok(status.code().unwrap_or(1) as u8)
}

fn env_path_or(name: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or(fallback)
}

fn output_name(matrix: &str, profile: &str, case_filter: Option<&str>) -> String {
    match case_filter {
        Some(case) => format!("{matrix}-{profile}-{}", sanitize_file_stem(case)),
        None => format!("{matrix}-{profile}"),
    }
}

fn sanitize_file_stem(value: &str) -> String {
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

fn dirname(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(dir, _)| dir.to_string())
        .unwrap_or_else(|| ".".to_string())
}

fn number_field(value: &Value, name: &str) -> String {
    value[name]
        .as_i64()
        .map(|number| number.to_string())
        .or_else(|| value[name].as_u64().map(|number| number.to_string()))
        .or_else(|| value[name].as_f64().map(|number| number.to_string()))
        .unwrap_or_else(|| "--".to_string())
}

fn string_field(value: &Value, name: &str) -> String {
    value[name].as_str().unwrap_or("--").to_string()
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn timestamp_slug() -> Option<String> {
    let output = Command::new("date")
        .args(["-u", "+%Y%m%dT%H%M%SZ"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki benchmark-profile-remote\n\n\
Runs one Rust benchmark profile/matrix on the shared lab machine without a\n\
checked baseline, then copies JSON back and writes a Markdown summary.\n\n\
Environment:\n  KYUUBIKI_LAB_HOST\n  KYUUBIKI_LAB_BENCH_DIR\n  PROFILE\n  MATRIX\n  CASE\n  REPEAT\n  RUSTUP_TOOLCHAIN_OVERRIDE\n  SOLVER_PRECONDITIONER\n  OUTPUT_SLUG\n  LOCAL_OUTPUT_DIR\n  REMOTE_OUTPUT_DIR\n  SYNC_TO_REMOTE\n"
    );
}

#[cfg(test)]
mod tests {
    use super::{delta_pct, solver_pairs, stage_elapsed_ms};
    use serde_json::json;

    #[test]
    fn solver_pairs_match_jacobi_and_sgs_rows() {
        let cases = vec![
            json!({
                "id": "truss-roof-300k#jacobi",
                "solver_preconditioner": "jacobi",
                "memory_stages": [
                    { "label": "solve_system", "elapsed_ms": 100.0 }
                ]
            }),
            json!({
                "id": "truss-roof-300k#symmetric-gauss-seidel",
                "solver_preconditioner": "symmetric-gauss-seidel",
                "memory_stages": [
                    { "label": "solve_system", "elapsed_ms": 75.0 }
                ]
            }),
        ];

        let pairs = solver_pairs(&cases);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "truss-roof-300k");
        assert_eq!(stage_elapsed_ms(pairs[0].1, "solve_system"), Some(100.0));
        assert_eq!(stage_elapsed_ms(pairs[0].2, "solve_system"), Some(75.0));
    }

    #[test]
    fn delta_pct_formats_signed_relative_change() {
        assert_eq!(delta_pct(Some(100.0), Some(75.0)), "-25.00%");
        assert_eq!(delta_pct(Some(100.0), Some(125.0)), "+25.00%");
        assert_eq!(delta_pct(Some(0.0), Some(1.0)), "--");
    }
}
