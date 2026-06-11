use kyuubiki_headless_sdk::{
    workflow_axis, workflow_dataset_contract, workflow_dataset_value, workflow_edge, workflow_graph, workflow_node, workflow_port,
    workflow_schema_ref, workflow_shape, WORKFLOW_DATASET_SCHEMA_VERSION, WORKFLOW_GRAPH_SCHEMA_VERSION,
};

#[test]
fn builds_valid_dataset_contract() {
    let contract = workflow_dataset_contract(
        "dataset.demo/v1",
        "1.0.0",
        vec![workflow_dataset_value("thermal_case", "study_model", "json_object")
            .with_shape(workflow_shape(vec![workflow_axis("elements").with_semantic("mesh_element")]))
            .with_encoding("json")
            .with_schema_ref(workflow_schema_ref("kyuubiki.operator.demo.input", "1"))],
    );
    assert_eq!(contract.schema_version, WORKFLOW_DATASET_SCHEMA_VERSION);
    contract.validate().expect("dataset validates");
}

#[test]
fn builds_valid_graph() {
    let dataset = workflow_dataset_contract(
        "dataset.demo/v1",
        "1.0.0",
        vec![
            workflow_dataset_value("thermal_case", "study_model", "json_object"),
            workflow_dataset_value("thermal_result", "result", "json_object"),
        ],
    );
    let graph = workflow_graph(
        "workflow.demo",
        "Demo workflow",
        "1.0.0",
        vec!["input".into()],
        vec![
            workflow_node("input", "input").with_outputs(vec![workflow_port("case", "study_model/demo").with_dataset_value("thermal_case")]),
            workflow_node("solve", "solve")
                .with_operator_id("solve.demo")
                .with_inputs(vec![workflow_port("case", "study_model/demo").with_dataset_value("thermal_case")])
                .with_outputs(vec![workflow_port("result", "result/demo").with_dataset_value("thermal_result")]),
            workflow_node("output", "output").with_inputs(vec![workflow_port("result", "result/demo").with_dataset_value("thermal_result")]),
        ],
        vec![
            workflow_edge("edge-1", "input", "case", "solve", "case", "study_model/demo").with_dataset_value("thermal_case"),
            workflow_edge("edge-2", "solve", "result", "output", "result", "result/demo").with_dataset_value("thermal_result"),
        ],
    )
    .with_dataset_contract(dataset)
    .with_output_nodes(vec!["output".into()]);
    assert_eq!(graph.schema_version, WORKFLOW_GRAPH_SCHEMA_VERSION);
    graph.validate().expect("graph validates");
}
