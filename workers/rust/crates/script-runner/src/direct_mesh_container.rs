use crate::native_time::{utc_iso_timestamp, utc_timestamp_slug};
use std::env;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

macro_rules! write_line {
    ($target:expr, $($arg:tt)*) => {
        writeln!($target, $($arg)*).map_err(|error| format!("failed to write file: {error}"))?
    };
}

#[derive(Clone)]
struct Options {
    base_image: String,
    docker_build_network: Option<String>,
    docker_run_network: Option<String>,
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    image_tag: String,
    keep_containers: bool,
    no_proxy: Option<String>,
    output_dir: PathBuf,
    repeat: usize,
    skip_build: bool,
}

#[derive(Default)]
struct TimeMetrics {
    elapsed_s: f64,
    max_rss_kib: f64,
    sys_s: f64,
    user_s: f64,
}

#[derive(Default)]
struct Aggregate {
    count: usize,
    max: f64,
    min: f64,
    sum: f64,
}

pub(crate) fn run_direct_mesh_benchmark_container(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    let options = parse_options(root, args)?;
    std::fs::create_dir_all(&options.output_dir)
        .map_err(|error| format!("failed to create {}: {error}", options.output_dir.display()))?;
    File::create(options.output_dir.join("time-index.txt"))
        .map_err(|error| format!("failed to reset time-index.txt: {error}"))?;

    if !options.skip_build {
        docker_build(root, &options)?;
    }
    for run_index in 1..=options.repeat {
        println!(
            "==> direct-mesh docker benchmark run {run_index}/{}",
            options.repeat
        );
        let status = run_container_benchmark(run_index, &options)?;
        if status != 0 {
            return Ok(status);
        }
    }
    write_root_summary(root, &options)?;
    println!(
        "wrote benchmark artifacts to {}",
        options.output_dir.display()
    );
    println!(
        "summary: {}",
        options.output_dir.join("summary.json").display()
    );
    Ok(0)
}

fn parse_options(root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let timestamp = utc_timestamp_slug();
    let mut options = Options {
        base_image: env::var("BASE_IMAGE").unwrap_or_else(|_| "elixir:1.19".to_string()),
        docker_build_network: env_nonempty("DOCKER_BUILD_NETWORK"),
        docker_run_network: env_nonempty("DOCKER_RUN_NETWORK"),
        http_proxy: env_nonempty("HTTP_PROXY").or_else(|| env_nonempty("http_proxy")),
        https_proxy: env_nonempty("HTTPS_PROXY").or_else(|| env_nonempty("https_proxy")),
        image_tag: env::var("IMAGE_TAG")
            .unwrap_or_else(|_| "kyuubiki/direct-mesh-benchmark:local".to_string()),
        keep_containers: false,
        no_proxy: env_nonempty("NO_PROXY").or_else(|| env_nonempty("no_proxy")),
        output_dir: root
            .join("tmp/direct-mesh-benchmark-container")
            .join(timestamp),
        repeat: env::var("REPEAT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3),
        skip_build: false,
    };

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--repeat" => options.repeat = parse_usize(next_arg(&mut iter, "--repeat")?)?,
            "--output-dir" => {
                options.output_dir = PathBuf::from(next_arg(&mut iter, "--output-dir")?)
            }
            "--image-tag" => options.image_tag = next_arg(&mut iter, "--image-tag")?,
            "--base-image" => options.base_image = next_arg(&mut iter, "--base-image")?,
            "--skip-build" => options.skip_build = true,
            "--keep-containers" => options.keep_containers = true,
            other => return Err(format!("unknown option: {other}")),
        }
    }
    if options.repeat == 0 {
        return Err("--repeat must be at least 1".to_string());
    }
    Ok(options)
}

fn docker_build(root: &Path, options: &Options) -> RunnerResult<()> {
    let mut args = Vec::<OsString>::new();
    args.push(OsString::from("build"));
    if let Some(network) = &options.docker_build_network {
        args.extend([OsString::from("--network"), OsString::from(network)]);
    }
    args.extend([
        OsString::from("--build-arg"),
        format!("BASE_IMAGE={}", options.base_image).into(),
    ]);
    push_proxy_build_args(&mut args, options);
    args.extend([
        OsString::from("-f"),
        root.join("deploy/docker/direct-mesh-benchmark.Dockerfile")
            .into_os_string(),
        OsString::from("-t"),
        OsString::from(&options.image_tag),
        root.as_os_str().to_owned(),
    ]);
    run_status("docker", args, root).map(|_| ())
}

fn run_container_benchmark(run_index: usize, options: &Options) -> RunnerResult<u8> {
    let run_dir = options.output_dir.join(format!("run-{run_index:02}"));
    std::fs::create_dir_all(&run_dir)
        .map_err(|error| format!("failed to create {}: {error}", run_dir.display()))?;
    let mut args = Vec::<OsString>::new();
    args.extend([OsString::from("run"), OsString::from("--name")]);
    args.push(OsString::from(format!(
        "kyuubiki-direct-mesh-bench-{}-{run_index}",
        utc_timestamp_slug()
    )));
    if !options.keep_containers {
        args.push(OsString::from("--rm"));
    }
    if let Some(network) = &options.docker_run_network {
        args.extend([OsString::from("--network"), OsString::from(network)]);
    }
    args.extend([
        OsString::from("-v"),
        OsString::from(format!("{}:/artifacts", run_dir.display())),
    ]);
    push_proxy_run_args(&mut args, options);
    args.push(OsString::from(&options.image_tag));
    push_container_benchmark_command(&mut args);
    let status = run_status_with_log("docker", args, Path::new("."), &run_dir.join("test.log"))?;
    if status != 0 {
        return Ok(status);
    }
    extract_subtests(&run_dir.join("test.log"), &run_dir.join("subtests.tsv"))?;
    write_run_summary(&run_dir, run_index)?;
    append_time_index(&options.output_dir, &run_dir.join("time.txt"))?;
    Ok(0)
}

fn write_run_summary(run_dir: &Path, run_index: usize) -> RunnerResult<()> {
    let metrics = read_time_metrics(&run_dir.join("time.txt"))?;
    let subtests = read_subtests(&run_dir.join("subtests.tsv"))?;
    let mut output = File::create(run_dir.join("summary.json"))
        .map_err(|error| format!("failed to create run summary: {error}"))?;
    write_line!(output, "{{");
    write_line!(output, "  \"run_index\": {run_index},");
    write_line!(output, "  \"elapsed_s\": {:.3},", metrics.elapsed_s);
    write_line!(output, "  \"user_s\": {:.3},", metrics.user_s);
    write_line!(output, "  \"sys_s\": {:.3},", metrics.sys_s);
    write_line!(output, "  \"max_rss_kib\": {:.0},", metrics.max_rss_kib);
    write_line!(output, "  \"subtests\": [");
    for (index, (name, duration)) in subtests.iter().enumerate() {
        let comma = if index + 1 == subtests.len() { "" } else { "," };
        write_line!(
            output,
            "    {{\"name\": \"{}\", \"duration_ms\": {duration}}}{comma}",
            json_escape(name)
        );
    }
    write_line!(output, "  ]");
    write_line!(output, "}}");
    Ok(())
}

fn write_root_summary(root: &Path, options: &Options) -> RunnerResult<()> {
    let git_sha = command_output(
        "git",
        ["-C", root.to_str().unwrap_or("."), "rev-parse", "HEAD"],
    )
    .unwrap_or_else(|| "unknown".to_string());
    let mut elapsed = Aggregate::default();
    let mut user = Aggregate::default();
    let mut sys = Aggregate::default();
    let mut rss = Aggregate::default();
    for run_index in 1..=options.repeat {
        let metrics = read_time_metrics(
            &options
                .output_dir
                .join(format!("run-{run_index:02}"))
                .join("time.txt"),
        )?;
        elapsed.add(metrics.elapsed_s);
        user.add(metrics.user_s);
        sys.add(metrics.sys_s);
        rss.add(metrics.max_rss_kib);
    }
    write_summary_json(options, &git_sha, &elapsed, &user, &sys, &rss)?;
    write_summary_md(options, &git_sha, &elapsed, &user, &sys, &rss)
}

fn write_summary_json(
    options: &Options,
    git_sha: &str,
    elapsed: &Aggregate,
    user: &Aggregate,
    sys: &Aggregate,
    rss: &Aggregate,
) -> RunnerResult<()> {
    let mut output = File::create(options.output_dir.join("summary.json"))
        .map_err(|error| format!("failed to create root summary: {error}"))?;
    write_line!(output, "{{");
    write_line!(output, "  \"generated_at\": \"{}\",", utc_iso_timestamp());
    write_line!(output, "  \"git_sha\": \"{}\",", json_escape(git_sha));
    write_line!(
        output,
        "  \"image_tag\": \"{}\",",
        json_escape(&options.image_tag)
    );
    write_line!(output, "  \"repeat\": {},", options.repeat);
    write_line!(output, "  \"aggregate\": {{");
    write_aggregate_json(&mut output, "elapsed_s", elapsed, true)?;
    write_aggregate_json(&mut output, "user_s", user, true)?;
    write_aggregate_json(&mut output, "sys_s", sys, true)?;
    write_aggregate_json(&mut output, "max_rss_kib", rss, false)?;
    write_line!(output, "  }},");
    write_line!(output, "  \"runs\": [");
    for run_index in 1..=options.repeat {
        let path = options
            .output_dir
            .join(format!("run-{run_index:02}"))
            .join("summary.json");
        let content = std::fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let comma = if run_index == options.repeat { "" } else { "," };
        let lines: Vec<&str> = content.lines().collect();
        for (line_index, line) in lines.iter().enumerate() {
            if line_index + 1 == lines.len() && !comma.is_empty() {
                write_line!(output, "    {line}{comma}");
            } else {
                write_line!(output, "    {line}");
            }
        }
    }
    write_line!(output, "  ]");
    write_line!(output, "}}");
    Ok(())
}

fn write_summary_md(
    options: &Options,
    git_sha: &str,
    elapsed: &Aggregate,
    user: &Aggregate,
    sys: &Aggregate,
    rss: &Aggregate,
) -> RunnerResult<()> {
    let mut output = File::create(options.output_dir.join("summary.md"))
        .map_err(|error| format!("failed to create summary markdown: {error}"))?;
    write_line!(output, "# Direct-mesh Docker benchmark\n");
    write_line!(output, "- Git SHA: `{git_sha}`");
    write_line!(output, "- Image: `{}`", options.image_tag);
    write_line!(output, "- Repeat: `{}`", options.repeat);
    write_line!(output, "- Output: `{}`\n", options.output_dir.display());
    write_line!(output, "| Metric | Min | Max | Mean |");
    write_line!(output, "| --- | ---: | ---: | ---: |");
    write_md_metric(&mut output, "Elapsed (s)", elapsed)?;
    write_md_metric(&mut output, "User CPU (s)", user)?;
    write_md_metric(&mut output, "System CPU (s)", sys)?;
    write_md_metric(&mut output, "Peak RSS (KiB)", rss)
}

impl Aggregate {
    fn add(&mut self, value: f64) {
        if self.count == 0 || value < self.min {
            self.min = value;
        }
        if self.count == 0 || value > self.max {
            self.max = value;
        }
        self.count += 1;
        self.sum += value;
    }

    fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
}

fn extract_subtests(log_path: &Path, output_path: &Path) -> RunnerResult<()> {
    let input = BufReader::new(
        File::open(log_path)
            .map_err(|error| format!("failed to open test log {}: {error}", log_path.display()))?,
    );
    let mut output = File::create(output_path).map_err(|error| {
        format!(
            "failed to create subtest file {}: {error}",
            output_path.display()
        )
    })?;
    let mut current_name: Option<String> = None;
    for line in input.lines() {
        let line = line.map_err(|error| format!("failed to read test log: {error}"))?;
        if let Some(name) = line.strip_prefix("# Subtest: ") {
            current_name = Some(name.to_string());
            continue;
        }
        if let Some(value) = line.trim().strip_prefix("duration_ms: ") {
            if let Some(name) = current_name.take() {
                let duration = value.trim_end_matches(',').trim();
                write_line!(output, "{name}\t{duration}");
            }
        }
    }
    Ok(())
}

fn read_subtests(path: &Path) -> RunnerResult<Vec<(String, String)>> {
    let mut entries = Vec::new();
    let input = BufReader::new(
        File::open(path)
            .map_err(|error| format!("failed to open subtest file {}: {error}", path.display()))?,
    );
    for line in input.lines() {
        let line = line.map_err(|error| format!("failed to read subtest file: {error}"))?;
        if let Some((name, duration)) = line.split_once('\t') {
            entries.push((name.to_string(), duration.to_string()));
        }
    }
    Ok(entries)
}

fn read_time_metrics(path: &Path) -> RunnerResult<TimeMetrics> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut metrics = TimeMetrics::default();
    for token in content.split_whitespace() {
        if let Some((key, value)) = token.split_once('=') {
            let parsed = value.parse::<f64>().unwrap_or(0.0);
            match key {
                "elapsed_s" => metrics.elapsed_s = parsed,
                "user_s" => metrics.user_s = parsed,
                "sys_s" => metrics.sys_s = parsed,
                "max_rss_kib" => metrics.max_rss_kib = parsed,
                _ => {}
            }
        }
    }
    Ok(metrics)
}

fn append_time_index(output_dir: &Path, time_file: &Path) -> RunnerResult<()> {
    let content = std::fs::read_to_string(time_file)
        .map_err(|error| format!("failed to read {}: {error}", time_file.display()))?;
    let mut output = OpenOptions::new()
        .append(true)
        .open(output_dir.join("time-index.txt"))
        .map_err(|error| format!("failed to append time-index.txt: {error}"))?;
    write_line!(output, "{}", content.trim());
    Ok(())
}

fn push_proxy_build_args(args: &mut Vec<OsString>, options: &Options) {
    for (key, value) in proxy_pairs(options) {
        args.extend([
            OsString::from("--build-arg"),
            OsString::from(format!("{key}={value}")),
        ]);
    }
}

fn push_proxy_run_args(args: &mut Vec<OsString>, options: &Options) {
    for (key, value) in proxy_pairs(options) {
        args.extend([
            OsString::from("-e"),
            OsString::from(format!("{key}={value}")),
        ]);
    }
}

fn proxy_pairs(options: &Options) -> Vec<(&'static str, &str)> {
    let mut pairs = Vec::new();
    if let Some(value) = &options.http_proxy {
        pairs.extend([
            ("HTTP_PROXY", value.as_str()),
            ("http_proxy", value.as_str()),
        ]);
    }
    if let Some(value) = &options.https_proxy {
        pairs.extend([
            ("HTTPS_PROXY", value.as_str()),
            ("https_proxy", value.as_str()),
        ]);
    }
    if let Some(value) = &options.no_proxy {
        pairs.extend([("NO_PROXY", value.as_str()), ("no_proxy", value.as_str())]);
    }
    pairs
}

fn push_container_benchmark_command(args: &mut Vec<OsString>) {
    args.extend([
        OsString::from("/usr/bin/time"),
        OsString::from("-f"),
        OsString::from("elapsed_s=%e user_s=%U sys_s=%S max_rss_kib=%M"),
        OsString::from("-o"),
        OsString::from("/artifacts/time.txt"),
        OsString::from("node"),
        OsString::from("--test"),
        OsString::from("tests/integration/direct-mesh-gui-smoke.test.mjs"),
    ]);
}

fn write_aggregate_json(
    output: &mut File,
    name: &str,
    value: &Aggregate,
    comma: bool,
) -> RunnerResult<()> {
    let trailing = if comma { "," } else { "" };
    write_line!(
        output,
        "    \"{name}\": {{\"min\": {:.3}, \"max\": {:.3}, \"mean\": {:.3}}}{trailing}",
        value.min,
        value.max,
        value.mean()
    );
    Ok(())
}

fn write_md_metric(output: &mut File, label: &str, value: &Aggregate) -> RunnerResult<()> {
    write_line!(
        output,
        "| {label} | {:.3} | {:.3} | {:.3} |",
        value.min,
        value.max,
        value.mean()
    );
    Ok(())
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

fn run_status_with_log<I>(program: &str, args: I, cwd: &Path, log_path: &Path) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    let mut log = File::create(log_path)
        .map_err(|error| format!("failed to create {}: {error}", log_path.display()))?;
    log.write_all(&output.stdout)
        .map_err(|error| format!("failed to write {}: {error}", log_path.display()))?;
    log.write_all(&output.stderr)
        .map_err(|error| format!("failed to write {}: {error}", log_path.display()))?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    Ok(output.status.code().unwrap_or(1) as u8)
}

fn next_arg(iter: &mut impl Iterator<Item = OsString>, name: &str) -> RunnerResult<String> {
    iter.next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(|| format!("{name} requires a value"))
}

fn parse_usize(value: String) -> RunnerResult<usize> {
    value
        .parse()
        .map_err(|_| format!("expected positive integer, got {value}"))
}

fn env_nonempty(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn command_output<const N: usize>(program: &str, args: [&str; N]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki direct-mesh-benchmark-container [options]\n\n\
Options:\n  --repeat <count>\n  --output-dir <path>\n  --image-tag <tag>\n  --base-image <image>\n  --skip-build\n  --keep-containers\n"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_benchmark_command_avoids_shell_and_make() {
        let mut args = Vec::new();
        push_container_benchmark_command(&mut args);
        let rendered = args
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert!(!rendered.iter().any(|arg| arg == "bash" || arg == "sh"));
        assert!(!rendered.iter().any(|arg| arg == "make"));
        assert_eq!(rendered.first().map(String::as_str), Some("/usr/bin/time"));
        assert!(rendered.contains(&"node".to_string()));
        assert!(rendered.contains(&"tests/integration/direct-mesh-gui-smoke.test.mjs".to_string()));
    }
}
