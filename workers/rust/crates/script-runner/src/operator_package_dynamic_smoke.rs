use crate::{RepoPaths, RunnerResult, native_time::utc_iso_timestamp, run_command};
use serde_json::{Value, json};
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

mod check;

pub(crate) use check::{
    run_check_operator_package_dynamic_smoke, run_check_operator_package_dynamic_smoke_contract,
};

#[derive(Debug, Clone)]
struct Options {
    output_path: PathBuf,
}

#[derive(Debug, Clone)]
struct StageRecord {
    id: &'static str,
    description: &'static str,
    cwd: String,
    command: Vec<String>,
    status: u8,
}

pub fn run_operator_package_dynamic_smoke(
    paths: &RepoPaths,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(paths, args)?;
    let template_manifest = paths
        .root
        .join("workers/rust/templates/operator-crate-template/Cargo.toml");
    let template_root = paths.root.join("workers/rust/templates");
    let preflight_report_path = options
        .output_path
        .parent()
        .unwrap_or_else(|| paths.root.as_path())
        .join("operator-package-dynamic-preflight.json");
    let dynamic_library_path = paths
        .root
        .join("workers/rust/templates/operator-crate-template/target/debug")
        .join(dynamic_library_file_name());
    let root_cwd = ".".to_string();
    let rust_cwd = repo_relative(paths, &paths.rust);
    let template_manifest_report = repo_relative(paths, &template_manifest);
    let template_root_from_rust = relative_to(paths, &paths.rust, &template_root);
    let preflight_report_path_from_rust = relative_to(paths, &paths.rust, &preflight_report_path);
    let mut stages = Vec::new();

    let template_tests = run_command(
        &paths.root,
        "cargo",
        [
            "test",
            "--manifest-path",
            template_manifest.to_string_lossy().as_ref(),
            "--",
            "--nocapture",
        ]
        .map(OsString::from),
    )?;
    stages.push(stage_record(
        "template_tests",
        "Run template operator crate tests and descriptor readiness checks.",
        root_cwd.clone(),
        [
            "cargo",
            "test",
            "--manifest-path",
            template_manifest_report.as_str(),
            "--",
            "--nocapture",
        ],
        template_tests,
    ));
    if template_tests != 0 {
        write_report(
            paths,
            &options,
            &stages,
            &preflight_report_path,
            &dynamic_library_path,
        )?;
        return Ok(template_tests);
    }

    let preflight = run_command(
        &paths.rust,
        "cargo",
        [
            "run",
            "-p",
            "kyuubiki-installer",
            "--",
            "operator-package-preflight",
            template_root.to_string_lossy().as_ref(),
            "--out",
            preflight_report_path.to_string_lossy().as_ref(),
            "--fail-on-rejected",
            "--fail-on-readiness-warnings",
        ]
        .map(OsString::from),
    )?;
    stages.push(stage_record(
        "strict_preflight",
        "Run strict read-only package preflight with rejection and readiness-warning gates.",
        rust_cwd.clone(),
        [
            "cargo",
            "run",
            "-p",
            "kyuubiki-installer",
            "--",
            "operator-package-preflight",
            template_root_from_rust.as_str(),
            "--out",
            preflight_report_path_from_rust.as_str(),
            "--fail-on-rejected",
            "--fail-on-readiness-warnings",
        ],
        preflight,
    ));
    if preflight != 0 {
        write_report(
            paths,
            &options,
            &stages,
            &preflight_report_path,
            &dynamic_library_path,
        )?;
        return Ok(preflight);
    }

    let build = run_command(
        &paths.root,
        "cargo",
        [
            "build",
            "--manifest-path",
            template_manifest.to_string_lossy().as_ref(),
        ]
        .map(OsString::from),
    )?;
    stages.push(stage_record(
        "template_cdylib_build",
        "Build the template operator dynamic library for host loading.",
        root_cwd,
        [
            "cargo",
            "build",
            "--manifest-path",
            template_manifest_report.as_str(),
        ],
        build,
    ));
    if build != 0 {
        write_report(
            paths,
            &options,
            &stages,
            &preflight_report_path,
            &dynamic_library_path,
        )?;
        return Ok(build);
    }

    let dynamic_host = run_command(
        &paths.rust,
        "cargo",
        [
            "test",
            "-p",
            "kyuubiki-engine",
            "loads_prebuilt_template_cdylib_through_dynamic_host",
            "--",
            "--ignored",
            "--nocapture",
        ]
        .map(OsString::from),
    )?;
    stages.push(stage_record(
        "engine_dynamic_host_load",
        "Run the engine dynamic host test that loads and dispatches the template operator.",
        rust_cwd,
        [
            "cargo",
            "test",
            "-p",
            "kyuubiki-engine",
            "loads_prebuilt_template_cdylib_through_dynamic_host",
            "--",
            "--ignored",
            "--nocapture",
        ],
        dynamic_host,
    ));
    write_report(
        paths,
        &options,
        &stages,
        &preflight_report_path,
        &dynamic_library_path,
    )?;
    Ok(dynamic_host)
}

fn parse_options(paths: &RepoPaths, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut output_path = env::var_os("OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| paths.root.join("tmp/operator-package-dynamic-smoke.json"));
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--out" => {
                let Some(path) = iter.next() else {
                    return Err("missing value for --out".to_string());
                };
                output_path = PathBuf::from(path);
            }
            "--help" | "-h" => {
                println!("Usage: ./scripts/kyuubiki operator-package-dynamic-smoke [--out path]");
                std::process::exit(0);
            }
            other => {
                return Err(format!(
                    "unknown operator-package-dynamic-smoke option: {other}"
                ));
            }
        }
    }
    Ok(Options { output_path })
}

fn write_report(
    paths: &RepoPaths,
    options: &Options,
    stages: &[StageRecord],
    preflight_report_path: &PathBuf,
    dynamic_library_path: &PathBuf,
) -> RunnerResult<()> {
    if let Some(parent) = options.output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let ok = stages.iter().all(|stage| stage.status == 0);
    let preflight_summary = read_preflight_summary(preflight_report_path);
    let payload = json!({
        "schema_version": "kyuubiki.operator-package-dynamic-smoke/v1",
        "generated_at": utc_iso_timestamp(),
        "ok": ok,
        "package_id": preflight_summary.package_id,
        "operator_ids": preflight_summary.operator_ids,
        "host_version": preflight_summary.host_version,
        "sdk_api_version": preflight_summary.sdk_api_version,
        "template_manifest": repo_relative(paths, &paths.root.join("workers/rust/templates/operator-crate-template/Cargo.toml")),
        "package_manifest": repo_relative(paths, &paths.root.join("workers/rust/templates/operator-crate-template/kyuubiki-operator.json")),
        "preflight_report": repo_relative(paths, preflight_report_path),
        "dynamic_library": repo_relative(paths, dynamic_library_path),
        "stages": stages.iter().map(|stage| {
            json!({
                "id": stage.id,
                "description": stage.description,
                "cwd": stage.cwd,
                "command": stage.command,
                "status": stage.status,
                "ok": stage.status == 0,
            })
        }).collect::<Vec<_>>(),
    });
    let content = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("failed to encode dynamic smoke report: {error}"))?;
    std::fs::write(&options.output_path, format!("{content}\n"))
        .map_err(|error| format!("failed to write {}: {error}", options.output_path.display()))?;
    println!(
        "operator package dynamic smoke report: {}",
        options.output_path.display()
    );
    Ok(())
}

fn stage_record<const N: usize>(
    id: &'static str,
    description: &'static str,
    cwd: String,
    command: [&str; N],
    status: u8,
) -> StageRecord {
    StageRecord {
        id,
        description,
        cwd,
        command: command.into_iter().map(ToString::to_string).collect(),
        status,
    }
}

#[derive(Debug, Clone)]
struct PreflightSummary {
    package_id: Option<String>,
    operator_ids: Vec<String>,
    host_version: Option<String>,
    sdk_api_version: Option<String>,
}

fn read_preflight_summary(path: &Path) -> PreflightSummary {
    let Ok(content) = std::fs::read_to_string(path) else {
        return PreflightSummary {
            package_id: None,
            operator_ids: Vec::new(),
            host_version: None,
            sdk_api_version: None,
        };
    };
    let Ok(value) = serde_json::from_str::<Value>(&content) else {
        return PreflightSummary {
            package_id: None,
            operator_ids: Vec::new(),
            host_version: None,
            sdk_api_version: None,
        };
    };
    let first_package = value
        .get("accepted_packages")
        .and_then(Value::as_array)
        .and_then(|packages| packages.first());
    PreflightSummary {
        package_id: first_package
            .and_then(|package| package.get("package_id"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        operator_ids: first_package
            .and_then(|package| package.get("operator_ids"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
        host_version: value
            .get("host_version")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        sdk_api_version: first_package
            .and_then(|package| package.get("sdk_api_version"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
    }
}

fn dynamic_library_file_name() -> String {
    match env::consts::OS {
        "macos" => "libkyuubiki_operator_template.dylib".to_string(),
        "windows" => "kyuubiki_operator_template.dll".to_string(),
        _ => "libkyuubiki_operator_template.so".to_string(),
    }
}

fn repo_relative(paths: &RepoPaths, path: &Path) -> String {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        paths.root.join(path)
    };
    absolute
        .strip_prefix(&paths.root)
        .unwrap_or(&absolute)
        .to_string_lossy()
        .to_string()
}

fn relative_to(paths: &RepoPaths, base: &Path, path: &Path) -> String {
    let absolute_base = if base.is_absolute() {
        base.to_path_buf()
    } else {
        paths.root.join(base)
    };
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        paths.root.join(path)
    };
    if let Ok(stripped) = absolute_path.strip_prefix(&absolute_base) {
        return stripped.to_string_lossy().to_string();
    }
    let base_parts: Vec<_> = repo_relative(paths, &absolute_base)
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .map(ToString::to_string)
        .collect();
    let path_parts: Vec<_> = repo_relative(paths, &absolute_path)
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .map(ToString::to_string)
        .collect();
    let common = base_parts
        .iter()
        .zip(path_parts.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative_parts = vec!["..".to_string(); base_parts.len().saturating_sub(common)];
    relative_parts.extend(path_parts.into_iter().skip(common));
    if relative_parts.is_empty() {
        ".".to_string()
    } else {
        relative_parts.join("/")
    }
}
