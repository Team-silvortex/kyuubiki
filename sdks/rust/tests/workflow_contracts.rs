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
