use serde_json::Value;

use crate::operator_package_fetch_runtime::{
    finalize_orchestra_operator_package, prepare_orchestra_operator_package,
};
use crate::operator_package_runtime::{
    ExternalOperatorExecution, ExternalOperatorTaskError, OperatorPackageRuntimeBinding,
    try_execute_external_operator_task,
};
use kyuubiki_protocol::OperatorTaskExecutionSummary;

use super::OperatorTaskRuntimeError;

pub(super) fn try_execute(
    summary: &OperatorTaskExecutionSummary,
    task_ir: &Value,
    job_id: Option<&str>,
    package_runtime: &mut OperatorPackageRuntimeBinding,
) -> Result<Option<ExternalOperatorExecution>, OperatorTaskRuntimeError> {
    let prepared = prepare_orchestra_operator_package(summary, job_id)
        .map_err(|error| runtime_error(error, task_ir))?;
    if let Some(prepared) = prepared.as_ref() {
        *package_runtime = prepared.binding.clone();
    }

    let execution = try_execute_external_operator_task(
        summary,
        task_ir,
        package_runtime,
        prepared.as_ref().map(|prepared| prepared.cache_status),
        prepared.as_ref().map(|prepared| &prepared.runtime_lease),
    );
    let eviction = finalize_orchestra_operator_package(summary, prepared.as_ref());
    let (execution, eviction) = match (execution, eviction) {
        (Ok(execution), Ok(eviction)) => (execution, eviction),
        (Err(execution), Ok(eviction)) => {
            return Err(runtime_error_with_eviction(execution, task_ir, eviction));
        }
        (Ok(Some(_)), Err(eviction)) => {
            return Err(OperatorTaskRuntimeError::with_task(
                eviction.code,
                format!(
                    "external operator execution completed but cache eviction failed: {}",
                    eviction.message
                ),
                eviction.stage,
                Some(task_ir),
            ));
        }
        (Ok(None), Err(eviction)) => return Err(runtime_error(eviction, task_ir)),
        (Err(execution), Err(eviction)) => {
            return Err(OperatorTaskRuntimeError::with_task(
                eviction.code,
                format!(
                    "external operator execution failed: {}; cache eviction also failed: {}",
                    execution.message, eviction.message
                ),
                eviction.stage,
                Some(task_ir),
            ));
        }
    };

    Ok(execution.map(|mut execution| {
        if let Some(eviction) = eviction {
            execution
                .package_receipt
                .as_object_mut()
                .expect("external package receipt must remain an object")
                .insert("cache_eviction".to_string(), eviction);
        }
        execution
    }))
}

fn runtime_error(error: ExternalOperatorTaskError, task_ir: &Value) -> OperatorTaskRuntimeError {
    OperatorTaskRuntimeError::with_task(error.code, error.message, error.stage, Some(task_ir))
}

fn runtime_error_with_eviction(
    error: ExternalOperatorTaskError,
    task_ir: &Value,
    eviction: Option<Value>,
) -> OperatorTaskRuntimeError {
    let mut error = runtime_error(error, task_ir);
    if let (Some(details), Some(eviction)) = (error.details.as_object_mut(), eviction) {
        details.insert("cache_eviction".to_string(), eviction);
    }
    error
}
