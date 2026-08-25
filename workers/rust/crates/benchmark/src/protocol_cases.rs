use std::collections::BTreeMap;

use kyuubiki_protocol::{WorkflowGraph, WorkflowGraphRunRequest};
use serde_json::Value;

use crate::models::{BenchmarkCase, BenchmarkWorkload};

pub(crate) const PROTOCOL_MATRIX: &str = "protocol";

pub(crate) fn protocol_cases() -> Vec<BenchmarkCase> {
    vec![
        BenchmarkCase {
            id: "operator-task-ir-preview".to_string(),
            family: "protocol_operator_task_ir",
            workload: BenchmarkWorkload::ProtocolOperatorTaskPreview(operator_task_fixture()),
        },
        BenchmarkCase {
            id: "workflow-graph-roundtrip".to_string(),
            family: "protocol_workflow_graph",
            workload: BenchmarkWorkload::ProtocolWorkflowRoundTrip(workflow_request_fixture()),
        },
    ]
}

pub(crate) fn is_protocol_matrix(matrix: &str) -> bool {
    matrix == PROTOCOL_MATRIX
}

fn operator_task_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../schemas/examples.operator-task-ir.json"
    ))
    .expect("operator task benchmark fixture should parse")
}

fn workflow_request_fixture() -> WorkflowGraphRunRequest {
    let graph = serde_json::from_str::<WorkflowGraph>(include_str!(
        "../../../../../schemas/examples.workflow-graph.json"
    ))
    .expect("workflow graph benchmark fixture should parse");
    WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use kyuubiki_protocol::preview_operator_task_execution;

    use super::{PROTOCOL_MATRIX, is_protocol_matrix, protocol_cases};

    #[test]
    fn protocol_matrix_owns_task_ir_and_workflow_round_trip_cases() {
        let cases = protocol_cases();

        assert_eq!(cases.len(), 2);
        assert!(is_protocol_matrix(PROTOCOL_MATRIX));
        assert_eq!(cases[0].id, "operator-task-ir-preview");
        assert_eq!(cases[1].id, "workflow-graph-roundtrip");
    }

    #[test]
    fn operator_task_fixture_passes_protocol_preview() {
        let cases = protocol_cases();
        let crate::models::BenchmarkWorkload::ProtocolOperatorTaskPreview(task) =
            &cases[0].workload
        else {
            panic!("first protocol benchmark should be TaskIR preview");
        };

        preview_operator_task_execution(task).expect("TaskIR fixture should pass preview");
    }
}
