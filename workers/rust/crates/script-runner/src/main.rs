use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

mod agent_registry_sync;
mod beam_frame_release_evidence;
mod benchmark_profile_index;
mod benchmark_profile_plan;
mod benchmark_profile_remote;
mod benchmark_profile_remote_summary;
mod central_database_readiness;
mod central_database_smoke;
mod central_readiness_report;
mod central_store_contract;
mod commercial_readiness;
mod component_integrity_protocol;
mod contracts_runtime_api_surface;
mod dependency_audit;
mod desktop;
mod desktop_icon_variants;
mod desktop_linux_remote;
mod desktop_release_upload_remote;
mod desktop_shared_sync;
mod direct_mesh_benchmark_compare;
mod direct_mesh_container;
mod direct_mesh_remote;
mod docs_book;
mod elixir_self_host;
mod frontend_checks;
mod governance_commands;
mod gui_runtime_capability_contract;
mod help;
mod install_update_disk_hygiene;
mod installation_integrity_docs;
mod lab;
mod language_packs;
mod line_field_baseline;
mod line_field_provenance;
mod line_field_release_evidence;
mod local_path_audit;
mod make_modules;
mod material_card_contract;
mod material_exploration_chain_contract;
mod material_research;
mod material_research_bundle;
mod material_research_bundle_build;
mod material_research_bundle_contract;
mod material_research_bundle_index;
mod material_research_bundle_index_contract;
mod material_research_example;
mod material_score_contract;
mod material_study_execution_plan_contract;
mod material_study_sdk_examples;
mod materialization_plan_contract;
mod minimal_industrial_closure;
mod module_extension_standard;
mod module_function_matrix;
mod module_function_tensor;
mod module_topology;
mod module_topology_report;
mod native_script_audit;
mod native_time;
mod nightly_artifact_overview;
mod node_tests;
mod operator_package_dynamic_smoke;
mod operator_qualification_evidence_kits;
mod operator_qualification_readiness;
mod operator_qualification_release_records;
mod operator_reliability;
mod operator_reliability_rules;
mod operator_reliability_schemas;
mod operator_task_ir_contract;
mod operator_validation;
mod project_organization_audit;
mod regression_gate_report;
mod regression_lane_catalog;
mod release_snapshot;
mod remote_central_database_smoke;
mod remote_host;
mod remote_material_health;
mod remote_material_research_example;
mod remote_material_stage_health;
mod remote_material_summary;
mod remote_ssh_fixture;
mod rust_line_counts;
mod standard_benchmark_index;
mod standard_benchmark_remote;
mod standard_benchmark_report;
mod toolchain_contract;
mod ui_automation_contract;
mod update_catalog_docs;
mod verification_evidence_surface;
mod version_line_audit;
mod workflow_catalog_benchmark_compare;
mod workflow_catalog_remote;
mod workflow_dataset_contract;
mod workflow_mesh;
mod workflow_mesh_index;
mod workflow_mesh_remote;
mod workflow_mesh_summary;

type RunnerResult<T> = Result<T, String>;

use desktop::{
    DesktopApp, host_platform, run_desktop_build, run_desktop_build_host, run_desktop_dev,
    run_desktop_release, run_desktop_stage, run_desktop_status, run_desktop_verify,
    run_package_desktop,
};
use help::print_help;

struct RepoPaths {
    root: PathBuf,
    frontend: PathBuf,
    rust: PathBuf,
    web: PathBuf,
    hub_gui: PathBuf,
    installer_gui: PathBuf,
    workbench_gui: PathBuf,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> RunnerResult<u8> {
    let paths = RepoPaths::discover()?;
    let mut args = env::args_os().skip(1);
    let command = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "help".to_string());
    let rest: Vec<OsString> = args.collect();

    if let Some(result) =
        material_research::run_material_research_command(&paths.root, &command, rest.clone())
    {
        return result;
    }
    if let Some(result) = governance_commands::run_governance_command(
        &paths.root,
        &paths.frontend,
        &command,
        rest.clone(),
    ) {
        return result;
    }
    if let Some(result) = node_tests::run_node_command(&paths, &command, rest.clone()) {
        return result;
    }

    match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(0)
        }
        "native-script-audit" => native_script_audit::run_native_script_audit(
            &paths.root,
            host_platform().as_str(),
            rest,
        ),
        "audit-version-line" => version_line_audit::run_audit_version_line(&paths.root, rest),
        "create-release-snapshot" => release_snapshot::run_create_release_snapshot(rest),
        "status"
        | "start"
        | "start-local"
        | "start-cloud"
        | "start-distributed"
        | "restart"
        | "restart-local"
        | "restart-cloud"
        | "restart-distributed"
        | "stop"
        | "export-db"
        | "hot-status"
        | "hot-start-local"
        | "hot-start-cloud"
        | "hot-start-distributed"
        | "hot-stop" => {
            let mut runtime_args = vec![
                paths
                    .root
                    .join("scripts/kyuubiki-runtime.mjs")
                    .into_os_string(),
                OsString::from(&command),
            ];
            runtime_args.extend(rest);
            run_command(&paths.root, "node", runtime_args)
        }
        "doctor" => run_installer(&paths, "doctor", rest),
        "validate-env" => run_installer(&paths, "validate-env", rest),
        "cross-platform-audit" => run_installer(&paths, "cross-platform-audit", rest),
        "check-elixir-self-host" => elixir_self_host::run_check_elixir_self_host(&paths.root, rest),
        "check-central-database-readiness" => {
            central_database_readiness::run_check_central_database_readiness(&paths.root, rest)
        }
        "central-database-smoke" => {
            central_database_smoke::run_central_database_smoke(&paths.root, rest)
        }
        "remote-central-database-smoke" => {
            remote_central_database_smoke::run_remote_central_database_smoke(&paths.root, rest)
        }
        "operator-package-preflight" => run_installer(&paths, "operator-package-preflight", rest),
        "operator-package-dynamic-smoke" => {
            operator_package_dynamic_smoke::run_operator_package_dynamic_smoke(&paths, rest)
        }
        "check-operator-package-dynamic-smoke" => {
            operator_package_dynamic_smoke::run_check_operator_package_dynamic_smoke(
                &paths.root,
                rest,
            )
        }
        "check-operator-package-dynamic-smoke-contract" => {
            operator_package_dynamic_smoke::run_check_operator_package_dynamic_smoke_contract(
                &paths.root,
                rest,
            )
        }
        "check-operator-reliability-rules" => {
            operator_reliability_rules::run_check_operator_reliability_rules(rest)
        }
        "check-operator-reliability-schemas" => {
            operator_reliability_schemas::run_check_operator_reliability_schemas(&paths.root, rest)
        }
        "check-operator-validation" => {
            operator_validation::run_check_operator_validation(&paths.root, rest)
        }
        "check-operator-reliability" => {
            operator_reliability::run_check_operator_reliability(&paths.root, rest)
        }
        "check-line-field-closed-form-baseline" => {
            line_field_baseline::run_check_line_field_closed_form_baseline(&paths.root, rest)
        }
        "capture-line-field-qualification-provenance" => {
            line_field_provenance::run_capture_line_field_qualification_provenance(
                &paths.root,
                rest,
            )
        }
        "capture-line-field-qualification-release-evidence" => {
            line_field_release_evidence::run_capture_line_field_qualification_release_evidence(
                &paths.root,
                rest,
            )
        }
        "check-line-field-qualification-release-evidence" => {
            line_field_release_evidence::run_check_line_field_qualification_release_evidence(
                &paths.root,
                rest,
            )
        }
        "check-beam-frame-qualification-release-evidence" => {
            beam_frame_release_evidence::run_check_beam_frame_qualification_release_evidence(
                &paths.root,
                rest,
            )
        }
        "build-operator-qualification-readiness" => {
            operator_qualification_readiness::run_build_operator_qualification_readiness(
                &paths.root,
                rest,
            )
        }
        "check-operator-qualification-readiness" => {
            operator_qualification_readiness::run_check_operator_qualification_readiness(
                &paths.root,
                rest,
            )
        }
        "check-operator-qualification-release-records" => {
            operator_qualification_release_records::run_check_operator_qualification_release_records(
                &paths.root,
                rest,
            )
        }
        "install" => run_command(&paths.rust, "cargo", cargo_run("kyuubiki-installer", rest)),
        "package" | "package-runtime" => {
            let platform = host_platform().as_str();
            run_installer(&paths, "stage-release", prepend(platform, rest))
        }
        "project" | "macro" => run_command(
            &paths.frontend,
            "node",
            [
                OsString::from("./scripts/kyuubiki-cli.mjs"),
                OsString::from(&command),
            ]
            .into_iter()
            .chain(rest),
        ),
        "build-frontend" => run_command(
            &paths.frontend,
            "npm",
            prepend("run", prepend("build", rest)),
        ),
        "build-orchestrator" => run_with_env(
            &paths.web,
            "mix",
            prepend("compile", rest),
            &[("MIX_ENV", "prod")],
        ),
        "build-agent" => run_command(
            &paths.rust,
            "cargo",
            ["build", "-p", "kyuubiki-cli", "--release"]
                .into_iter()
                .map(OsString::from)
                .chain(rest),
        ),
        "build-hub-gui" => run_desktop_build(&paths, DesktopApp::Hub, rest),
        "build-installer-gui" => run_desktop_build(&paths, DesktopApp::Installer, rest),
        "build-workbench-gui" => run_desktop_build(&paths, DesktopApp::Workbench, rest),
        "sync-desktop-shared" => desktop_shared_sync::run_sync_desktop_shared(&paths.root),
        "check-desktop-shared" => desktop_shared_sync::run_check_desktop_shared(&paths.root, rest),
        "package-desktop" => run_package_desktop(&paths, rest),
        "desktop-status" => run_desktop_status(&paths, rest),
        "desktop-stage" => run_desktop_stage(&paths, rest),
        "desktop-build-host" => run_desktop_build_host(&paths),
        "desktop-release" => run_desktop_release(&paths, rest),
        "desktop-verify" => run_desktop_verify(&paths, rest),
        "desktop-linux-remote" => desktop_linux_remote::run_desktop_linux_remote(&paths.root, rest),
        "desktop-upload-remote" | "desktop-release-upload-remote" => {
            desktop_release_upload_remote::run_desktop_release_upload_remote(&paths.root, rest)
        }
        "generate-desktop-icon-variants" => {
            desktop_icon_variants::run_generate_desktop_icon_variants(&paths.root, rest)
        }
        "agent-registry-sync" => agent_registry_sync::run_agent_registry_sync(&paths.root, rest),
        "lab" => lab::run_lab(&paths.root, rest),
        "remote-ssh-fixture" => remote_ssh_fixture::run_remote_ssh_fixture(&paths.root, rest),
        "web-test" => run_command(&paths.web, "mix", prepend("test", rest)),
        "rust-test" => {
            let status = run_command(&paths.rust, "cargo", prepend("test", rest.clone()))?;
            if status != 0 {
                return Ok(status);
            }
            rust_line_counts::run_rust_line_audit(&paths.root, Vec::new())
        }
        "rust-line-audit" => rust_line_counts::run_rust_line_audit(&paths.root, rest),
        "frontend-test" => {
            let typecheck = node_tests::run_frontend_typecheck(&paths.frontend, Vec::new())?;
            if typecheck != 0 {
                return Ok(typecheck);
            }
            run_command(&paths.frontend, "npm", ["run", "build"].map(OsString::from))
        }
        "headless-test" => {
            node_tests::run_frontend_unit_test(&paths.frontend, &["headless"], Vec::new())
        }
        "headless-live-test" => node_tests::run_frontend_unit_test(
            &paths.frontend,
            &["kyuubiki-headless-live"],
            Vec::new(),
        ),
        "headless-rust-live-test" => run_command(
            &paths.rust,
            "cargo",
            [
                "test",
                "-p",
                "kyuubiki-cli",
                "--test",
                "headless_live",
                "--",
                "--test-threads=1",
            ]
            .map(OsString::from),
        ),
        "sdk-smoke" => run_sdk_smoke(&paths),
        "workflow-preflight" => {
            let unit =
                node_tests::run_frontend_unit_test(&paths.frontend, &["workflow"], Vec::new())?;
            if unit != 0 {
                return Ok(unit);
            }
            node_tests::run_frontend_check(
                &paths.frontend,
                "./scripts/workflow-browser-preflight.mjs",
            )
        }
        "benchmark-profile-remote" => {
            benchmark_profile_remote::run_benchmark_profile_remote(&paths.root, rest)
        }
        "benchmark-profile-plan" => {
            benchmark_profile_plan::run_benchmark_profile_plan(&paths.root, rest)
        }
        "build-benchmark-profile-index" => {
            benchmark_profile_index::run_build_benchmark_profile_index(&paths.root, rest)
        }
        "build-regression-lane-catalog" => {
            regression_lane_catalog::run_build_regression_lane_catalog(&paths.root, rest)
        }
        "build-regression-gate-report" => {
            regression_gate_report::run_build_regression_gate_report(&paths.root, rest)
        }
        "build-nightly-artifact-overview" => {
            nightly_artifact_overview::run_build_nightly_artifact_overview(&paths.root, rest)
        }
        "direct-mesh-benchmark-container" => {
            direct_mesh_container::run_direct_mesh_benchmark_container(&paths.root, rest)
        }
        "compare-direct-mesh-benchmark" => {
            direct_mesh_benchmark_compare::run_compare_direct_mesh_benchmark(&paths.root, rest)
        }
        "direct-mesh-benchmark-regression" => {
            direct_mesh_remote::run_direct_mesh_benchmark_regression(&paths.root, rest)
        }
        "standard-benchmark-regression" => {
            standard_benchmark_remote::run_standard_benchmark_regression(&paths.root, rest)
        }
        "build-standard-benchmark-index" => {
            standard_benchmark_index::run_build_standard_benchmark_index(&paths.root, rest)
        }
        "build-standard-benchmark-report" => {
            standard_benchmark_report::run_build_standard_benchmark_report(&paths.root, rest)
        }
        "workflow-catalog-benchmark-regression" => {
            workflow_catalog_remote::run_workflow_catalog_remote(&paths.root, rest)
        }
        "compare-workflow-catalog-benchmark" => {
            workflow_catalog_benchmark_compare::run_compare_workflow_catalog_benchmark(
                &paths.root,
                rest,
            )
        }
        "workflow-mesh-regression-remote" => {
            workflow_mesh_remote::run_workflow_mesh_remote(&paths.root, rest)
        }
        "build-workflow-mesh-regression-index" => {
            workflow_mesh_index::run_build_workflow_mesh_regression_index(&paths.root, rest)
        }
        "build-workflow-mesh-regression-summary" => {
            workflow_mesh_summary::run_build_workflow_mesh_regression_summary(&paths.root, rest)
        }
        "workflow-mesh-regression" => workflow_mesh::run_workflow_mesh_regression(&paths.root),
        "agent-capability-smoke" => {
            run_python_script(&paths.root, "agent-capability-smoke.py", rest)
        }
        "worker" => run_command(&paths.rust, "cargo", cargo_run("kyuubiki-cli", rest)),
        "benchmark" => run_command(&paths.rust, "cargo", cargo_run("kyuubiki-benchmark", rest)),
        "agent" => {
            let agent_args = if rest.is_empty() {
                vec![
                    OsString::from("agent"),
                    OsString::from("--port"),
                    OsString::from("5001"),
                ]
            } else {
                prepend("agent", rest)
            };
            run_command(&paths.rust, "cargo", cargo_run("kyuubiki-cli", agent_args))
        }
        "frontend" => run_command(&paths.frontend, "npm", ["run", "dev"].map(OsString::from)),
        "format" => {
            let mix = run_command(&paths.web, "mix", ["format"].map(OsString::from))?;
            if mix != 0 {
                return Ok(mix);
            }
            run_command(&paths.rust, "cargo", ["fmt"].map(OsString::from))
        }
        "hub-gui-dev" => run_desktop_dev(&paths, DesktopApp::Hub),
        "installer-gui-dev" => run_desktop_dev(&paths, DesktopApp::Installer),
        "workbench-gui-dev" => run_desktop_dev(&paths, DesktopApp::Workbench),
        _ => {
            eprintln!("unknown native command: {command}");
            eprintln!("run `./scripts/kyuubiki help` for supported native commands");
            Ok(2)
        }
    }
}

impl RepoPaths {
    fn discover() -> RunnerResult<Self> {
        let exe =
            env::current_exe().map_err(|error| format!("failed to resolve executable: {error}"))?;
        let mut current = exe.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        loop {
            if current.join("workers/rust/Cargo.toml").is_file() && current.join("scripts").is_dir()
            {
                return Ok(Self {
                    frontend: current.join("apps/frontend"),
                    rust: current.join("workers/rust"),
                    web: current.join("apps/web"),
                    hub_gui: current.join("apps/hub-gui"),
                    installer_gui: current.join("apps/installer-gui"),
                    workbench_gui: current.join("apps/workbench-gui"),
                    root: current,
                });
            }
            if !current.pop() {
                break;
            }
        }
        let cwd = env::current_dir().map_err(|error| format!("failed to resolve cwd: {error}"))?;
        Ok(Self {
            frontend: cwd.join("apps/frontend"),
            rust: cwd.join("workers/rust"),
            web: cwd.join("apps/web"),
            hub_gui: cwd.join("apps/hub-gui"),
            installer_gui: cwd.join("apps/installer-gui"),
            workbench_gui: cwd.join("apps/workbench-gui"),
            root: cwd,
        })
    }
}

fn run_installer(paths: &RepoPaths, subcommand: &str, rest: Vec<OsString>) -> RunnerResult<u8> {
    run_command(
        &paths.rust,
        "cargo",
        cargo_run("kyuubiki-installer", prepend(subcommand, rest)),
    )
}

fn run_python_script(root: &Path, script: &str, rest: Vec<OsString>) -> RunnerResult<u8> {
    run_with_env(
        root,
        "python3",
        [root.join("scripts").join(script).into_os_string()]
            .into_iter()
            .chain(rest),
        &[("PYTHONPATH", "sdks/python")],
    )
}

fn run_sdk_smoke(paths: &RepoPaths) -> RunnerResult<u8> {
    let python = run_with_env(
        &paths.root,
        "python3",
        ["-m", "unittest", "discover", "-s", "sdks/python/tests"].map(OsString::from),
        &[("PYTHONPATH", "sdks/python")],
    )?;
    if python != 0 {
        return Ok(python);
    }

    let elixir = run_command(
        &paths.root.join("sdks/elixir"),
        "mix",
        ["test"].map(OsString::from),
    )?;
    if elixir != 0 {
        return Ok(elixir);
    }

    run_command(
        &paths.root,
        "cargo",
        ["test", "--manifest-path", "sdks/rust/Cargo.toml"].map(OsString::from),
    )
}

fn run_command<I>(cwd: &Path, program: &str, args: I) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    run_with_env(cwd, program, args, &[])
}

fn run_with_env<I>(cwd: &Path, program: &str, args: I, envs: &[(&str, &str)]) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .envs(envs.iter().copied())
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    Ok(status.code().unwrap_or(1) as u8)
}

fn cargo_run(package: &str, rest: Vec<OsString>) -> impl Iterator<Item = OsString> {
    ["run", "-p", package, "--"]
        .into_iter()
        .map(OsString::from)
        .chain(rest)
}

fn prepend(value: &str, rest: Vec<OsString>) -> Vec<OsString> {
    std::iter::once(OsString::from(value)).chain(rest).collect()
}
