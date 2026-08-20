use std::env;
use std::path::PathBuf;

use kyuubiki_installer::{
    active_agent_binary, agent_update_status, credential_storage_contract,
    cross_platform_audit_report, default_remote_artifact_delivery_manifest,
    default_remote_deployment_dry_run, default_remote_deployment_journal,
    default_remote_deployment_plan, default_remote_host_trust_plan,
    default_remote_ssh_fixture_plan, default_remote_ssh_fixture_report, embedded_runtime_report,
    exit_on_err, export_launch_config, init_env, install_agent_update_package,
    install_runtime_payload, installation_integrity_report, launch_managed_agent,
    linux_desktop_dependency_plan, operator_package_preflight, parse_platform,
    prepare_agent_update_package, prepare_layout, prepare_staged_update, print_help,
    remote_deployment_roadmap, repair_installation, rollback_agent_update,
    rollback_runtime_payload, run_agent_solver_operational_qualification,
    run_agent_update_qualification, run_doctor, run_runtime_payload_qualification,
    runtime_payload_status, seal_agent_update_package, seal_runtime_payload, stage_release,
    unified_update_plan, unified_update_preview, validate_env_file,
    write_agent_solver_operational_qualification_report, write_agent_update_qualification_report,
    write_operator_package_preflight_outcome, write_runtime_payload_qualification_report,
};

fn main() {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());

    match command.as_str() {
        "help" | "--help" | "-h" => print_help(),
        "doctor" => run_doctor(),
        "installation-integrity" => println!("{}", installation_integrity_report().render()),
        "credential-storage" => println!("{}", credential_storage_contract().render()),
        "cross-platform-audit" => println!("{}", cross_platform_audit_report().render()),
        "linux-desktop-deps" => println!("{}", linux_desktop_dependency_plan().render()),
        "embedded-runtimes" => exit_on_err(embedded_runtime_report().map(|report| report.render())),
        "runtime-payload-status" => {
            exit_on_err(runtime_payload_status().map(|report| report.render()))
        }
        "agent-update-status" => exit_on_err(agent_update_status().map(|report| report.render())),
        "active-agent-binary" => {
            exit_on_err(active_agent_binary().map(|path| path.display().to_string()))
        }
        "launch-managed-agent" => match launch_managed_agent(&args.collect::<Vec<_>>()) {
            Ok(code) => std::process::exit(code),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        "install-agent-update" => {
            let Some(path) = args.next() else {
                eprintln!("missing package path for install-agent-update");
                std::process::exit(1);
            };
            exit_on_err(
                install_agent_update_package(&PathBuf::from(path)).map(|record| record.render()),
            )
        }
        "rollback-agent-update" => {
            exit_on_err(rollback_agent_update().map(|record| record.render()))
        }
        "seal-agent-update" => {
            let Some(path) = args.next() else {
                eprintln!("missing package path for seal-agent-update");
                std::process::exit(1);
            };
            let Some(version) = args.next() else {
                eprintln!("missing version for seal-agent-update");
                std::process::exit(1);
            };
            let platform = parse_platform(args.next());
            exit_on_err(
                seal_agent_update_package(&PathBuf::from(path), &version, platform)
                    .map(|manifest| format!("sealed agent update: {}", manifest.version)),
            )
        }
        "prepare-agent-update" => {
            let Some(binary) = args.next() else {
                eprintln!("missing binary path for prepare-agent-update");
                std::process::exit(1);
            };
            let Some(package) = args.next() else {
                eprintln!("missing package path for prepare-agent-update");
                std::process::exit(1);
            };
            let Some(version) = args.next() else {
                eprintln!("missing version for prepare-agent-update");
                std::process::exit(1);
            };
            let platform = parse_platform(args.next());
            exit_on_err(
                prepare_agent_update_package(
                    &PathBuf::from(binary),
                    &PathBuf::from(package),
                    &version,
                    platform,
                )
                .map(|manifest| format!("prepared agent update: {}", manifest.version)),
            )
        }
        "qualify-agent-update" => {
            let Some(first_binary) = args.next() else {
                eprintln!("missing first binary path for qualify-agent-update");
                std::process::exit(1);
            };
            let Some(second_binary) = args.next() else {
                eprintln!("missing second binary path for qualify-agent-update");
                std::process::exit(1);
            };
            let Some(work_root) = args.next() else {
                eprintln!("missing work root for qualify-agent-update");
                std::process::exit(1);
            };
            let Some(first_version) = args.next() else {
                eprintln!("missing first version for qualify-agent-update");
                std::process::exit(1);
            };
            let Some(second_version) = args.next() else {
                eprintln!("missing second version for qualify-agent-update");
                std::process::exit(1);
            };
            let output = args.next().map(PathBuf::from);
            exit_on_err(
                run_agent_update_qualification(
                    &PathBuf::from(first_binary),
                    &PathBuf::from(second_binary),
                    &PathBuf::from(work_root),
                    &first_version,
                    &second_version,
                )
                .and_then(|report| match output {
                    Some(path) => {
                        write_agent_update_qualification_report(&report, &path)?;
                        Ok(format!(
                            "agent update qualification passed: {}",
                            path.display()
                        ))
                    }
                    None => {
                        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
                    }
                }),
            )
        }
        "qualify-agent-solver-operational" => {
            let Some(agent_binary) = args.next() else {
                eprintln!("missing agent binary path for qualify-agent-solver-operational");
                std::process::exit(1);
            };
            let Some(work_root) = args.next() else {
                eprintln!("missing work root for qualify-agent-solver-operational");
                std::process::exit(1);
            };
            let Some(package_version) = args.next() else {
                eprintln!("missing package version for qualify-agent-solver-operational");
                std::process::exit(1);
            };
            let output = args.next().map(PathBuf::from);
            exit_on_err(
                run_agent_solver_operational_qualification(
                    &PathBuf::from(agent_binary),
                    &PathBuf::from(work_root),
                    &package_version,
                )
                .and_then(|report| match output {
                    Some(path) => {
                        write_agent_solver_operational_qualification_report(&report, &path)?;
                        Ok(format!(
                            "agent solver operational qualification passed: {}",
                            path.display()
                        ))
                    }
                    None => {
                        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
                    }
                }),
            )
        }
        "qualify-runtime-payload" => {
            let Some(first_binary) = args.next() else {
                eprintln!("missing first binary path for qualify-runtime-payload");
                std::process::exit(1);
            };
            let Some(second_binary) = args.next() else {
                eprintln!("missing second binary path for qualify-runtime-payload");
                std::process::exit(1);
            };
            let Some(work_root) = args.next() else {
                eprintln!("missing work root for qualify-runtime-payload");
                std::process::exit(1);
            };
            let Some(first_version) = args.next() else {
                eprintln!("missing first version for qualify-runtime-payload");
                std::process::exit(1);
            };
            let Some(second_version) = args.next() else {
                eprintln!("missing second version for qualify-runtime-payload");
                std::process::exit(1);
            };
            let output = args.next().map(PathBuf::from);
            exit_on_err(
                run_runtime_payload_qualification(
                    &PathBuf::from(first_binary),
                    &PathBuf::from(second_binary),
                    &PathBuf::from(work_root),
                    &first_version,
                    &second_version,
                )
                .and_then(|report| match output {
                    Some(path) => {
                        write_runtime_payload_qualification_report(&report, &path)?;
                        Ok(format!(
                            "runtime payload qualification passed: {}",
                            path.display()
                        ))
                    }
                    None => {
                        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
                    }
                }),
            )
        }
        "install-runtime-payload" => {
            let Some(path) = args.next() else {
                eprintln!("missing payload path for install-runtime-payload");
                std::process::exit(1);
            };
            exit_on_err(install_runtime_payload(&PathBuf::from(path)).map(|record| record.render()))
        }
        "rollback-runtime-payload" => {
            exit_on_err(rollback_runtime_payload().map(|record| record.render()))
        }
        "seal-runtime-payload" => {
            let Some(path) = args.next() else {
                eprintln!("missing payload path for seal-runtime-payload");
                std::process::exit(1);
            };
            let Some(version) = args.next() else {
                eprintln!("missing version for seal-runtime-payload");
                std::process::exit(1);
            };
            let platform = parse_platform(args.next());
            exit_on_err(
                seal_runtime_payload(&PathBuf::from(path), &version, platform)
                    .map(|manifest| format!("sealed runtime payload: {manifest}")),
            )
        }
        "operator-package-preflight" => {
            let Some(packages_root) = args.next() else {
                eprintln!("missing packages root for operator-package-preflight");
                print_help();
                std::process::exit(1);
            };
            let packages_root = PathBuf::from(packages_root);
            match parse_operator_package_preflight_flags(args.collect()) {
                Ok(flags) => exit_on_err(run_operator_package_preflight_command(
                    &packages_root,
                    flags,
                )),
                Err(error) => {
                    eprintln!("{error}");
                    print_help();
                    std::process::exit(1);
                }
            }
        }
        "update-plan" => {
            let channel = args.next();
            exit_on_err(unified_update_plan(channel).map(|report| report.render()))
        }
        "update-preview" => {
            let channel = args.next();
            exit_on_err(unified_update_preview(channel).map(|report| report.render()))
        }
        "prepare-staged-update" => {
            let channel = args.next();
            let platform = parse_platform(args.next());
            let target_dir = args.next().map(PathBuf::from);
            exit_on_err(
                prepare_staged_update(channel, platform, target_dir).map(|report| report.render()),
            )
        }
        "remote-deployment-roadmap" => println!("{}", remote_deployment_roadmap().render()),
        "remote-deployment-plan" => println!("{}", default_remote_deployment_plan().render()),
        "remote-deployment-journal" => println!("{}", default_remote_deployment_journal().render()),
        "remote-artifacts" => exit_on_err(
            default_remote_artifact_delivery_manifest().map(|manifest| manifest.render()),
        ),
        "remote-deployment-dry-run" => {
            println!("{}", default_remote_deployment_dry_run().render())
        }
        "remote-host-trust" => println!("{}", default_remote_host_trust_plan().render()),
        "remote-ssh-fixture" => println!("{}", default_remote_ssh_fixture_report().render()),
        "remote-ssh-fixture-plan" => println!("{}", default_remote_ssh_fixture_plan().render()),
        "repair-installation" => exit_on_err(repair_installation()),
        "validate-env" => exit_on_err(validate_env_file()),
        "init-env" => {
            let force = args.any(|arg| arg == "--force");
            exit_on_err(init_env(force));
        }
        "prepare-layout" => exit_on_err(prepare_layout()),
        "export-launch" => {
            let platform = parse_platform(args.next());
            print!("{}", export_launch_config(platform));
        }
        "stage-release" => {
            let platform = parse_platform(args.next());
            let target_dir = args.next().map(PathBuf::from);
            exit_on_err(stage_release(platform, target_dir));
        }
        "bootstrap" => {
            run_doctor();
            exit_on_err(prepare_layout());
            exit_on_err(init_env(false));
            exit_on_err(validate_env_file());
        }
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct OperatorPackagePreflightFlags {
    output_path: Option<PathBuf>,
    fail_on_rejected: bool,
    fail_on_readiness_warnings: bool,
}

fn parse_operator_package_preflight_flags(
    args: Vec<String>,
) -> Result<OperatorPackagePreflightFlags, String> {
    let mut flags = OperatorPackagePreflightFlags::default();
    let mut output_path = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("missing value for --out".to_string());
                };
                output_path = Some(PathBuf::from(path));
                index += 2;
            }
            "--fail-on-rejected" => {
                flags.fail_on_rejected = true;
                index += 1;
            }
            "--fail-on-readiness-warnings" => {
                flags.fail_on_readiness_warnings = true;
                index += 1;
            }
            other => return Err(format!("unknown operator-package-preflight flag: {other}")),
        }
    }
    flags.output_path = output_path;
    Ok(flags)
}

fn run_operator_package_preflight_command(
    packages_root: &std::path::Path,
    flags: OperatorPackagePreflightFlags,
) -> Result<String, String> {
    if let Some(output_path) = flags.output_path {
        let outcome = operator_package_preflight(packages_root)?;
        write_operator_package_preflight_outcome(&outcome, &output_path)?;
        outcome.ensure_no_rejections().or_else(|error| {
            if flags.fail_on_rejected {
                Err(error)
            } else {
                Ok(())
            }
        })?;
        outcome.ensure_no_readiness_warnings().or_else(|error| {
            if flags.fail_on_readiness_warnings {
                Err(error)
            } else {
                Ok(())
            }
        })?;
        return Ok(format!("wrote {}", output_path.display()));
    }

    let outcome = operator_package_preflight(packages_root)?;
    let json = outcome.json.clone();
    outcome.ensure_no_rejections().or_else(|error| {
        if flags.fail_on_rejected {
            Err(error)
        } else {
            Ok(())
        }
    })?;
    outcome.ensure_no_readiness_warnings().or_else(|error| {
        if flags.fail_on_readiness_warnings {
            Err(error)
        } else {
            Ok(())
        }
    })?;
    Ok(json)
}
