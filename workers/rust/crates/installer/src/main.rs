use std::env;
use std::path::PathBuf;

use kyuubiki_installer::{
    cross_platform_audit_report, exit_on_err, export_launch_config, init_env,
    installation_integrity_report, parse_platform, prepare_layout, print_help, repair_installation,
    run_doctor, stage_release, unified_update_plan, unified_update_preview, validate_env_file,
};

fn main() {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());

    match command.as_str() {
        "help" | "--help" | "-h" => print_help(),
        "doctor" => run_doctor(),
        "installation-integrity" => println!("{}", installation_integrity_report().render()),
        "cross-platform-audit" => println!("{}", cross_platform_audit_report().render()),
        "update-plan" => {
            let channel = args.next();
            exit_on_err(unified_update_plan(channel).map(|report| report.render()))
        }
        "update-preview" => {
            let channel = args.next();
            exit_on_err(unified_update_preview(channel).map(|report| report.render()))
        }
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
