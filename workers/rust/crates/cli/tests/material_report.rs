use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_temp_json(prefix: &str, payload: &Value) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("kyuubiki-material-report-test-{unique}"));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{prefix}.json"));
    fs::write(
        &path,
        serde_json::to_vec_pretty(payload).expect("serialize input"),
    )
    .expect("write input");
    path
}

#[test]
fn material_report_cli_builds_ranked_heat_spreader_report() {
    let input = write_temp_json(
        "results",
        &json!({
            "results": [
                { "result": { "max_temperature": 82.0, "max_heat_flux": 900.0 } },
                { "result": { "result": { "max_temperature": 64.0, "max_heat_flux": 1400.0 } } },
                { "max_temperature": 58.0, "max_heat_flux": 1800.0 }
            ]
        }),
    );
    let output_path = input.with_file_name("report.json");
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args([
            "heat-spreader",
            "--results",
            input.to_str().expect("input path"),
            "--out",
            output_path.to_str().expect("output path"),
            "--json",
        ])
        .output()
        .expect("run material report");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("stdout report json");
    assert_eq!(
        report["schema_version"].as_str(),
        Some("kyuubiki.material-research-report/v1")
    );
    assert_eq!(
        report["winner_candidate_id"].as_str(),
        Some("pyrolytic_graphite_in_plane")
    );
    assert_eq!(
        report["optimization"]["id"].as_str(),
        Some("material.heat_spreader_screening.optimization.v1")
    );
    assert_eq!(report["candidates"].as_array().map(Vec::len), Some(3));
    assert_eq!(
        report["candidates"][0]["optimization_terms"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );

    let file_report: Value =
        serde_json::from_slice(&fs::read(output_path).expect("read output report"))
            .expect("file report json");
    assert_eq!(file_report, report);
}

#[test]
fn material_report_cli_text_summary_shows_reliability_decision() {
    let input = write_temp_json(
        "results-text",
        &json!({
            "results": [
                { "result": { "max_temperature": 82.0, "max_heat_flux": 900.0 } },
                { "result": { "result": { "max_temperature": 64.0, "max_heat_flux": 1400.0 } } },
                { "max_temperature": 58.0, "max_heat_flux": 1800.0 }
            ]
        }),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args([
            "heat-spreader",
            "--results",
            input.to_str().expect("input path"),
        ])
        .output()
        .expect("run material report");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Reliability: blocked_by_quality_gates"));
    assert!(stdout.contains("gate.areal_mass.warning"));
}

#[test]
fn material_report_cli_lists_study_catalog() {
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args(["list", "--json"])
        .output()
        .expect("run material report list");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("catalog json");
    assert_eq!(
        payload["schema_version"].as_str(),
        Some("kyuubiki.material-study-catalog/v1")
    );
    assert_eq!(payload["study_count"].as_u64(), Some(5));
    assert!(payload["studies"].as_array().is_some_and(|studies| {
        studies.iter().any(|study| {
            study["id"] == "material_dielectric_screening"
                && study["metric_specs"]
                    .as_array()
                    .is_some_and(|metrics| !metrics.is_empty())
        })
    }));
}

#[test]
fn material_report_cli_describes_study_alias() {
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args(["describe", "structural-panel", "--json"])
        .output()
        .expect("run material report describe");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("study json");
    assert_eq!(
        payload["id"].as_str(),
        Some("material_structural_panel_screening")
    );
    assert_eq!(payload["domain"].as_str(), Some("structural"));
    assert!(payload["metric_specs"].as_array().is_some_and(|metrics| {
        metrics
            .iter()
            .any(|metric| metric["id"].as_str() == Some("yield_safety_factor"))
    }));
}

#[test]
fn material_report_cli_builds_dielectric_report() {
    let input = write_temp_json(
        "dielectric-results",
        &json!([
            { "result": { "max_electric_field": 42.0e6, "max_flux_density": 1.2e-3 } },
            { "result": { "result": { "max_electric_field": 38.0e6, "max_flux_density": 3.3e-3 } } },
            { "max_electric_field": 48.0e6, "max_flux_density": 0.9e-3 }
        ]),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args([
            "dielectric-screening",
            "--results",
            input.to_str().expect("input path"),
            "--json",
        ])
        .output()
        .expect("run material report");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("stdout report json");
    assert_eq!(
        report["schema_version"].as_str(),
        Some("kyuubiki.dielectric-material-report/v1")
    );
    assert_eq!(
        report["winner_candidate_id"].as_str(),
        Some("polyimide_film")
    );
    assert_eq!(
        report["optimization"]["id"].as_str(),
        Some("material.dielectric_screening.optimization.v1")
    );
    assert_eq!(report["candidates"].as_array().map(Vec::len), Some(3));
    assert_eq!(
        report["candidates"][0]["optimization_terms"]
            .as_array()
            .map(Vec::len),
        Some(4)
    );
}

#[test]
fn material_report_cli_rejects_unknown_study() {
    let input = write_temp_json("results", &json!([]));
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args([
            "unknown-study",
            "--results",
            input.to_str().expect("input path"),
        ])
        .output()
        .expect("run material report");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported material report study"));
}

#[test]
fn material_report_cli_rejects_missing_flag_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args(["heat-spreader", "--results"])
        .output()
        .expect("run material report");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--results requires a value"));
}

#[test]
fn material_report_cli_rejects_build_flags_on_catalog_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args(["list", "--results", "ignored.json"])
        .output()
        .expect("run material report list");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--results is only valid when building a material report")
    );
}

#[test]
fn material_report_cli_builds_thermo_shield_report() {
    let input = write_temp_json(
        "thermo-results",
        &json!([
            { "result": { "max_stress": 180.0e6, "max_displacement": 0.00032, "max_temperature_delta": 110.0 } },
            { "result": { "max_stress": 90.0e6, "max_displacement": 0.00022, "max_temperature_delta": 110.0 } },
            { "max_stress": 35.0e6, "max_displacement": 0.00018, "max_temperature_delta": 110.0 }
        ]),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args([
            "thermo-shield",
            "--results",
            input.to_str().expect("input path"),
            "--json",
        ])
        .output()
        .expect("run material report");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("stdout report json");
    assert_eq!(
        report["schema_version"].as_str(),
        Some("kyuubiki.thermo-material-report/v1")
    );
    assert_eq!(report["winner_candidate_id"].as_str(), Some("invar_36"));
    assert_eq!(
        report["optimization"]["id"].as_str(),
        Some("material.thermo_shield_screening.optimization.v1")
    );
    assert_eq!(report["candidates"].as_array().map(Vec::len), Some(3));
    assert_eq!(
        report["candidates"][0]["optimization_terms"]
            .as_array()
            .map(Vec::len),
        Some(4)
    );
}

#[test]
fn material_report_cli_accepts_custom_optimization_profile() {
    let input = write_temp_json(
        "thermo-results",
        &json!([
            { "result": { "max_stress": 180.0e6, "max_displacement": 0.00032, "max_temperature_delta": 110.0 } },
            { "result": { "max_stress": 90.0e6, "max_displacement": 0.00022, "max_temperature_delta": 110.0 } },
            { "max_stress": 35.0e6, "max_displacement": 0.00018, "max_temperature_delta": 110.0 }
        ]),
    );
    let profile = write_temp_json(
        "profile",
        &json!({
            "id": "custom.mass-first.v1",
            "goal": "Prefer the lightest candidate for early envelope exploration.",
            "score_range": "0.0..1.0 higher_is_better",
            "score_formula": "1.00*areal_mass_kg_m2:min",
            "weights": [
                { "metric_id": "areal_mass_kg_m2", "direction": "minimize", "weight": 1.0 },
                { "metric_id": "max_stress_pa", "direction": "minimize", "weight": 0.0 },
                { "metric_id": "max_displacement_m", "direction": "minimize", "weight": 0.0 },
                { "metric_id": "thermal_expansion_1_k", "direction": "minimize", "weight": 0.0 }
            ],
            "constraints": []
        }),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args([
            "thermo-shield",
            "--results",
            input.to_str().expect("input path"),
            "--profile",
            profile.to_str().expect("profile path"),
            "--json",
        ])
        .output()
        .expect("run material report");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("stdout report json");
    assert_eq!(
        report["optimization"]["id"].as_str(),
        Some("custom.mass-first.v1")
    );
    assert_eq!(
        report["winner_candidate_id"].as_str(),
        Some("aluminum_6061_t6")
    );
}

#[test]
fn material_report_cli_builds_structural_panel_report() {
    let input = write_temp_json(
        "structural-results",
        &json!([
            { "result": { "max_stress": 210.0e6, "max_displacement": 0.0009 } },
            { "result": { "max_stress": 160.0e6, "max_displacement": 0.00042 } },
            { "max_stress": 120.0e6, "max_displacement": 0.00055 }
        ]),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args([
            "structural-panel",
            "--results",
            input.to_str().expect("input path"),
            "--json",
        ])
        .output()
        .expect("run material report");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("stdout report json");
    assert_eq!(
        report["schema_version"].as_str(),
        Some("kyuubiki.structural-material-report/v1")
    );
    assert_eq!(
        report["winner_candidate_id"].as_str(),
        Some("carbon_fiber_quasi_iso")
    );
    assert_eq!(
        report["optimization"]["id"].as_str(),
        Some("material.structural_panel_screening.optimization.v1")
    );
    assert_eq!(report["candidates"].as_array().map(Vec::len), Some(3));
    assert_eq!(
        report["candidates"][0]["optimization_terms"]
            .as_array()
            .map(Vec::len),
        Some(5)
    );
}

#[test]
fn material_report_cli_reads_headless_run_report_result_fetch_steps() {
    let input = write_temp_json(
        "headless-run",
        &json!({
            "schema_version": "kyuubiki.headless-execution-run/v1",
            "workflow_id": "template.material_structural_panel_screening",
            "mode": "execute:service",
            "status": "ok",
            "steps": [
                { "index": 1, "action": "solve_plane_quad_2d", "status": "executed", "result_preview": { "job_id": "job-a" } },
                { "index": 2, "action": "job_wait", "status": "executed", "result_preview": { "status": "completed" } },
                { "index": 3, "action": "result_fetch", "status": "executed", "result_preview": { "result": { "max_stress": 210.0e6, "max_displacement": 0.0009 } } },
                { "index": 4, "action": "solve_plane_quad_2d", "status": "executed", "result_preview": { "job_id": "job-b" } },
                { "index": 5, "action": "job_wait", "status": "executed", "result_preview": { "status": "completed" } },
                { "index": 6, "action": "result_fetch", "status": "executed", "result_preview": { "result": { "max_stress": 160.0e6, "max_displacement": 0.00042 } } },
                { "index": 7, "action": "solve_plane_quad_2d", "status": "executed", "result_preview": { "job_id": "job-c" } },
                { "index": 8, "action": "job_wait", "status": "executed", "result_preview": { "status": "completed" } },
                { "index": 9, "action": "result_fetch", "status": "executed", "result_preview": { "result": { "max_stress": 120.0e6, "max_displacement": 0.00055 } } }
            ]
        }),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args([
            "structural-panel",
            "--results",
            input.to_str().expect("input path"),
            "--json",
        ])
        .output()
        .expect("run material report");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("stdout report json");
    assert_eq!(
        report["winner_candidate_id"].as_str(),
        Some("carbon_fiber_quasi_iso")
    );
    assert_eq!(report["warnings"].as_array().map(Vec::len), Some(0));
}
