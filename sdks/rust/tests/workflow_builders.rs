use kyuubiki_headless_sdk::{
    WORKFLOW_DATASET_SCHEMA_VERSION, WORKFLOW_GRAPH_SCHEMA_VERSION, workflow_axis,
    workflow_dataset_contract, workflow_dataset_value, workflow_defaults, workflow_edge,
    workflow_graph, workflow_node, workflow_operator_fetch_entry, workflow_port,
    workflow_schema_ref, workflow_shape,
};

#[test]
fn builds_valid_dataset_contract() {
    let contract = workflow_dataset_contract(
        "dataset.demo/v1",
        "1.0.0",
        vec![
            workflow_dataset_value("thermal_case", "study_model", "json_object")
                .with_shape(workflow_shape(vec![
                    workflow_axis("elements").with_semantic("mesh_element"),
                ]))
                .with_encoding("json")
                .with_schema_ref(workflow_schema_ref("kyuubiki.operator.demo.input", "1")),
        ],
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
            workflow_node("input", "input").with_outputs(vec![
                workflow_port("case", "study_model/demo").with_dataset_value("thermal_case"),
            ]),
            workflow_node("solve", "solve")
                .with_operator_id("solve.demo")
                .with_placement_tags(vec!["gpu-preferred".into()])
                .with_required_capabilities(vec!["solver.thermal".into()])
                .with_inputs(vec![
                    workflow_port("case", "study_model/demo").with_dataset_value("thermal_case"),
                ])
                .with_outputs(vec![
                    workflow_port("result", "result/demo").with_dataset_value("thermal_result"),
                ]),
            workflow_node("output", "output").with_inputs(vec![
                workflow_port("result", "result/demo").with_dataset_value("thermal_result"),
            ]),
        ],
        vec![
            workflow_edge(
                "edge-1",
                "input",
                "case",
                "solve",
                "case",
                "study_model/demo",
            )
            .with_dataset_value("thermal_case"),
            workflow_edge(
                "edge-2",
                "solve",
                "result",
                "output",
                "result",
                "result/demo",
            )
            .with_dataset_value("thermal_result"),
        ],
    )
    .with_dataset_contract(dataset)
    .with_output_nodes(vec!["output".into()])
    .with_defaults(
        workflow_defaults()
            .with_cache_policy("cached")
            .with_orchestrated(false)
            .with_dispatch_policy("central_fetch")
            .with_placement_tags(vec!["cpu".into()])
            .with_required_capabilities(vec!["solver.thermal".into()]),
    )
    .with_dispatch_policy("central_fetch")
    .with_operator_fetch_plan(vec![
        workflow_operator_fetch_entry("solve", "solve.demo")
            .with_package_ref("kyuubiki://operators/solve.demo")
            .with_version("1.0.0")
            .with_integrity("sha256:demo")
            .with_cache_scope("agent"),
    ])
    .with_placement_tags(vec!["mesh-enabled".into()])
    .with_required_capabilities(vec!["artifact-cache".into()]);
    assert_eq!(graph.schema_version, WORKFLOW_GRAPH_SCHEMA_VERSION);
    assert_eq!(graph.dispatch_policy.as_deref(), Some("central_fetch"));
    assert_eq!(
        graph.defaults.as_ref().and_then(|item| item.orchestrated),
        Some(false)
    );
    graph.validate().expect("graph validates");
}
