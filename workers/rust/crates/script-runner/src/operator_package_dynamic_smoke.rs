use crate::{RepoPaths, RunnerResult, native_time::utc_iso_timestamp, run_command};
use serde_json::json;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Options {
    output_path: PathBuf,
}

#[derive(Debug, Clone)]
struct StageRecord {
    id: &'static str,
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
    stages.push(StageRecord {
        id: "template_tests",
        status: template_tests,
    });
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
    stages.push(StageRecord {
        id: "strict_preflight",
        status: preflight,
    });
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
    stages.push(StageRecord {
        id: "template_cdylib_build",
        status: build,
    });
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
    stages.push(StageRecord {
        id: "engine_dynamic_host_load",
        status: dynamic_host,
    });
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
    let payload = json!({
        "schema_version": "kyuubiki.operator-package-dynamic-smoke/v1",
        "generated_at": utc_iso_timestamp(),
        "ok": ok,
        "template_manifest": paths.root.join("workers/rust/templates/operator-crate-template/Cargo.toml"),
        "package_manifest": paths.root.join("workers/rust/templates/operator-crate-template/kyuubiki-operator.json"),
        "preflight_report": preflight_report_path,
        "dynamic_library": dynamic_library_path,
        "stages": stages.iter().map(|stage| {
            json!({
                "id": stage.id,
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

fn dynamic_library_file_name() -> String {
    match env::consts::OS {
        "macos" => "libkyuubiki_operator_template.dylib".to_string(),
        "windows" => "kyuubiki_operator_template.dll".to_string(),
        _ => "libkyuubiki_operator_template.so".to_string(),
    }
}
