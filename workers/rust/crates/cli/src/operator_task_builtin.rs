pub(crate) mod material;
mod workflow_diagnostics;

use serde_json::Value;

pub(crate) fn is_agent_native_builtin_operator(operator_id: &str) -> bool {
    material::is_material_builtin_operator(operator_id)
        || workflow_diagnostics::supports(operator_id)
}

pub(crate) fn run_agent_native_builtin_task(
    operator_id: &str,
    task_ir: &Value,
) -> Result<Value, String> {
    if workflow_diagnostics::supports(operator_id) {
        workflow_diagnostics::run(operator_id, task_ir)
    } else {
        material::run_material_builtin_task(operator_id, task_ir)
    }
}
