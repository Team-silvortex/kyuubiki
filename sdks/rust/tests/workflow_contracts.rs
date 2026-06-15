use kyuubiki_headless_sdk::{SdkError, WorkflowDatasetContract, WorkflowGraphDefinition};

#[test]
fn validates_reference_workflow_examples() {
    let dataset: WorkflowDatasetContract =
        serde_json::from_str(include_str!("../../../schemas/examples.workflow-dataset.json")).expect("dataset example");
    dataset.validate().expect("dataset validates");

    let graph: WorkflowGraphDefinition =
        serde_json::from_str(include_str!("../../../schemas/examples.workflow-graph.json")).expect("graph example");
    graph.validate().expect("graph validates");
}

#[test]
fn rejects_unknown_dataset_value_reference() {
    let mut graph: WorkflowGraphDefinition =
        serde_json::from_str(include_str!("../../../schemas/examples.workflow-graph.json")).expect("graph example");
    graph.edges[0].dataset_value = Some("missing_value".into());
    match graph.validate() {
        Err(SdkError::Validation { errors }) => {
            assert!(errors.iter().any(|item| item.contains("missing_value")));
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn validates_execution_hints() {
    let mut graph: WorkflowGraphDefinition =
        serde_json::from_str(include_str!("../../../schemas/examples.workflow-graph.json")).expect("graph example");
    graph.dispatch_policy = Some("central_fetch".into());
    graph.placement_tags = vec!["mesh-enabled".into()];
    graph.required_capabilities = vec!["artifact-cache".into()];
    graph.operator_fetch_plan = vec![kyuubiki_headless_sdk::WorkflowOperatorFetchEntry {
        node_id: "thermal_solve".into(),
        operator_id: "solve.thermal.steady_state".into(),
        package_ref: Some("kyuubiki://operators/solve.thermal.steady_state".into()),
        version: Some("1.0.0".into()),
        integrity: Some("sha256:demo".into()),
        cache_scope: Some("agent".into()),
    }];
    graph.defaults = Some(kyuubiki_headless_sdk::WorkflowDefaults {
        cache_policy: Some("cached".into()),
        orchestrated: Some(false),
        dispatch_policy: Some("central_fetch".into()),
        placement_tags: vec!["cpu".into()],
        required_capabilities: vec!["solver.thermal".into()],
    });
    graph.nodes[1].placement_tags = vec!["gpu-preferred".into()];
    graph.nodes[1].required_capabilities = vec!["solver.thermal".into()];
    graph.validate().expect("graph validates");
}

#[test]
fn rejects_invalid_dispatch_policy() {
    let mut graph: WorkflowGraphDefinition =
        serde_json::from_str(include_str!("../../../schemas/examples.workflow-graph.json")).expect("graph example");
    graph.dispatch_policy = Some("mystery_mode".into());
    match graph.validate() {
        Err(SdkError::Validation { errors }) => {
            assert!(errors.iter().any(|item| item.contains("dispatch_policy")));
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}
