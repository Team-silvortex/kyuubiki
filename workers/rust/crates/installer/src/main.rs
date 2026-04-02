use std::env;
use std::path::PathBuf;

use kyuubiki_installer::{
    exit_on_err, export_launch_config, init_env, parse_platform, print_help, prepare_layout,
    run_doctor, stage_release, validate_env_file,
};

fn main() {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());

    match command.as_str() {
        "help" | "--help" | "-h" => print_help(),
        "doctor" => run_doctor(),
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
