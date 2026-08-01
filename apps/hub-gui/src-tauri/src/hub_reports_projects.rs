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
