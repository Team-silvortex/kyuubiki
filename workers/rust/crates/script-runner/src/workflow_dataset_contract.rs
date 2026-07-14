use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const DATASET_SCHEMA_PATH: &str = "schemas/workflow-dataset.schema.json";
const GRAPH_SCHEMA_PATH: &str = "schemas/workflow-graph.schema.json";
const DATASET_EXAMPLE_PATH: &str = "schemas/examples.workflow-dataset.json";
const GRAPH_EXAMPLE_PATH: &str = "schemas/examples.workflow-graph.json";
const DOCS_PATH: &str = "docs/workflow-dataset.md";
const DATASET_SCHEMA_VERSION: &str = "kyuubiki.workflow-dataset/v1";
const GRAPH_SCHEMA_VERSION: &str = "kyuubiki.workflow-graph/v1";
const REQUIRED_DATA_CLASSES: &[&str] = &[
    "study_model",
    "result",
    "field",
    "table",
    "report",
    "export",
    "scalar",
    "metadata",
];

pub(crate) fn run_check_workflow_dataset_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("workflow dataset contract check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-workflow-dataset-contract only accepts --self-test".to_string());
    }
    let issues = check_contracts(root)?;
    if let Some(issue) = issues.first() {
        eprintln!("workflow dataset contract check failed: {issue}");
        return Ok(1);
    }
    println!("workflow dataset contract check passed");
    Ok(0)
}

fn check_contracts(root: &Path) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    let dataset_schema = read_json(root, DATASET_SCHEMA_PATH)?;
    check_schema(&dataset_schema, &mut issues);
    check_graph_schema(&read_json(root, GRAPH_SCHEMA_PATH)?, &mut issues);
    let allowed = data_classes_from_schema(&dataset_schema, &mut issues);
    check_dataset_contract(
        &read_json(root, DATASET_EXAMPLE_PATH)?,
        DATASET_EXAMPLE_PATH,
        &allowed,
        &mut issues,
    );
    check_graph_example(&read_json(root, GRAPH_EXAMPLE_PATH)?, &allowed, &mut issues);
    check_documentation(root, &mut issues)?;
    Ok(issues)
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let dataset_schema = read_json(root, DATASET_SCHEMA_PATH)?;
    let allowed = data_classes_from_schema(&dataset_schema, &mut Vec::new());
    let mut example = read_json(root, DATASET_EXAMPLE_PATH)?;
    let Some(values) = example.get_mut("values").and_then(Value::as_array_mut) else {
        return Err("self-test dataset example missing values".to_string());
    };
    if let Some(first) = values.first().cloned() {
        values.push(first);
    }
    let mut issues = Vec::new();
    check_dataset_contract(&example, DATASET_EXAMPLE_PATH, &allowed, &mut issues);
    if issues.is_empty() {
        return Err("self-test did not reject duplicate dataset value id".to_string());
    }
    Ok(())
}

fn check_schema(schema: &Value, issues: &mut Vec<String>) {
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(DATASET_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{DATASET_SCHEMA_PATH}: schema_version const must be {DATASET_SCHEMA_VERSION}"
        ));
    }
    let enum_values = data_classes_from_schema(schema, issues);
    for data_class in REQUIRED_DATA_CLASSES {
        if !enum_values.iter().any(|value| value == data_class) {
            issues.push(format!(
                "{DATASET_SCHEMA_PATH}: data_class enum is missing {data_class}"
            ));
        }
    }
    for (pointer, label) in [
        (
            "/$defs/valueInfo/properties/element_type/minLength",
            "element_type must require minLength 1",
        ),
        (
            "/$defs/axis/properties/id/minLength",
            "axis.id must require minLength 1",
        ),
        (
            "/$defs/axis/properties/label/minLength",
            "axis.label must require minLength 1 when present",
        ),
        (
            "/$defs/axis/properties/semantic/minLength",
            "axis.semantic must require minLength 1 when present",
        ),
        (
            "/$defs/valueInfo/properties/unit/minLength",
            "unit must require minLength 1 when present",
        ),
        (
            "/$defs/schemaRef/properties/schema/minLength",
            "schema_ref.schema must require minLength 1",
        ),
    ] {
        if schema.pointer(pointer).and_then(Value::as_i64) != Some(1) {
            issues.push(format!("{DATASET_SCHEMA_PATH}: {label}"));
        }
    }
}

fn data_classes_from_schema(schema: &Value, issues: &mut Vec<String>) -> Vec<String> {
    let values = schema
        .pointer("/$defs/valueInfo/properties/data_class/enum")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.is_empty() {
        issues.push(format!(
            "{DATASET_SCHEMA_PATH}: data_class enum must be a non-empty array"
        ));
    }
    values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn check_graph_schema(schema: &Value, issues: &mut Vec<String>) {
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(GRAPH_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{GRAPH_SCHEMA_PATH}: schema_version const must be {GRAPH_SCHEMA_VERSION}"
        ));
    }
    if schema
        .pointer("/properties/dataset_contract/$ref")
        .and_then(Value::as_str)
        != Some("workflow-dataset.schema.json")
    {
        issues.push(format!(
            "{GRAPH_SCHEMA_PATH}: dataset_contract must reference workflow-dataset.schema.json"
        ));
    }
}

fn check_dataset_contract(
    contract: &Value,
    context: &str,
    allowed: &[String],
    issues: &mut Vec<String>,
) -> BTreeMap<String, Value> {
    if field(contract, "schema_version") != DATASET_SCHEMA_VERSION {
        issues.push(format!(
            "{context}: schema_version must be {DATASET_SCHEMA_VERSION}"
        ));
    }
    require_string(contract, "id", context, issues);
    require_string(contract, "version", context, issues);
    let values = contract
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.is_empty() {
        issues.push(format!("{context}: values must be a non-empty array"));
    }
    let mut value_ids = BTreeSet::new();
    let mut by_id = BTreeMap::new();
    for (index, value) in values.iter().enumerate() {
        let value_context = format!("{context}#values/{index}");
        require_string(value, "id", &value_context, issues);
        let id = field(value, "id");
        if !id.is_empty() && !value_ids.insert(id.to_string()) {
            issues.push(format!("{context}: duplicate dataset value id {id}"));
        }
        if !allowed
            .iter()
            .any(|item| item == field(value, "data_class"))
        {
            issues.push(format!(
                "{value_context}: unsupported data_class {}",
                field(value, "data_class")
            ));
        }
        require_string(value, "element_type", &value_context, issues);
        require_optional_string(value, "semantic_type", &value_context, issues);
        require_optional_string(value, "unit", &value_context, issues);
        check_shape(
            value.get("shape").unwrap_or(&Value::Null),
            &value_context,
            issues,
        );
        if let Some(schema_ref) = value.get("schema_ref") {
            require_string(
                schema_ref,
                "schema",
                &format!("{value_context}.schema_ref"),
                issues,
            );
            require_string(
                schema_ref,
                "version",
                &format!("{value_context}.schema_ref"),
                issues,
            );
        }
        by_id.insert(id.to_string(), value.clone());
    }
    by_id
}

fn check_shape(shape: &Value, context: &str, issues: &mut Vec<String>) {
    let axes = shape
        .get("axes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut axis_ids = BTreeSet::new();
    for (index, axis) in axes.iter().enumerate() {
        require_string(axis, "id", &format!("shape.axes/{index}.id"), issues);
        require_optional_string(axis, "label", &format!("shape.axes/{index}.label"), issues);
        require_optional_string(
            axis,
            "semantic",
            &format!("shape.axes/{index}.semantic"),
            issues,
        );
        let id = field(axis, "id");
        if !id.is_empty() && !axis_ids.insert(id.to_string()) {
            issues.push(format!("{context}: duplicate shape axis id {id}"));
        }
    }
}

fn check_graph_example(graph: &Value, allowed: &[String], issues: &mut Vec<String>) {
    if field(graph, "schema_version") != GRAPH_SCHEMA_VERSION {
        issues.push(format!(
            "{GRAPH_EXAMPLE_PATH}: schema_version must be {GRAPH_SCHEMA_VERSION}"
        ));
    }
    let values = check_dataset_contract(
        graph.get("dataset_contract").unwrap_or(&Value::Null),
        &format!("{GRAPH_EXAMPLE_PATH}#dataset_contract"),
        allowed,
        issues,
    );
    let nodes = graph
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let node_by_id = nodes
        .iter()
        .map(|node| (field(node, "id").to_string(), node.clone()))
        .collect::<BTreeMap<_, _>>();
    for node in &nodes {
        for port in ports(node, "inputs")
            .into_iter()
            .chain(ports(node, "outputs"))
        {
            let dataset_value = field(port, "dataset_value");
            if dataset_value.is_empty() {
                continue;
            }
            let Some(value) = values.get(dataset_value) else {
                issues.push(format!(
                    "{GRAPH_EXAMPLE_PATH}: port {}.{} references unknown dataset value {dataset_value}",
                    field(node, "id"),
                    field(port, "id")
                ));
                continue;
            };
            if !field(value, "semantic_type").is_empty()
                && field(value, "semantic_type") != field(port, "artifact_type")
            {
                issues.push(format!(
                    "{GRAPH_EXAMPLE_PATH}: port {}.{} artifact_type does not match dataset semantic_type",
                    field(node, "id"),
                    field(port, "id")
                ));
            }
        }
    }
    for edge in graph
        .get("edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        check_edge(edge, &node_by_id, &values, issues);
    }
}

fn check_edge(
    edge: &Value,
    node_by_id: &BTreeMap<String, Value>,
    values: &BTreeMap<String, Value>,
    issues: &mut Vec<String>,
) {
    let from_node = edge
        .pointer("/from/node")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let from_port_id = edge
        .pointer("/from/port")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let to_node = edge
        .pointer("/to/node")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let to_port_id = edge
        .pointer("/to/port")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let from_port = node_by_id
        .get(from_node)
        .and_then(|node| find_port(node, "outputs", from_port_id));
    let to_port = node_by_id
        .get(to_node)
        .and_then(|node| find_port(node, "inputs", to_port_id));
    if from_port.is_none() || to_port.is_none() {
        issues.push(format!(
            "{GRAPH_EXAMPLE_PATH}: edge {} references missing port",
            field(edge, "id")
        ));
        return;
    }
    let from_port = from_port.unwrap();
    let to_port = to_port.unwrap();
    let dataset_value = [
        field(edge, "dataset_value"),
        field(from_port, "dataset_value"),
        field(to_port, "dataset_value"),
    ]
    .into_iter()
    .find(|value| !value.is_empty())
    .unwrap_or_default();
    let Some(value) = values.get(dataset_value) else {
        issues.push(format!(
            "{GRAPH_EXAMPLE_PATH}: edge {} references unknown dataset value {dataset_value}",
            field(edge, "id")
        ));
        return;
    };
    if !field(from_port, "dataset_value").is_empty()
        && field(from_port, "dataset_value") != dataset_value
    {
        issues.push(format!(
            "{GRAPH_EXAMPLE_PATH}: edge {} disagrees with source port dataset value",
            field(edge, "id")
        ));
    }
    if !field(to_port, "dataset_value").is_empty()
        && field(to_port, "dataset_value") != dataset_value
    {
        issues.push(format!(
            "{GRAPH_EXAMPLE_PATH}: edge {} disagrees with target port dataset value",
            field(edge, "id")
        ));
    }
    if !field(value, "semantic_type").is_empty()
        && field(value, "semantic_type") != field(edge, "artifact_type")
    {
        issues.push(format!(
            "{GRAPH_EXAMPLE_PATH}: edge {} artifact_type does not match dataset semantic_type",
            field(edge, "id")
        ));
    }
}

fn check_documentation(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    let docs = read_text(root, DOCS_PATH)?;
    for phrase in [
        "dataset contract `id` and `version` must be non-empty",
        "dataset value ids inside one contract must be non-empty and unique",
        "`data_class` must stay inside the stable workflow dataset class set",
        "`element_type`, `schema_ref`, shape axis ids, and optional shape axis text",
        "shape axis ids must be unique inside each dataset value",
    ] {
        if !docs.contains(phrase) {
            issues.push(format!("{DOCS_PATH}: missing runtime rule \"{phrase}\""));
        }
    }
    Ok(())
}

fn ports<'a>(node: &'a Value, key: &str) -> Vec<&'a Value> {
    node.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect()
}

fn find_port<'a>(node: &'a Value, key: &str, port_id: &str) -> Option<&'a Value> {
    ports(node, key)
        .into_iter()
        .find(|port| field(port, "id") == port_id)
}

fn require_string(value: &Value, key: &str, context: &str, issues: &mut Vec<String>) {
    if field(value, key).trim().is_empty() {
        issues.push(format!("{context}: {key} must be a non-empty string"));
    }
}

fn require_optional_string(value: &Value, key: &str, context: &str, issues: &mut Vec<String>) {
    if value.get(key).is_some() && field(value, key).trim().is_empty() {
        issues.push(format!("{context}: {key} must be a non-empty string"));
    }
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::REQUIRED_DATA_CLASSES;

    #[test]
    fn required_classes_include_result_and_metadata() {
        assert!(REQUIRED_DATA_CLASSES.contains(&"result"));
        assert!(REQUIRED_DATA_CLASSES.contains(&"metadata"));
    }
}
