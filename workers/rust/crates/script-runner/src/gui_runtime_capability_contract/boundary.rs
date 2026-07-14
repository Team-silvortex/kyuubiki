use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(super) fn check_runtime_client_boundary(
    root: &Path,
    issues: &mut Vec<String>,
) -> RunnerResult<()> {
    for (file, needles) in boundary_needles() {
        let text = read_text(root, file)?;
        for needle in needles {
            require_contains(&text, needle, file, issues);
        }
    }
    for file in WORKBENCH_RUNTIME_SERVICE_FILES {
        let text = read_text(root, file)?;
        require_contains(
            &text,
            "defaultWorkbenchRuntimeBackedBackendServices",
            file,
            issues,
        );
        if imports_from_module(
            &text,
            "@/lib/api/runtime-client",
            &["fetch", "submit", "create", "update", "delete", "cancel"],
        ) {
            issues.push(format!("{file}: Workbench services must use the runtime client instance instead of importing runtime functions directly"));
        }
    }
    check_project_and_headless_boundaries(root, issues)?;
    check_security_result_boundaries(root, issues)
}

fn check_project_and_headless_boundaries(
    root: &Path,
    issues: &mut Vec<String>,
) -> RunnerResult<()> {
    let project_library = read_text(root, PROJECT_LIBRARY_SERVICE)?;
    if imports_from_module(
        &project_library,
        "@/lib/api/project-client",
        &["create", "fetch", "update", "delete"],
    ) {
        issues.push(format!("{PROJECT_LIBRARY_SERVICE}: project library service must use the project client instance instead of importing API functions directly"));
    }
    let workbench_root = read_text(root, WORKBENCH_ROOT)?;
    if imports_from_module(
        &workbench_root,
        "@/lib/api",
        &[
            "createProject",
            "fetchProject",
            "updateProject",
            "deleteProject",
            "createModel",
            "fetchModel",
        ],
    ) {
        issues.push(format!("{WORKBENCH_ROOT}: Workbench root must use project library backend service instead of importing project API functions directly"));
    }
    let headless_execution = read_text(root, HEADLESS_EXECUTION)?;
    if headless_execution.contains("from \"@/lib/api\"")
        || headless_execution.contains("from '@/lib/api'")
    {
        issues.push(format!("{HEADLESS_EXECUTION}: headless execution must import concrete API contract files instead of the API facade"));
    }
    let panel = read_text(root, HEADLESS_WORKFLOW_PANEL)?;
    if imports_from_module(
        &panel,
        "@/lib/api",
        &[
            "fetchProtocolAgents",
            "submitHeadlessOrchestraHandoff",
            "fetchHeadlessOrchestraHandoff",
        ],
    ) {
        issues.push(format!("{HEADLESS_WORKFLOW_PANEL}: headless workflow panel must use the backend service instead of importing headless/runtime API functions directly"));
    }
    Ok(())
}

fn check_security_result_boundaries(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    for file in [
        BACKEND_SERVICE_COMPOSER,
        RESULT_SERVICE,
        SECURITY_EVENT_SERVICE,
    ] {
        let text = read_text(root, file)?;
        if imports_from_module(
            &text,
            "@/lib/api/security-results-client",
            &["fetch", "submit", "create", "update", "delete", "cancel"],
        ) {
            issues.push(format!("{file}: Workbench services must use the security results client instance instead of importing API functions directly"));
        }
    }
    Ok(())
}

fn boundary_needles() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        (
            RUNTIME_CLIENT,
            vec!["createRuntimeApiClient", "defaultRuntimeApiClient"],
        ),
        (
            SECURITY_RESULTS_CLIENT,
            vec![
                "createSecurityResultsApiClient",
                "defaultSecurityResultsApiClient",
            ],
        ),
        (
            PROJECT_CLIENT,
            vec!["createProjectApiClient", "defaultProjectApiClient"],
        ),
        (
            HEADLESS_RESULTS_CLIENT,
            vec![
                "createHeadlessResultsApiClient",
                "defaultHeadlessResultsApiClient",
            ],
        ),
        (
            HEADLESS_HANDOFF_CLIENT,
            vec![
                "createHeadlessHandoffApiClient",
                "defaultHeadlessHandoffApiClient",
            ],
        ),
        (
            BACKEND_SERVICE_COMPOSER,
            vec![
                "createWorkbenchRuntimeBackedBackendServices",
                "defaultWorkbenchRuntimeBackedBackendServices",
                "defaultRuntimeApiClient",
                "defaultSecurityResultsApiClient",
            ],
        ),
        (PROJECT_LIBRARY_SERVICE, vec!["defaultProjectApiClient"]),
        (
            WORKBENCH_ROOT,
            vec!["workbenchProjectLibraryBackendService"],
        ),
        (
            HEADLESS_EXECUTION,
            vec![
                "HeadlessExecutionBackendClients",
                "defaultHeadlessResultsApiClient",
                "defaultProjectApiClient",
                "defaultRuntimeApiClient",
            ],
        ),
        (
            HEADLESS_WORKFLOW_BACKEND_SERVICE,
            vec![
                "createWorkbenchHeadlessWorkflowBackendService",
                "defaultHeadlessHandoffApiClient",
                "defaultRuntimeApiClient",
            ],
        ),
        (
            HEADLESS_WORKFLOW_PANEL,
            vec![
                "defaultWorkbenchHeadlessWorkflowBackendService",
                "workbench-headless-workflow-panel-actions",
                "workbench-headless-workflow-panel-controls",
                "workbench-headless-workflow-panel-library",
                "workbench-headless-workflow-step-list",
                "workbench-headless-workflow-panel-state",
            ],
        ),
        (
            HEADLESS_WORKFLOW_PANEL_ACTIONS,
            vec![
                "buildHeadlessAgentDispatchPlanFromBackend",
                "buildHeadlessOrchestraHandoffFromBackend",
                "submitHeadlessOrchestraHandoffFromBackend",
            ],
        ),
        (
            HEADLESS_WORKFLOW_PANEL_CONTROLS,
            vec!["WorkbenchHeadlessWorkflowPanelControls", "executionLog"],
        ),
        (
            HEADLESS_WORKFLOW_PANEL_LIBRARY,
            vec![
                "WorkbenchHeadlessFrontendAssetCatalog",
                "WorkbenchHeadlessTemplateCatalog",
                "WorkbenchHeadlessActionButtons",
            ],
        ),
        (
            HEADLESS_WORKFLOW_PANEL_STATE,
            vec![
                "buildFrontendMacroBridgePayload",
                "parseFrontendMacroBridgePayload",
                "moveItem",
            ],
        ),
        (
            HEADLESS_WORKFLOW_STEP_LIST,
            vec![
                "WorkbenchHeadlessWorkflowStepList",
                "WorkbenchHeadlessWorkflowStepEditor",
            ],
        ),
        (RESULT_SERVICE, vec!["defaultSecurityResultsApiClient"]),
        (
            SECURITY_EVENT_SERVICE,
            vec!["defaultSecurityResultsApiClient"],
        ),
        (
            "apps/frontend/src/lib/workbench/study-run-backend-service.ts",
            vec!["createStudyRunBackendServiceFromRuntimeClient"],
        ),
    ]
}

const RUNTIME_CLIENT: &str = "apps/frontend/src/lib/api/runtime-client.ts";
const SECURITY_RESULTS_CLIENT: &str = "apps/frontend/src/lib/api/security-results-client.ts";
const PROJECT_CLIENT: &str = "apps/frontend/src/lib/api/project-client.ts";
const HEADLESS_RESULTS_CLIENT: &str = "apps/frontend/src/lib/api/headless-results-client.ts";
const HEADLESS_HANDOFF_CLIENT: &str = "apps/frontend/src/lib/api/headless-handoff-client.ts";
const BACKEND_SERVICE_COMPOSER: &str =
    "apps/frontend/src/lib/workbench/backend-service-composer.ts";
const WORKBENCH_ROOT: &str = "apps/frontend/src/components/workbench/workbench.tsx";
const PROJECT_LIBRARY_SERVICE: &str =
    "apps/frontend/src/lib/workbench/project-library-backend-service.ts";
const HEADLESS_EXECUTION: &str = "apps/frontend/src/lib/scripting/workbench-headless-execution.ts";
const HEADLESS_WORKFLOW_PANEL: &str =
    "apps/frontend/src/components/workbench/workbench-headless-workflow-panel.tsx";
const HEADLESS_WORKFLOW_PANEL_ACTIONS: &str =
    "apps/frontend/src/components/workbench/workbench-headless-workflow-panel-actions.ts";
const HEADLESS_WORKFLOW_PANEL_CONTROLS: &str =
    "apps/frontend/src/components/workbench/workbench-headless-workflow-panel-controls.tsx";
const HEADLESS_WORKFLOW_PANEL_LIBRARY: &str =
    "apps/frontend/src/components/workbench/workbench-headless-workflow-panel-library.tsx";
const HEADLESS_WORKFLOW_PANEL_STATE: &str =
    "apps/frontend/src/components/workbench/workbench-headless-workflow-panel-state.ts";
const HEADLESS_WORKFLOW_STEP_LIST: &str =
    "apps/frontend/src/components/workbench/workbench-headless-workflow-step-list.tsx";
const HEADLESS_WORKFLOW_BACKEND_SERVICE: &str =
    "apps/frontend/src/lib/workbench/headless-workflow-backend-service.ts";
const RESULT_SERVICE: &str = "apps/frontend/src/lib/workbench/result-backend-service.ts";
const SECURITY_EVENT_SERVICE: &str =
    "apps/frontend/src/lib/workbench/security-event-backend-service.ts";
const WORKBENCH_RUNTIME_SERVICE_FILES: &[&str] = &[
    "apps/frontend/src/lib/workbench/admin-data-backend-service.ts",
    "apps/frontend/src/lib/workbench/job-history-backend-service.ts",
    "apps/frontend/src/lib/workbench/runtime-status-backend-service.ts",
    "apps/frontend/src/lib/workbench/workflow-backend-service.ts",
];

fn imports_from_module(text: &str, module: &str, prefixes: &[&str]) -> bool {
    for chunk in text.split("import").skip(1) {
        let Some((imports, rest)) = chunk.split_once("from") else {
            continue;
        };
        if !(rest.contains(&format!("\"{module}\"")) || rest.contains(&format!("'{module}'"))) {
            continue;
        }
        if prefixes.iter().any(|prefix| imports.contains(prefix)) {
            return true;
        }
    }
    false
}

fn require_contains(text: &str, needle: &str, context: &str, issues: &mut Vec<String>) {
    if !text.contains(needle) {
        issues.push(format!("{context}: missing {needle}"));
    }
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::imports_from_module;

    #[test]
    fn import_scanner_finds_direct_api_imports() {
        assert!(imports_from_module(
            "import { fetchRuntime } from \"@/lib/api/runtime-client\";",
            "@/lib/api/runtime-client",
            &["fetch"]
        ));
    }
}
