#[tauri::command]
fn service_status() -> Result<ServiceStatusPayload, String> {
    let rendered = desktop_service_status()?;
    Ok(ServiceStatusPayload {
        summary: desktop_summarize_service_status(&rendered),
        rendered,
    })
}

#[tauri::command]
fn get_global_language_preference() -> DesktopPreferencesPayload {
    DesktopPreferencesPayload {
        language: desktop_read_global_language_preference().unwrap_or_else(|| "en".to_string()),
    }
}

#[tauri::command]
fn set_global_language_preference(payload: DesktopPreferencesInputPayload) -> Result<DesktopPreferencesPayload, String> {
    Ok(DesktopPreferencesPayload {
        language: desktop_write_global_language_preference(&payload.language)?,
    })
}

#[tauri::command]
fn hot_service_status() -> Result<TextReportPayload, String> {
    Ok(TextReportPayload {
        rendered: desktop_hot_service_status()?,
    })
}

#[tauri::command]
fn read_runtime_log(payload: LogPayload) -> Result<RuntimeLogPayload, String> {
    Ok(RuntimeLogPayload {
        service: payload.service.clone(),
        rendered: read_shared_runtime_log(&payload.service, 180)?,
    })
}

#[tauri::command]
fn doctor_report() -> Result<TextReportPayload, String> {
    Ok(TextReportPayload {
        rendered: build_doctor_report().render(),
    })
}

#[tauri::command]
fn guarded_mutation_action(payload: GuardedMutationPayload) -> Result<String, String> {
    let result = match payload.action.as_str() {
        "service_start" => desktop_service_start(resolve_service_mode(payload.mode.as_deref())),
        "service_restart" => desktop_service_restart(resolve_service_mode(payload.mode.as_deref())),
        "service_stop" => desktop_service_stop(),
        "hot_service_start" => desktop_hot_service_start(resolve_hot_service_mode(payload.mode.as_deref())),
        "hot_service_stop" => desktop_hot_service_stop(),
        "validate_env" => validate_env_file(),
        "desktop_stage" => stage_release(parse_platform(payload.platform.clone()), None),
        "desktop_verify" => verify_desktop_platform(parse_platform(payload.platform.clone())),
        "desktop_build_host" => build_host_desktop_bundles(),
        "project_bundle_normalize" => run_project_cli_with_output(
            "normalize",
            payload.path.as_deref().unwrap_or(""),
            payload.out.as_deref().unwrap_or(""),
        ),
        "project_bundle_unpack" => run_project_cli_with_output(
            "unpack",
            payload.path.as_deref().unwrap_or(""),
            payload.out.as_deref().unwrap_or(""),
        ),
        "project_bundle_pack" => run_project_cli_with_output(
            "pack",
            payload.path.as_deref().unwrap_or(""),
            payload.out.as_deref().unwrap_or(""),
        ),
        _ => Err(format!("unsupported guarded mutation action: {}", payload.action)),
    };

    match &result {
        Ok(message) => {
            let _ = append_guarded_mutation_audit(&payload, "ok", message);
        }
        Err(error) => {
            let _ = append_guarded_mutation_audit(&payload, "failed", error);
        }
    }

    result
}

#[tauri::command]
fn desktop_status(payload: PlatformPayload) -> Result<String, String> {
    Ok(desktop_status_text(payload.platform))
}

#[tauri::command]
fn project_bundle_inspect(payload: ProjectBundlePayload) -> Result<String, String> {
    run_project_cli("inspect", &payload.path)
}

#[tauri::command]
fn project_bundle_validate(payload: ProjectBundlePayload) -> Result<String, String> {
    run_project_cli("validate", &payload.path)
}

#[tauri::command]
fn project_bundle_diff(payload: ProjectBundleComparePayload) -> Result<String, String> {
    run_project_cli_compare("diff", &payload.left_path, &payload.right_path)
}

#[tauri::command]
fn launch_workbench_gui() -> Result<String, String> {
    launch_desktop_app_with_fallback(
        "workbench-gui",
        "workbench-gui",
        "Kyuubiki Workbench",
        "kyuubiki-workbench-gui",
    )
}

#[tauri::command]
fn launch_installer_gui() -> Result<String, String> {
    launch_desktop_app_with_fallback(
        "installer-gui",
        "installer-gui",
        "Kyuubiki Installer",
        "kyuubiki-installer-gui",
    )
}

#[tauri::command]
fn open_docs_index() -> Result<String, String> {
    open_host_path(&hub_docs_file("index.html"))
}

#[tauri::command]
fn open_current_line_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("current-line.html"))
}

#[tauri::command]
fn open_operations_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("operations.html"))
}

#[tauri::command]
fn open_troubleshooting_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("troubleshooting.html"))
}

#[tauri::command]
fn open_accuracy_plan_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("accuracy-plan.html"))
}

#[tauri::command]
fn open_accuracy_baselines_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("accuracy-baselines.html"))
}

#[tauri::command]
fn open_testing_and_ci_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("testing-and-ci.html"))
}

#[tauri::command]
fn open_direct_mesh_baseline() -> Result<String, String> {
    open_host_path(
        &workspace_root()
            .join("tests")
            .join("integration")
            .join("benchmarks")
            .join("direct-mesh-docker-baseline.json"),
    )
}

#[tauri::command]
fn open_direct_mesh_output_dir() -> Result<String, String> {
    open_host_path(&direct_mesh_output_root())
}

#[tauri::command]
fn hub_direct_mesh_regression_snapshot() -> Result<DirectMeshRegressionSnapshotPayload, String> {
    direct_mesh_regression_snapshot()
}

#[tauri::command]
fn hub_regression_gate_report() -> Result<RegressionGateReportPayload, String> {
    let report_path = workspace_root().join("tmp").join("regression-gate-report.json");
    let content = fs::read_to_string(&report_path)
        .map_err(|error| format!("failed to read regression gate report: {error}"))?;
    let mut payload: RegressionGateReportPayload =
        serde_json::from_str(&content).map_err(|error| format!("invalid regression gate report: {error}"))?;
    payload.rendered = format!(
        "overall gate: {} | failing lanes: {} | warning lanes: {}",
        payload.overall_gate_status, payload.failing_lane_count, payload.warning_lane_count
    );
    Ok(payload)
}

#[tauri::command]
fn hub_environment() -> HubEnvironmentPayload {
    HubEnvironmentPayload {
        hub_role: "desktop-orchestration-shell".to_string(),
        workbench_url: "http://127.0.0.1:3000".to_string(),
        orchestrator_url: "http://127.0.0.1:4000".to_string(),
        deployment_mode: std::env::var("KYUUBIKI_DEPLOYMENT_MODE")
            .unwrap_or_else(|_| "local".to_string()),
        host_platform: Platform::current().as_str().to_string(),
        installer_gui_hint: "Use installer-gui for bootstrap and heavier deployment flows."
            .to_string(),
        workbench_gui_hint: "Use workbench-gui for focused modeling and analysis."
            .to_string(),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            service_status,
            hot_service_status,
            read_runtime_log,
            doctor_report,
            desktop_status,
            guarded_mutation_action,
            project_bundle_inspect,
            project_bundle_validate,
            project_bundle_diff,
            launch_workbench_gui,
            launch_installer_gui,
            get_global_language_preference,
            set_global_language_preference,
            open_docs_index,
            open_current_line_doc,
            open_operations_doc,
            open_troubleshooting_doc,
            open_accuracy_plan_doc,
            open_accuracy_baselines_doc,
            open_testing_and_ci_doc,
            open_direct_mesh_baseline,
            open_direct_mesh_output_dir,
            hub_direct_mesh_regression_snapshot,
            hub_regression_gate_report,
            hub_environment
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki hub gui");
}
