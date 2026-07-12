use crate::error::{SdkError, SdkResult};
use crate::workflow_contracts::{
    WORKFLOW_DATASET_SCHEMA_VERSION, WORKFLOW_DISPATCH_POLICIES, WORKFLOW_GRAPH_SCHEMA_VERSION,
    WorkflowDatasetContract, WorkflowGraphDefinition, WorkflowGraphPort,
    WorkflowOperatorFetchEntry,
};
use std::collections::{HashMap, HashSet};

const WORKFLOW_DATASET_DATA_CLASSES: &[&str] = &[
    "study_model",
    "result",
    "field",
    "table",
    "report",
    "export",
    "scalar",
    "metadata",
];

impl WorkflowDatasetContract {
    pub fn validate(&self) -> SdkResult<()> {
        let mut errors = Vec::new();
        if self.schema_version != WORKFLOW_DATASET_SCHEMA_VERSION {
            errors.push(format!(
                "dataset.schema_version must be {WORKFLOW_DATASET_SCHEMA_VERSION:?}"
            ));
        }
        require_string(&self.id, "dataset.id", &mut errors);
        require_string(&self.version, "dataset.version", &mut errors);
        if self.values.is_empty() {
            errors.push("dataset.values must contain at least 1 item".into());
        }
        let mut ids = HashSet::new();
        for (index, value) in self.values.iter().enumerate() {
            let path = format!("dataset.values[{index}]");
            require_string(&value.id, &format!("{path}.id"), &mut errors);
            if !value.id.is_empty() && !ids.insert(value.id.as_str()) {
                errors.push(format!(
                    "dataset.values contains duplicate id {:?}",
                    value.id
                ));
            }
            require_string(
                &value.data_class,
                &format!("{path}.data_class"),
                &mut errors,
            );
            require_string(
                &value.element_type,
                &format!("{path}.element_type"),
                &mut errors,
            );
            if !WORKFLOW_DATASET_DATA_CLASSES.contains(&value.data_class.as_str()) {
                errors.push(format!("{path}.data_class is invalid"));
            }
            validate_optional_non_empty(
                value.semantic_type.as_deref(),
                &format!("{path}.semantic_type"),
                &mut errors,
            );
            validate_optional_non_empty(
                value.unit.as_deref(),
                &format!("{path}.unit"),
                &mut errors,
            );
            if let Some(encoding) = &value.encoding {
                if !matches!(
                    encoding.as_str(),
                    "json" | "json_lines" | "f64_le" | "f32_le" | "i64_le" | "i32_le" | "u8"
                ) {
                    errors.push(format!("{path}.encoding is invalid"));
                }
            }
            if let Some(schema_ref) = &value.schema_ref {
                require_string(
                    &schema_ref.schema,
                    &format!("{path}.schema_ref.schema"),
                    &mut errors,
                );
                require_string(
                    &schema_ref.version,
                    &format!("{path}.schema_ref.version"),
                    &mut errors,
                );
            }
            let mut axis_ids = HashSet::new();
            for (axis_index, axis) in value.shape.axes.iter().enumerate() {
                let axis_path = format!("{path}.shape.axes[{axis_index}]");
                require_string(&axis.id, &format!("{axis_path}.id"), &mut errors);
                if !axis.id.is_empty() && !axis_ids.insert(axis.id.as_str()) {
                    errors.push(format!(
                        "{path}.shape.axes contains duplicate id {:?}",
                        axis.id
                    ));
                }
                validate_optional_non_empty(
                    axis.label.as_deref(),
                    &format!("{axis_path}.label"),
                    &mut errors,
                );
                validate_optional_non_empty(
                    axis.semantic.as_deref(),
                    &format!("{axis_path}.semantic"),
                    &mut errors,
                );
            }
        }
        finish_validation(errors)
    }
}

impl WorkflowGraphDefinition {
    pub fn validate(&self) -> SdkResult<()> {
        let mut errors = Vec::new();
        if self.schema_version != WORKFLOW_GRAPH_SCHEMA_VERSION {
            errors.push(format!(
                "graph.schema_version must be {WORKFLOW_GRAPH_SCHEMA_VERSION:?}"
            ));
        }
        require_string(&self.id, "graph.id", &mut errors);
        require_string(&self.name, "graph.name", &mut errors);
        require_string(&self.version, "graph.version", &mut errors);
        if self.entry_nodes.is_empty() {
            errors.push("graph.entry_nodes must contain at least 1 item".into());
        }
        if self.nodes.is_empty() {
            errors.push("graph.nodes must contain at least 1 item".into());
        }
        let dataset_semantics = collect_dataset_semantics(self, &mut errors);
        validate_graph_hints(self, &mut errors);
        validate_graph_nodes_and_edges(self, &dataset_semantics, &mut errors);
        finish_validation(errors)
    }
}

fn collect_dataset_semantics<'a>(
    graph: &'a WorkflowGraphDefinition,
    errors: &mut Vec<String>,
) -> HashMap<&'a str, Option<&'a str>> {
    match &graph.dataset_contract {
        Some(contract) => {
            if let Err(SdkError::Validation { errors: nested }) = contract.validate() {
                errors.extend(
                    nested
                        .into_iter()
                        .map(|item| format!("graph.dataset_contract: {item}")),
                );
            }
            contract
                .values
                .iter()
                .map(|value| (value.id.as_str(), value.semantic_type.as_deref()))
                .collect()
        }
        None => HashMap::new(),
    }
}

fn validate_graph_hints(graph: &WorkflowGraphDefinition, errors: &mut Vec<String>) {
    validate_dispatch_policy(
        graph.dispatch_policy.as_deref(),
        "graph.dispatch_policy",
        errors,
    );
    validate_string_list(&graph.placement_tags, "graph.placement_tags", errors);
    validate_string_list(
        &graph.required_capabilities,
        "graph.required_capabilities",
        errors,
    );
    validate_operator_fetch_plan(
        &graph.operator_fetch_plan,
        "graph.operator_fetch_plan",
        errors,
    );
    if let Some(defaults) = &graph.defaults {
        if let Some(cache_policy) = defaults.cache_policy.as_deref() {
            if !matches!(cache_policy, "ephemeral" | "cached" | "persisted") {
                errors.push("graph.defaults.cache_policy is invalid".into());
            }
        }
        validate_dispatch_policy(
            defaults.dispatch_policy.as_deref(),
            "graph.defaults.dispatch_policy",
            errors,
        );
        validate_string_list(
            &defaults.placement_tags,
            "graph.defaults.placement_tags",
            errors,
        );
        validate_string_list(
            &defaults.required_capabilities,
            "graph.defaults.required_capabilities",
            errors,
        );
    }
}

fn validate_graph_nodes_and_edges(
    graph: &WorkflowGraphDefinition,
    dataset_semantics: &HashMap<&str, Option<&str>>,
    errors: &mut Vec<String>,
) {
    let mut node_ids = HashSet::new();
    let mut input_ports = HashMap::new();
    let mut output_ports = HashMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        let path = format!("graph.nodes[{index}]");
        require_string(&node.id, &format!("{path}.id"), errors);
        require_string(&node.kind, &format!("{path}.kind"), errors);
        if !node.id.is_empty() && !node_ids.insert(node.id.as_str()) {
            errors.push(format!("graph.nodes contains duplicate id {:?}", node.id));
        }
        if !matches!(
            node.kind.as_str(),
            "input" | "solve" | "transform" | "extract" | "export" | "condition" | "output"
        ) {
            errors.push(format!("{path}.kind is invalid"));
        }
        if matches!(
            node.kind.as_str(),
            "solve" | "transform" | "extract" | "export" | "condition"
        ) {
            require_option_string(
                node.operator_id.as_deref(),
                &format!("{path}.operator_id"),
                errors,
            );
        }
        validate_string_list(
            &node.placement_tags,
            &format!("{path}.placement_tags"),
            errors,
        );
        validate_string_list(
            &node.required_capabilities,
            &format!("{path}.required_capabilities"),
            errors,
        );
        collect_ports(
            &node.id,
            &node.inputs,
            dataset_semantics,
            &mut input_ports,
            errors,
            &format!("{path}.inputs"),
        );
        collect_ports(
            &node.id,
            &node.outputs,
            dataset_semantics,
            &mut output_ports,
            errors,
            &format!("{path}.outputs"),
        );
    }
    validate_node_refs("graph.entry_nodes", &graph.entry_nodes, &node_ids, errors);
    validate_node_refs("graph.output_nodes", &graph.output_nodes, &node_ids, errors);
    validate_edges(
        graph,
        dataset_semantics,
        &input_ports,
        &output_ports,
        errors,
    );
}

fn validate_edges<'a>(
    graph: &'a WorkflowGraphDefinition,
    dataset_semantics: &HashMap<&str, Option<&str>>,
    input_ports: &HashMap<(&'a str, &'a str), &'a WorkflowGraphPort>,
    output_ports: &HashMap<(&'a str, &'a str), &'a WorkflowGraphPort>,
    errors: &mut Vec<String>,
) {
    let mut edge_ids = HashSet::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        let path = format!("graph.edges[{index}]");
        require_string(&edge.id, &format!("{path}.id"), errors);
        if !edge.id.is_empty() && !edge_ids.insert(edge.id.as_str()) {
            errors.push(format!("graph.edges contains duplicate id {:?}", edge.id));
        }
        require_string(
            &edge.artifact_type,
            &format!("{path}.artifact_type"),
            errors,
        );
        validate_dataset_semantic(
            edge.dataset_value.as_deref(),
            &edge.artifact_type,
            &path,
            dataset_semantics,
            errors,
        );
        let source_key = (edge.from.node.as_str(), edge.from.port.as_str());
        let target_key = (edge.to.node.as_str(), edge.to.port.as_str());
        let source = output_ports.get(&source_key);
        let target = input_ports.get(&target_key);
        validate_edge_endpoint(
            &path,
            "from",
            source_key,
            *source.unwrap_or(&&DUMMY_PORT),
            errors,
        );
        validate_edge_endpoint(
            &path,
            "to",
            target_key,
            *target.unwrap_or(&&DUMMY_PORT),
            errors,
        );
        if let Some(source_port) = source {
            validate_port_edge_match(
                &path,
                "source output",
                edge.dataset_value.as_deref(),
                source_port,
                errors,
            );
        }
        if let Some(target_port) = target {
            validate_port_edge_match(
                &path,
                "target input",
                edge.dataset_value.as_deref(),
                target_port,
                errors,
            );
        }
        if let (Some(source_port), Some(target_port)) = (source, target) {
            if source_port.artifact_type != target_port.artifact_type {
                errors.push(format!(
                    "{path} connects ports with mismatched artifact_type values"
                ));
            }
        }
    }
}

static DUMMY_PORT: WorkflowGraphPort = WorkflowGraphPort {
    id: String::new(),
    name: None,
    artifact_type: String::new(),
    required: None,
    cardinality: None,
    dataset_value: None,
};

fn validate_edge_endpoint(
    path: &str,
    field: &str,
    key: (&str, &str),
    port: &WorkflowGraphPort,
    errors: &mut Vec<String>,
) {
    if port.id.is_empty() {
        errors.push(format!("{path}.{field} references unknown port {:?}", key));
    }
}

fn collect_ports<'a>(
    node_id: &'a str,
    ports: &'a [WorkflowGraphPort],
    dataset_semantics: &HashMap<&str, Option<&str>>,
    bucket: &mut HashMap<(&'a str, &'a str), &'a WorkflowGraphPort>,
    errors: &mut Vec<String>,
    path: &str,
) {
    let mut ids = HashSet::new();
    for (index, port) in ports.iter().enumerate() {
        let port_path = format!("{path}[{index}]");
        require_string(&port.id, &format!("{port_path}.id"), errors);
        require_string(
            &port.artifact_type,
            &format!("{port_path}.artifact_type"),
            errors,
        );
        if !port.id.is_empty() && !ids.insert(port.id.as_str()) {
            errors.push(format!("{path} contains duplicate port id {:?}", port.id));
        }
        if let Some(cardinality) = &port.cardinality {
            if !matches!(cardinality.as_str(), "one" | "many") {
                errors.push(format!("{port_path}.cardinality is invalid"));
            }
        }
        validate_dataset_semantic(
            port.dataset_value.as_deref(),
            &port.artifact_type,
            &port_path,
            dataset_semantics,
            errors,
        );
        if !node_id.is_empty() && !port.id.is_empty() {
            bucket.insert((node_id, port.id.as_str()), port);
        }
    }
}

fn validate_dataset_semantic(
    dataset_value: Option<&str>,
    artifact_type: &str,
    path: &str,
    dataset_semantics: &HashMap<&str, Option<&str>>,
    errors: &mut Vec<String>,
) {
    let Some(dataset_value) = dataset_value else {
        return;
    };
    if dataset_semantics.is_empty() {
        return;
    }
    match dataset_semantics.get(dataset_value) {
        Some(Some(semantic_type)) if *semantic_type != artifact_type => errors.push(format!(
            "{path}.dataset_value {dataset_value:?} semantic_type {:?} does not match artifact_type {:?}",
            semantic_type, artifact_type
        )),
        Some(_) => {}
        None => errors.push(format!(
            "{path}.dataset_value {dataset_value:?} is not declared in graph.dataset_contract"
        )),
    }
}

fn validate_port_edge_match(
    path: &str,
    role: &str,
    edge_dataset_value: Option<&str>,
    port: &WorkflowGraphPort,
    errors: &mut Vec<String>,
) {
    if port.artifact_type.is_empty() {
        return;
    }
    if let Some(edge_dataset_value) = edge_dataset_value {
        if port.dataset_value.as_deref() != Some(edge_dataset_value) {
            errors.push(format!(
                "{path}.dataset_value does not match {role} port dataset_value"
            ));
        }
    }
}

fn validate_node_refs(
    path: &str,
    values: &[String],
    node_ids: &HashSet<&str>,
    errors: &mut Vec<String>,
) {
    for (index, node_id) in values.iter().enumerate() {
        if !node_ids.contains(node_id.as_str()) {
            errors.push(format!(
                "{path}[{index}] references unknown node {:?}",
                node_id
            ));
        }
    }
}

fn finish_validation(errors: Vec<String>) -> SdkResult<()> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(SdkError::Validation { errors })
    }
}

fn require_string(value: &str, path: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{path} must be a non-empty string"));
    }
}

fn require_option_string(value: Option<&str>, path: &str, errors: &mut Vec<String>) {
    match value {
        Some(value) if !value.trim().is_empty() => {}
        _ => errors.push(format!("{path} must be a non-empty string")),
    }
}

fn validate_dispatch_policy(value: Option<&str>, path: &str, errors: &mut Vec<String>) {
    if let Some(value) = value {
        if !WORKFLOW_DISPATCH_POLICIES.contains(&value) {
            errors.push(format!(
                "{path} must be one of {:?}",
                WORKFLOW_DISPATCH_POLICIES
            ));
        }
    }
}

fn validate_string_list(values: &[String], path: &str, errors: &mut Vec<String>) {
    for (index, value) in values.iter().enumerate() {
        if value.trim().is_empty() {
            errors.push(format!("{path}[{index}] must be a non-empty string"));
        }
    }
}

fn validate_operator_fetch_plan(
    entries: &[WorkflowOperatorFetchEntry],
    path: &str,
    errors: &mut Vec<String>,
) {
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = format!("{path}[{index}]");
        require_string(&entry.node_id, &format!("{entry_path}.node_id"), errors);
        require_string(
            &entry.operator_id,
            &format!("{entry_path}.operator_id"),
            errors,
        );
        validate_optional_non_empty(
            entry.package_ref.as_deref(),
            &format!("{entry_path}.package_ref"),
            errors,
        );
        validate_optional_non_empty(
            entry.version.as_deref(),
            &format!("{entry_path}.version"),
            errors,
        );
        validate_optional_non_empty(
            entry.integrity.as_deref(),
            &format!("{entry_path}.integrity"),
            errors,
        );
        validate_optional_non_empty(
            entry.cache_scope.as_deref(),
            &format!("{entry_path}.cache_scope"),
            errors,
        );
    }
}

fn validate_optional_non_empty(value: Option<&str>, path: &str, errors: &mut Vec<String>) {
    if let Some(value) = value {
        if value.trim().is_empty() {
            errors.push(format!("{path} must be a non-empty string"));
        }
    }
}
