use super::prelude::*;

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
