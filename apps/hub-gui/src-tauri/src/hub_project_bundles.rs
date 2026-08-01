fn create_project_bundle(path: &str) -> Result<String, String> {
    kyuubiki_project_bundle::create_project_bundle(path)
}

fn run_project_cli(command: &str, input_path: &str) -> Result<String, String> {
    match command {
        "inspect" => kyuubiki_project_bundle::inspect_project_bundle(input_path),
        "validate" => kyuubiki_project_bundle::validate_project_bundle(input_path),
        _ => Err(format!("unsupported native project action: {command}")),
    }
}

fn run_project_cli_with_output(
    command: &str,
    input_path: &str,
    output_path: &str,
) -> Result<String, String> {
    match command {
        "normalize" => {
            kyuubiki_project_bundle::normalize_project_bundle(input_path, output_path)
        }
        "pack" => kyuubiki_project_bundle::pack_project_bundle(input_path, output_path),
        "unpack" => kyuubiki_project_bundle::unpack_project_bundle(input_path, output_path),
        _ => Err(format!("unsupported native project action: {command}")),
    }
}

fn run_project_cli_compare(
    command: &str,
    left_path: &str,
    right_path: &str,
) -> Result<String, String> {
    match command {
        "diff" => kyuubiki_project_bundle::diff_project_bundles(left_path, right_path),
        _ => Err(format!("unsupported native project action: {command}")),
    }
}
