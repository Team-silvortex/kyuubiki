use kyuubiki_protocol::WORKFLOW_DATASET_DATA_CLASSES;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessWorkflowDatasetPreflightReport {
    pub ok: bool,
    pub issue_count: usize,
    pub issues: Vec<String>,
    pub dataset_value_count: usize,
    pub referenced_dataset_values: Vec<String>,
    pub unresolved_dataset_values: Vec<String>,
}

pub fn preflight_workflow_dataset_contract(
    graph: &Value,
) -> HeadlessWorkflowDatasetPreflightReport {
    let mut issues = Vec::new();
    let contract_values = collect_contract_values(graph, &mut issues);
    let referenced = collect_dataset_references(graph, &contract_values, &mut issues);
    let unresolved = referenced
        .iter()
        .filter(|id| !contract_values.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    for id in &unresolved {
        issues.push(format!(
            "dataset_value {id} is referenced by graph ports or edges but is missing from dataset_contract.values"
        ));
    }
    HeadlessWorkflowDatasetPreflightReport {
        ok: issues.is_empty(),
        issue_count: issues.len(),
        issues,
        dataset_value_count: contract_values.len(),
        referenced_dataset_values: referenced.into_iter().collect(),
        unresolved_dataset_values: unresolved,
    }
}

fn collect_contract_values(
    graph: &Value,
    issues: &mut Vec<String>,
) -> BTreeMap<String, DatasetValueSummary> {
    let Some(contract) = graph.get("dataset_contract") else {
        if has_dataset_references(graph) {
            issues.push("graph references dataset_value but has no dataset_contract".to_string());
        }
        return BTreeMap::new();
    };
    let Some(contract) = contract.as_object() else {
        issues.push("dataset_contract must be an object".to_string());
        return BTreeMap::new();
    };
    let schema_version = text_field(graph.get("dataset_contract"), "schema_version");
    if schema_version.as_deref() != Some("kyuubiki.workflow-dataset/v1") {
        issues.push(
            "dataset_contract.schema_version must be kyuubiki.workflow-dataset/v1".to_string(),
        );
    }
    for key in ["id", "version"] {
        if text_field(graph.get("dataset_contract"), key).is_none() {
            issues.push(format!("dataset_contract.{key} is missing or empty"));
        }
    }
    let Some(values) = contract.get("values").and_then(Value::as_array) else {
        issues.push("dataset_contract.values must be an array".to_string());
        return BTreeMap::new();
    };
    let mut summaries = BTreeMap::new();
    for (index, value) in values.iter().enumerate() {
        collect_contract_value(index, value, &mut summaries, issues);
    }
    summaries
}

fn collect_contract_value(
    index: usize,
    value: &Value,
    summaries: &mut BTreeMap<String, DatasetValueSummary>,
    issues: &mut Vec<String>,
) {
    let label = format!("dataset_contract.values[{}]", index);
    let Some(id) = text_field(Some(value), "id") else {
        issues.push(format!("{label}.id is missing or empty"));
        return;
    };
    if summaries.contains_key(&id) {
        issues.push(format!(
            "dataset_contract.values contains duplicate id {id}"
        ));
    }
    let data_class = text_field(Some(value), "data_class");
    if let Some(data_class) = data_class.as_deref() {
        if !WORKFLOW_DATASET_DATA_CLASSES.contains(&data_class) {
            issues.push(format!("{label}.data_class {data_class} is not supported"));
        }
    } else {
        issues.push(format!("{label}.data_class is missing or empty"));
    }
    if text_field(Some(value), "element_type").is_none() {
        issues.push(format!("{label}.element_type is missing or empty"));
    }
    validate_optional_text(value, "semantic_type", &label, issues);
    validate_optional_text(value, "unit", &label, issues);
    validate_schema_ref(value, &label, issues);
    validate_shape_axes(value, &label, issues);
    summaries.insert(
        id,
        DatasetValueSummary {
            semantic_type: text_field(Some(value), "semantic_type"),
        },
    );
}

fn validate_schema_ref(value: &Value, label: &str, issues: &mut Vec<String>) {
    if let Some(schema_ref) = value.get("schema_ref") {
        for key in ["schema", "version"] {
            if text_field(Some(schema_ref), key).is_none() {
                issues.push(format!("{label}.schema_ref.{key} is missing or empty"));
            }
        }
    }
}

fn validate_shape_axes(value: &Value, label: &str, issues: &mut Vec<String>) {
    let Some(axes) = value
        .get("shape")
        .and_then(|shape| shape.get("axes"))
        .and_then(Value::as_array)
    else {
        return;
    };
    let mut ids = BTreeSet::new();
    for (axis_index, axis) in axes.iter().enumerate() {
        let axis_label = format!("{label}.shape.axes[{axis_index}]");
        let Some(id) = text_field(Some(axis), "id") else {
            issues.push(format!("{axis_label}.id is missing or empty"));
            continue;
        };
        if !ids.insert(id.clone()) {
            issues.push(format!("{axis_label}.id duplicates axis id {id}"));
        }
        validate_optional_text(axis, "label", &axis_label, issues);
        validate_optional_text(axis, "semantic", &axis_label, issues);
    }
}

fn collect_dataset_references(
    graph: &Value,
    summaries: &BTreeMap<String, DatasetValueSummary>,
    issues: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    let mut ports = BTreeMap::new();
    collect_port_dataset_references(graph, summaries, &mut ports, &mut refs, issues);
    collect_edge_dataset_references(graph, summaries, &ports, &mut refs, issues);
    refs
}

fn collect_port_dataset_references(
    graph: &Value,
    summaries: &BTreeMap<String, DatasetValueSummary>,
    ports: &mut BTreeMap<String, PortSummary>,
    refs: &mut BTreeSet<String>,
    issues: &mut Vec<String>,
) {
    let Some(nodes) = graph.get("nodes").and_then(Value::as_array) else {
        return;
    };
    for node in nodes {
        let node_id =
            text_field(Some(node), "id").unwrap_or_else(|| "<missing-node-id>".to_string());
        for direction in ["inputs", "outputs"] {
            if let Some(port_array) = node.get(direction).and_then(Value::as_array) {
                for port in port_array {
                    collect_port_reference(&node_id, port, summaries, ports, refs, issues);
                }
            }
        }
    }
}

fn collect_port_reference(
    node_id: &str,
    port: &Value,
    summaries: &BTreeMap<String, DatasetValueSummary>,
    ports: &mut BTreeMap<String, PortSummary>,
    refs: &mut BTreeSet<String>,
    issues: &mut Vec<String>,
) {
    let Some(port_id) = text_field(Some(port), "id") else {
        return;
    };
    let Some(dataset_value) = text_field(Some(port), "dataset_value") else {
        return;
    };
    refs.insert(dataset_value.clone());
    let artifact_type = text_field(Some(port), "artifact_type");
    validate_artifact_matches_dataset(&dataset_value, artifact_type.as_deref(), summaries, issues);
    ports.insert(
        format!("{node_id}:{port_id}"),
        PortSummary {
            dataset_value,
            artifact_type,
        },
    );
}

fn collect_edge_dataset_references(
    graph: &Value,
    summaries: &BTreeMap<String, DatasetValueSummary>,
    ports: &BTreeMap<String, PortSummary>,
    refs: &mut BTreeSet<String>,
    issues: &mut Vec<String>,
) {
    let Some(edges) = graph.get("edges").and_then(Value::as_array) else {
        return;
    };
    for edge in edges {
        let Some(edge_id) = text_field(Some(edge), "id") else {
            continue;
        };
        if let Some(dataset_value) = text_field(Some(edge), "dataset_value") {
            refs.insert(dataset_value.clone());
            let artifact_type = text_field(Some(edge), "artifact_type");
            validate_artifact_matches_dataset(
                &dataset_value,
                artifact_type.as_deref(),
                summaries,
                issues,
            );
            validate_edge_endpoint(
                &edge_id,
                "from",
                edge,
                &dataset_value,
                artifact_type.as_deref(),
                ports,
                issues,
            );
            validate_edge_endpoint(
                &edge_id,
                "to",
                edge,
                &dataset_value,
                artifact_type.as_deref(),
                ports,
                issues,
            );
        }
    }
}

fn validate_edge_endpoint(
    edge_id: &str,
    endpoint: &str,
    edge: &Value,
    dataset_value: &str,
    artifact_type: Option<&str>,
    ports: &BTreeMap<String, PortSummary>,
    issues: &mut Vec<String>,
) {
    let Some(endpoint_value) = edge.get(endpoint) else {
        return;
    };
    let Some(node) = text_field(Some(endpoint_value), "node") else {
        return;
    };
    let Some(port) = text_field(Some(endpoint_value), "port") else {
        return;
    };
    if let Some(summary) = ports.get(&format!("{node}:{port}")) {
        if summary.dataset_value != dataset_value {
            issues.push(format!(
                "edge {edge_id} {endpoint} endpoint {node}:{port} dataset_value {} does not match edge dataset_value {dataset_value}",
                summary.dataset_value
            ));
        }
        if artifact_type.is_some() && summary.artifact_type.as_deref() != artifact_type {
            issues.push(format!(
                "edge {edge_id} {endpoint} endpoint {node}:{port} artifact_type {:?} does not match edge artifact_type {:?}",
                summary.artifact_type, artifact_type
            ));
        }
    }
}

fn validate_artifact_matches_dataset(
    dataset_value: &str,
    artifact_type: Option<&str>,
    summaries: &BTreeMap<String, DatasetValueSummary>,
    issues: &mut Vec<String>,
) {
    if let Some(summary) = summaries.get(dataset_value) {
        if let (Some(expected), Some(actual)) = (summary.semantic_type.as_deref(), artifact_type) {
            if expected != actual {
                issues.push(format!(
                    "dataset_value {dataset_value} semantic_type {expected} does not match artifact_type {actual}"
                ));
            }
        }
    }
}

fn has_dataset_references(graph: &Value) -> bool {
    graph.to_string().contains("\"dataset_value\"")
}

fn validate_optional_text(value: &Value, key: &str, label: &str, issues: &mut Vec<String>) {
    if value.get(key).is_some() && text_field(Some(value), key).is_none() {
        issues.push(format!("{label}.{key} is empty"));
    }
}

fn text_field(value: Option<&Value>, key: &str) -> Option<String> {
    value?
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|it| !it.is_empty())
        .map(ToString::to_string)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatasetValueSummary {
    semantic_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PortSummary {
    dataset_value: String,
    artifact_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_graph() -> Value {
        serde_json::from_str(include_str!(
            "../../../../../schemas/examples.workflow-graph.json"
        ))
        .expect("example workflow graph should parse")
    }

    #[test]
    fn workflow_dataset_preflight_accepts_example_graph() {
        let report = preflight_workflow_dataset_contract(&example_graph());
        assert!(report.ok, "{:?}", report.issues);
        assert_eq!(report.dataset_value_count, 4);
        assert!(report.unresolved_dataset_values.is_empty());
    }

    #[test]
    fn workflow_dataset_preflight_rejects_duplicate_contract_values() {
        let mut graph = example_graph();
        let duplicate = graph["dataset_contract"]["values"][0].clone();
        graph["dataset_contract"]["values"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        let report = preflight_workflow_dataset_contract(&graph);
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("duplicate id heat_model"))
        );
    }

    #[test]
    fn workflow_dataset_preflight_rejects_unresolved_dataset_value() {
        let mut graph = example_graph();
        graph["edges"][0]["dataset_value"] = Value::String("missing_value".to_string());
        let report = preflight_workflow_dataset_contract(&graph);
        assert!(!report.ok);
        assert_eq!(report.unresolved_dataset_values, vec!["missing_value"]);
    }

    #[test]
    fn workflow_dataset_preflight_rejects_endpoint_mismatch() {
        let mut graph = example_graph();
        graph["nodes"][1]["inputs"][0]["dataset_value"] = Value::String("heat_result".to_string());
        let report = preflight_workflow_dataset_contract(&graph);
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("does not match edge dataset_value heat_model"))
        );
    }
}
