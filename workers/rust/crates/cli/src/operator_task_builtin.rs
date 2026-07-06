pub(crate) mod material;

use serde_json::Value;

pub(crate) fn is_agent_native_builtin_operator(operator_id: &str) -> bool {
    material::is_material_builtin_operator(operator_id)
}

pub(crate) fn run_agent_native_builtin_task(
    operator_id: &str,
    task_ir: &Value,
) -> Result<Value, String> {
    material::run_material_builtin_task(operator_id, task_ir)
}
