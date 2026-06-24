fn hub_docs_file(relative: &str) -> PathBuf {
    workspace_root()
        .join("apps")
        .join("hub-gui")
        .join("ui")
        .join("docs")
        .join(relative)
}

fn direct_mesh_baseline_file() -> PathBuf {
    workspace_root()
        .join("tests")
        .join("integration")
        .join("benchmarks")
        .join("direct-mesh-docker-baseline.json")
}

fn direct_mesh_output_root() -> PathBuf {
    workspace_root()
        .join("tmp")
        .join("direct-mesh-benchmark-container")
        .join("latest")
}

fn json_number_field(value: &serde_json::Value, path: &[&str]) -> Result<f64, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing json field {}", path.join(".")))?;
    }
    current
        .as_f64()
        .ok_or_else(|| format!("json field {} is not a number", path.join(".")))
}

fn json_u64_field(value: &serde_json::Value, path: &[&str]) -> Result<u64, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing json field {}", path.join(".")))?;
    }
    current
        .as_u64()
        .ok_or_else(|| format!("json field {} is not an unsigned integer", path.join(".")))
}

fn json_string_field(value: &serde_json::Value, path: &[&str]) -> Result<String, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing json field {}", path.join(".")))?;
    }
    current
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("json field {} is not a string", path.join(".")))
}

fn read_json_file(path: &Path) -> Result<serde_json::Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn direct_mesh_regression_snapshot() -> Result<DirectMeshRegressionSnapshotPayload, String> {
    let baseline_path = direct_mesh_baseline_file();
    let output_root = direct_mesh_output_root();
    let latest_summary_path = output_root.join("summary.json");

    let baseline = read_json_file(&baseline_path)?;
    let baseline_mean_elapsed_s =
        json_number_field(&baseline, &["aggregate", "elapsed_s", "mean"])?;
    let baseline_mean_rss_kib =
        json_number_field(&baseline, &["aggregate", "max_rss_kib", "mean"])?;
    let repeat = json_u64_field(&baseline, &["source", "repeat"])?;
    let docker_run_network = json_string_field(&baseline, &["source", "docker_run_network"])?;

    if !latest_summary_path.is_file() {
        return Ok(DirectMeshRegressionSnapshotPayload {
            baseline_path: baseline_path.display().to_string(),
            output_root: output_root.display().to_string(),
            baseline_mean_elapsed_s,
            baseline_mean_rss_kib,
            repeat,
            docker_run_network,
            latest_exists: false,
            latest_generated_at: None,
            latest_mean_elapsed_s: None,
            latest_mean_rss_kib: None,
            elapsed_delta_pct: None,
            rss_delta_pct: None,
            status: "baseline_only".to_string(),
        });
    }

    let latest = read_json_file(&latest_summary_path)?;
    let latest_mean_elapsed_s = json_number_field(&latest, &["aggregate", "elapsed_s", "mean"])?;
    let latest_mean_rss_kib = json_number_field(&latest, &["aggregate", "max_rss_kib", "mean"])?;
    let latest_generated_at = latest
        .get("generated_at")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let elapsed_delta_pct =
        ((latest_mean_elapsed_s - baseline_mean_elapsed_s) / baseline_mean_elapsed_s) * 100.0;
    let rss_delta_pct =
        ((latest_mean_rss_kib - baseline_mean_rss_kib) / baseline_mean_rss_kib) * 100.0;
    let status = if elapsed_delta_pct > 5.0 || rss_delta_pct > 10.0 {
        "regressed"
    } else {
        "within_baseline"
    };

    Ok(DirectMeshRegressionSnapshotPayload {
        baseline_path: baseline_path.display().to_string(),
        output_root: output_root.display().to_string(),
        baseline_mean_elapsed_s,
        baseline_mean_rss_kib,
        repeat,
        docker_run_network,
        latest_exists: true,
        latest_generated_at,
        latest_mean_elapsed_s: Some(latest_mean_elapsed_s),
        latest_mean_rss_kib: Some(latest_mean_rss_kib),
        elapsed_delta_pct: Some(elapsed_delta_pct),
        rss_delta_pct: Some(rss_delta_pct),
        status: status.to_string(),
    })
}

fn node_command() -> &'static str {
    if cfg!(target_os = "windows") {
        "node.exe"
    } else {
        "node"
    }
}

fn nonempty_trimmed_path<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(trimmed)
    }
}

fn normalize_existing_path(value: &str, label: &str) -> Result<PathBuf, String> {
    let trimmed = nonempty_trimmed_path(value, label)?;
    let candidate = PathBuf::from(trimmed);
    if !candidate.exists() {
        return Err(format!("{label} does not exist: {}", candidate.display()));
    }
    candidate
        .canonicalize()
        .map_err(|error| format!("failed to resolve {label} {}: {error}", candidate.display()))
}

fn path_has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case(extension))
        .unwrap_or(false)
}

fn normalize_existing_bundle_path(value: &str, label: &str) -> Result<PathBuf, String> {
    let path = normalize_existing_path(value, label)?;
    if !path.is_file() {
        return Err(format!("{label} must point to a project bundle file"));
    }
    if !path_has_extension(&path, "kyuubiki") {
        return Err(format!("{label} must end with .kyuubiki"));
    }
    Ok(path)
}

fn normalize_existing_directory_path(value: &str, label: &str) -> Result<PathBuf, String> {
    let path = normalize_existing_path(value, label)?;
    if !path.is_dir() {
        return Err(format!("{label} must point to a directory"));
    }
    Ok(path)
}

fn normalize_output_path(value: &str, label: &str) -> Result<PathBuf, String> {
    let trimmed = nonempty_trimmed_path(value, label)?;
    let candidate = PathBuf::from(trimmed);
    let parent = candidate
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workspace_root());
    if !parent.exists() {
        return Err(format!(
            "{label} parent directory does not exist: {}",
            parent.display()
        ));
    }
    parent
        .canonicalize()
        .map_err(|error| format!("failed to resolve {label} parent {}: {error}", parent.display()))?;
    Ok(candidate)
}

fn ensure_distinct_paths(left: &Path, right: &Path, message: &str) -> Result<(), String> {
    if left == right {
        Err(message.to_string())
    } else {
        Ok(())
    }
}

fn run_project_cli(command: &str, input_path: &str) -> Result<String, String> {
    let normalized_input = normalize_existing_bundle_path(input_path, "project bundle path")?;

    let root = workspace_root();
    let script = root.join("apps").join("frontend").join("scripts").join("kyuubiki-cli.mjs");
    let output = Command::new(node_command())
        .current_dir(&root)
        .arg(script)
        .arg("project")
        .arg(command)
        .arg(&normalized_input)
        .arg("--json")
        .output()
        .map_err(|error| format!("failed to run project {}: {error}", command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

fn run_project_cli_with_output(command: &str, input_path: &str, output_path: &str) -> Result<String, String> {
    let normalized_input = match command {
        "pack" => normalize_existing_directory_path(input_path, "project directory path")?,
        _ => normalize_existing_bundle_path(input_path, "project bundle path")?,
    };
    let normalized_output = normalize_output_path(output_path, "output path")?;

    match command {
        "normalize" => {
            if !path_has_extension(&normalized_output, "kyuubiki") {
                return Err("output path for project normalize must end with .kyuubiki".to_string());
            }
            ensure_distinct_paths(
                &normalized_input,
                &normalized_output,
                "output path must be different from the input bundle path",
            )?;
        }
        "unpack" => {
            if path_has_extension(&normalized_output, "kyuubiki") {
                return Err("output path for project unpack must be a directory path, not a .kyuubiki bundle".to_string());
            }
        }
        "pack" => {
            if !path_has_extension(&normalized_output, "kyuubiki") {
                return Err("output path for project pack must end with .kyuubiki".to_string());
            }
        }
        _ => {}
    }

    let root = workspace_root();
    let script = root.join("apps").join("frontend").join("scripts").join("kyuubiki-cli.mjs");
    let output = Command::new(node_command())
        .current_dir(&root)
        .arg(script)
        .arg("project")
        .arg(command)
        .arg(&normalized_input)
        .arg("--out")
        .arg(&normalized_output)
        .output()
        .map_err(|error| format!("failed to run project {}: {error}", command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

fn run_project_cli_compare(command: &str, left_path: &str, right_path: &str) -> Result<String, String> {
    let normalized_left = normalize_existing_bundle_path(left_path, "left project bundle path")?;
    let normalized_right = normalize_existing_bundle_path(right_path, "right project bundle path")?;
    ensure_distinct_paths(
        &normalized_left,
        &normalized_right,
        "left and right project bundle paths must be different",
    )?;

    let root = workspace_root();
    let script = root.join("apps").join("frontend").join("scripts").join("kyuubiki-cli.mjs");
    let output = Command::new(node_command())
        .current_dir(&root)
        .arg(script)
        .arg("project")
        .arg(command)
        .arg(&normalized_left)
        .arg(&normalized_right)
        .arg("--json")
        .output()
        .map_err(|error| format!("failed to run project {}: {error}", command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}
