use super::prelude::*;
use serde_json::{Value, json};

fn sparse_graph() -> WorkflowGraph {
    serde_json::from_value(json!({"schema_version":"kyuubiki.workflow-graph/v1",
        "id":"typed-graph", "name":"Typed graph", "version":"1.0.0", "entry_nodes":["input"],
        "nodes":[{"id":"input","kind":"input","inputs":[],
            "outputs":[{"id":"payload","artifact_type":"artifact/json"}]},
            {"id":"output","kind":"output","inputs":[{"id":"payload","artifact_type":"artifact/json"}],"outputs":[]}],
        "edges":[{"id":"output-edge","from":{"node":"input","port":"payload"},
            "to":{"node":"output","port":"payload"},"artifact_type":"artifact/json"}]})).unwrap()
}

#[test]
fn typed_workflow_omits_unset_authoring_fields_without_dropping_explicit_values() {
    let mut graph = sparse_graph();
    let sparse = serde_json::to_value(&graph).unwrap();
    for key in ["description", "dataset_contract"] {
        assert!(sparse.get(key).is_none());
    }
    assert_eq!(sparse["defaults"], json!({}));
    for node in sparse["nodes"].as_array().unwrap() {
        for key in [
            "operator_id",
            "name",
            "description",
            "config",
            "cache_policy",
        ] {
            assert!(node.get(key).is_none());
        }
    }
    assert_eq!(
        sparse["nodes"][0]["outputs"][0],
        json!({"id":"payload","artifact_type":"artifact/json"})
    );
    assert!(sparse["edges"][0].get("dataset_value").is_none());
    graph.defaults.orchestrated = Some(false);
    graph.nodes[0].outputs[0].required = Some(false);
    graph.nodes[0].config = Some(json!({"gain":0}));
    let wire = serde_json::to_value(&graph).unwrap();
    assert_eq!(wire["defaults"]["orchestrated"], false);
    assert_eq!(wire["nodes"][0]["outputs"][0]["required"], false);
    assert_eq!(wire["nodes"][0]["config"], json!({"gain":0}));
    assert_eq!(
        serde_json::from_value::<WorkflowGraph>(wire).unwrap(),
        graph
    );
}

#[test]
fn typed_dataset_contract_retains_schema_marker_and_omits_unset_metadata() {
    let wire = json!({"schema_version":"kyuubiki.workflow-dataset/v1","id":"typed-values","version":"1.0.0",
        "values":[{"id":"summary","data_class":"report","element_type":"json_object",
            "shape":{"axes":[{"id":"samples"}]}}]});
    let contract: WorkflowDatasetContract = serde_json::from_value(wire.clone()).unwrap();
    let emitted = serde_json::to_value(&contract).unwrap();
    assert_eq!(emitted["schema_version"], wire["schema_version"]);
    assert_eq!(emitted["values"], wire["values"]);
    assert_eq!(
        serde_json::from_value::<WorkflowDatasetContract>(emitted).unwrap(),
        contract
    );
    let built = WorkflowDatasetContract {
        id: "rust-built".into(),
        version: "1.0.0".into(),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(built).unwrap()["schema_version"],
        wire["schema_version"]
    );
}

#[test]
fn typed_dataset_contract_rejects_missing_or_unsupported_schema_marker() {
    let valid = json!({"schema_version":"kyuubiki.workflow-dataset/v1","id":"typed-values","version":"1.0.0","values":[]});
    for invalid in [
        Value::Null,
        json!(""),
        json!("kyuubiki.workflow-dataset/v2"),
        json!(1),
    ] {
        let mut wire = valid.clone();
        wire["schema_version"] = invalid;
        assert!(serde_json::from_value::<WorkflowDatasetContract>(wire).is_err());
    }
    let mut missing = valid;
    missing.as_object_mut().unwrap().remove("schema_version");
    assert!(serde_json::from_value::<WorkflowDatasetContract>(missing).is_err());
}

#[test]
fn workflow_dataset_data_classes_match_schema_enum() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/workflow-dataset.schema.json"
    ))
    .expect("workflow dataset schema should parse");
    let enum_values = schema
        .pointer("/$defs/valueInfo/properties/data_class/enum")
        .and_then(|value| value.as_array())
        .expect("workflow dataset schema should expose data_class enum")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("data_class enum values should be strings")
        })
        .collect::<Vec<_>>();

    assert_eq!(enum_values, WORKFLOW_DATASET_DATA_CLASSES);
}
