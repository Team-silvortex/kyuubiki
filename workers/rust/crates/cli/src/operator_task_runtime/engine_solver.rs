use super::{OPERATOR_TASK_MODE_EXECUTE, OperatorTaskRuntimeError};
use kyuubiki_engine::{EngineSolveRequest, solve};
use kyuubiki_protocol::{
    AnalysisResult, OperatorTaskExecutionSummary, SolveBarRequest, SolverExecutionCapability,
    check_operator_task_execution_capability,
};
use serde_json::Value;

pub(super) const AGENT_ENGINE_SOLVER_STATUS: &str = "agent_engine_solver_executed";

pub(super) struct EngineSolverExecution {
    pub capability_report: Value,
    pub result: Value,
}

pub(super) fn try_execute(
    mode: &str,
    summary: &OperatorTaskExecutionSummary,
    task_ir: &Value,
) -> Result<Option<EngineSolverExecution>, OperatorTaskRuntimeError> {
    if mode != OPERATOR_TASK_MODE_EXECUTE || summary.program_kind != "solver" {
        return Ok(None);
    }

    let capability = SolverExecutionCapability::agent_builtin();
    let report =
        check_operator_task_execution_capability(task_ir, &capability).map_err(|error| {
            OperatorTaskRuntimeError::with_task(
                "operator_task_solver_capability_invalid",
                error.message,
                "check_solver_capability",
                Some(task_ir),
            )
        })?;
    if !report.accepted {
        return Err(OperatorTaskRuntimeError::with_task(
            "operator_task_solver_capability_rejected",
            report.rejection_reasons.join("; "),
            "check_solver_capability",
            Some(task_ir),
        ));
    }

    let result = dispatch_engine_solver(&summary.operator_id, task_ir)?;
    let capability_report = serde_json::to_value(report).map_err(|error| {
        OperatorTaskRuntimeError::with_task(
            "operator_task_solver_capability_invalid",
            format!("failed to serialize solver capability report: {error}"),
            "check_solver_capability",
            Some(task_ir),
        )
    })?;
    Ok(Some(EngineSolverExecution {
        capability_report,
        result,
    }))
}

fn dispatch_engine_solver(
    operator_id: &str,
    task_ir: &Value,
) -> Result<Value, OperatorTaskRuntimeError> {
    let input = task_ir.get("input_artifact").cloned().ok_or_else(|| {
        OperatorTaskRuntimeError::with_task(
            "operator_task_solver_input_invalid",
            "solver TaskIR is missing input_artifact",
            "decode_solver_input",
            Some(task_ir),
        )
    })?;
    let request = match operator_id {
        "solve.bar_1d" => serde_json::from_value::<SolveBarRequest>(input)
            .map(EngineSolveRequest::Bar1d)
            .map_err(|error| {
                OperatorTaskRuntimeError::with_task(
                    "operator_task_solver_input_invalid",
                    format!("invalid solve.bar_1d input_artifact: {error}"),
                    "decode_solver_input",
                    Some(task_ir),
                )
            })?,
        _ => {
            return Err(OperatorTaskRuntimeError::with_task(
                "operator_task_solver_capability_rejected",
                format!("Agent engine does not advertise TaskIR solver {operator_id}"),
                "check_solver_capability",
                Some(task_ir),
            ));
        }
    };

    match solve(request).map_err(|error| {
        OperatorTaskRuntimeError::with_task(
            "operator_task_solver_execution_failed",
            error,
            "dispatch_engine_solver",
            Some(task_ir),
        )
    })? {
        AnalysisResult::Bar1d(result) => serde_json::to_value(result).map_err(|error| {
            OperatorTaskRuntimeError::with_task(
                "operator_task_solver_result_invalid",
                format!("failed to serialize solve.bar_1d result: {error}"),
                "serialize_solver_result",
                Some(task_ir),
            )
        }),
        _ => Err(OperatorTaskRuntimeError::with_task(
            "operator_task_solver_result_invalid",
            "Agent engine returned a mismatched solver result variant",
            "serialize_solver_result",
            Some(task_ir),
        )),
    }
}
