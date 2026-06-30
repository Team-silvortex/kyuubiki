use std::env;
use std::path::PathBuf;

use kyuubiki_installer::{
    credential_storage_contract, cross_platform_audit_report,
    default_remote_artifact_delivery_manifest, default_remote_deployment_dry_run,
    default_remote_deployment_journal, default_remote_deployment_plan,
    default_remote_host_trust_plan, default_remote_ssh_fixture_plan,
    default_remote_ssh_fixture_report, embedded_runtime_report, exit_on_err, export_launch_config,
    init_env, installation_integrity_report, linux_desktop_dependency_plan, parse_platform,
    prepare_layout, prepare_staged_update, print_help, remote_deployment_roadmap,
    repair_installation, run_doctor, stage_release, unified_update_plan, unified_update_preview,
    validate_env_file,
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
