use serde::Serialize;

pub const PROTOCOL_BENCHMARK_SURFACE_SCHEMA_VERSION: &str =
    "kyuubiki.protocol-benchmark-surface/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtocolBenchmarkSurface {
    pub schema_version: &'static str,
    pub protocol_owner: &'static str,
    pub lanes: Vec<ProtocolBenchmarkLane>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtocolBenchmarkLane {
    pub id: &'static str,
    pub evidence_scope: &'static str,
    pub protocol_contracts: &'static [&'static str],
    pub regression_commands: &'static [&'static str],
}

pub fn protocol_benchmark_surface() -> ProtocolBenchmarkSurface {
    ProtocolBenchmarkSurface {
        schema_version: PROTOCOL_BENCHMARK_SURFACE_SCHEMA_VERSION,
        protocol_owner: "runtime-protocol",
        lanes: vec![
            ProtocolBenchmarkLane {
                id: "rpc_descriptor",
                evidence_scope: "agent RPC shape, methods, and descriptor serialization",
                protocol_contracts: &[
                    "kyuubiki.rpc/v1",
                    "kyuubiki.agent-descriptor/v1",
                    "kyuubiki.control-plane/http-v1",
                ],
                regression_commands: &["cargo test -p kyuubiki-protocol rpc_descriptor"],
            },
            ProtocolBenchmarkLane {
                id: "operator_task_ir",
                evidence_scope: "TaskIR digest, execution summary, and capability admission",
                protocol_contracts: &[
                    "kyuubiki.operator-task-ir/v1",
                    "kyuubiki.operator-execution/v1",
                    "kyuubiki.solver-execution-capability/v1",
                ],
                regression_commands: &[
                    "cargo test -p kyuubiki-protocol operator_task_ir",
                    "cargo test -p kyuubiki-protocol solver_execution_capability",
                ],
            },
            ProtocolBenchmarkLane {
                id: "workflow_contract",
                evidence_scope: "Workflow graph, dataset contract, and lineage serialization",
                protocol_contracts: &[
                    "kyuubiki.workflow-graph/v1",
                    "kyuubiki.workflow-dataset/v1",
                    "kyuubiki.workflow-lineage/v1",
                ],
                regression_commands: &[
                    "cargo test -p kyuubiki-protocol workflows",
                    "cargo test -p kyuubiki-protocol workflow_dataset_contract",
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{PROTOCOL_BENCHMARK_SURFACE_SCHEMA_VERSION, protocol_benchmark_surface};

    #[test]
    fn protocol_benchmark_surface_is_serializable_and_scoped() {
        let surface = protocol_benchmark_surface();

        assert_eq!(
            surface.schema_version,
            PROTOCOL_BENCHMARK_SURFACE_SCHEMA_VERSION
        );
        assert!(surface.lanes.iter().any(|lane| {
            lane.id == "operator_task_ir"
                && lane
                    .protocol_contracts
                    .contains(&"kyuubiki.solver-execution-capability/v1")
        }));
        assert!(surface.lanes.iter().any(|lane| {
            lane.id == "workflow_contract"
                && lane
                    .regression_commands
                    .iter()
                    .any(|command| command.contains("workflow_dataset_contract"))
        }));
        serde_json::to_value(surface).expect("protocol benchmark surface should serialize");
    }
}
