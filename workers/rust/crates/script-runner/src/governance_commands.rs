use crate::{
    central_database_readiness, central_readiness_report, central_store_contract,
    commercial_readiness, component_integrity_protocol, contracts_runtime_api_surface,
    dependency_audit, docs_book, frontend_checks, gui_runtime_capability_contract,
    install_update_disk_hygiene, installation_integrity_docs, language_packs, local_path_audit,
    make_modules, material_exploration_chain_contract, material_score_contract,
    material_study_execution_plan_contract, materialization_plan_contract,
    minimal_industrial_closure, module_extension_standard, module_function_matrix,
    module_function_tensor, module_topology, module_topology_report, operator_task_ir_contract,
    project_organization_audit, toolchain_contract, ui_automation_contract, update_catalog_docs,
    verification_evidence_surface, workflow_dataset_contract,
};
use std::ffi::OsString;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_governance_command(
    root: &Path,
    frontend: &Path,
    command: &str,
    args: Vec<OsString>,
) -> Option<RunnerResult<u8>> {
    Some(match command {
        "check-make-modules" => make_modules::run_check_make_modules(root, args),
        "check-doc-book" => docs_book::run_check_doc_book(root, args),
        "sync-doc-book-version" => docs_book::run_sync_doc_book_version(root, args),
        "check-toolchain-contract" => toolchain_contract::run_check_toolchain_contract(root, args),
        "check-install-update-disk-hygiene" => {
            install_update_disk_hygiene::run_check_install_update_disk_hygiene(root, args)
        }
        "check-component-integrity-protocol" => {
            component_integrity_protocol::run_check_component_integrity_protocol(root, args)
        }
        "build-installation-integrity-docs" => {
            installation_integrity_docs::run_build_installation_integrity_docs(root, args)
        }
        "build-update-catalog" => update_catalog_docs::run_build_update_catalog(root, args),
        "check-module-topology" => module_topology::run_check_module_topology(root, args),
        "build-module-topology-report" => {
            module_topology_report::run_build_module_topology_report(root, args)
        }
        "check-module-function-matrix" => {
            module_function_matrix::run_check_module_function_matrix(root, args)
        }
        "check-module-function-coverage-tensor" => {
            module_function_tensor::run_check_module_function_tensor(root, args)
        }
        "check-module-extension-standard" => {
            module_extension_standard::run_check_module_extension_standard(root, args)
        }
        "check-verification-evidence-surface" => {
            verification_evidence_surface::run_check_verification_evidence_surface(root, args)
        }
        "check-central-store-contract" => {
            central_store_contract::run_check_central_store_contract(root, args)
        }
        "check-central-database-readiness" => {
            central_database_readiness::run_check_central_database_readiness(root, args)
        }
        "build-central-readiness-report" => {
            central_readiness_report::run_build_central_readiness_report(root, args)
        }
        "check-central-readiness-report" => {
            central_readiness_report::run_check_central_readiness_report(root, args)
        }
        "check-contracts-runtime-api-surface" => {
            contracts_runtime_api_surface::run_check_contracts_runtime_api_surface(root, args)
        }
        "validate-language-packs" => language_packs::run_validate_language_packs(root, args),
        "validate-commercial-readiness" => {
            commercial_readiness::run_validate_commercial_readiness(root, args)
        }
        "validate-minimal-industrial-closure" => {
            minimal_industrial_closure::run_validate_minimal_industrial_closure(root, args)
        }
        "check-ui-automation-contract" => {
            ui_automation_contract::run_check_ui_automation_contract(root, args)
        }
        "check-gui-runtime-capability-contract" => {
            gui_runtime_capability_contract::run_check_gui_runtime_capability_contract(root, args)
        }
        "check-workflow-dataset-contract" => {
            workflow_dataset_contract::run_check_workflow_dataset_contract(root, args)
        }
        "check-materialization-plan-contract" => {
            materialization_plan_contract::run_check_materialization_plan_contract(root, args)
        }
        "check-material-study-execution-plan-contract" => {
            material_study_execution_plan_contract::run_check_material_study_execution_plan_contract(
                root, args,
            )
        }
        "check-material-exploration-chain-contract" => {
            material_exploration_chain_contract::run_check_material_exploration_chain_contract(
                root, args,
            )
        }
        "check-operator-task-ir-contract" => {
            operator_task_ir_contract::run_check_operator_task_ir_contract(root, args)
        }
        "validate-material-score-contract" => {
            material_score_contract::run_validate_material_score_contract(root, args)
        }
        "audit-local-paths" => local_path_audit::run_audit_local_paths(root, args),
        "audit-project-organization" => {
            project_organization_audit::run_audit_project_organization(root, args)
        }
        "audit-dependencies" => dependency_audit::run_audit_dependencies(root, args),
        "frontend-file-lines" => frontend_checks::run_frontend_file_lines(frontend, args),
        "frontend-storage-security" => {
            frontend_checks::run_frontend_storage_security(frontend, args)
        }
        _ => return None,
    })
}
