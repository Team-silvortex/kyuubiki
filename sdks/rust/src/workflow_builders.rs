use crate::workflow_contracts::{
    WORKFLOW_DATASET_SCHEMA_VERSION, WORKFLOW_GRAPH_SCHEMA_VERSION, WorkflowAxis,
    WorkflowDatasetContract, WorkflowDatasetValue, WorkflowDefaults, WorkflowGraphDefinition,
    WorkflowGraphEdge, WorkflowGraphNode, WorkflowGraphPort, WorkflowNodePortRef,
    WorkflowOperatorFetchEntry, WorkflowSchemaRef, WorkflowShape,
};
use std::collections::HashMap;

pub fn workflow_schema_ref(
    schema: impl Into<String>,
    version: impl Into<String>,
) -> WorkflowSchemaRef {
    WorkflowSchemaRef {
        schema: schema.into(),
        version: version.into(),
    }
}

pub fn workflow_axis(axis_id: impl Into<String>) -> WorkflowAxis {
    WorkflowAxis {
        id: axis_id.into(),
        ..WorkflowAxis::default()
    }
}

pub fn workflow_shape(axes: Vec<WorkflowAxis>) -> WorkflowShape {
    WorkflowShape { axes }
}

pub fn workflow_dataset_value(
    value_id: impl Into<String>,
    data_class: impl Into<String>,
    element_type: impl Into<String>,
) -> WorkflowDatasetValue {
    WorkflowDatasetValue {
        id: value_id.into(),
        data_class: data_class.into(),
        element_type: element_type.into(),
        shape: WorkflowShape::default(),
        semantic_type: None,
        unit: None,
        encoding: None,
        schema_ref: None,
    }
}

pub fn workflow_dataset_contract(
    contract_id: impl Into<String>,
    version: impl Into<String>,
    values: Vec<WorkflowDatasetValue>,
) -> WorkflowDatasetContract {
    WorkflowDatasetContract {
        schema_version: WORKFLOW_DATASET_SCHEMA_VERSION.into(),
        id: contract_id.into(),
        version: version.into(),
        name: None,
        description: None,
        values,
        metadata: HashMap::new(),
    }
}

pub fn workflow_port(
    port_id: impl Into<String>,
    artifact_type: impl Into<String>,
) -> WorkflowGraphPort {
    WorkflowGraphPort {
        id: port_id.into(),
        name: None,
        artifact_type: artifact_type.into(),
        required: None,
        cardinality: None,
        dataset_value: None,
    }
}

pub fn workflow_defaults() -> WorkflowDefaults {
    WorkflowDefaults::default()
}

pub fn workflow_operator_fetch_entry(
    node_id: impl Into<String>,
    operator_id: impl Into<String>,
) -> WorkflowOperatorFetchEntry {
    WorkflowOperatorFetchEntry {
        node_id: node_id.into(),
        operator_id: operator_id.into(),
        package_ref: None,
        version: None,
        integrity: None,
        cache_scope: None,
    }
}

pub fn workflow_node(node_id: impl Into<String>, kind: impl Into<String>) -> WorkflowGraphNode {
    WorkflowGraphNode {
        id: node_id.into(),
        kind: kind.into(),
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        placement_tags: Vec::new(),
        required_capabilities: Vec::new(),
        inputs: Vec::new(),
        outputs: Vec::new(),
    }
}

pub fn workflow_edge(
    edge_id: impl Into<String>,
    from_node: impl Into<String>,
    from_port: impl Into<String>,
    to_node: impl Into<String>,
    to_port: impl Into<String>,
    artifact_type: impl Into<String>,
) -> WorkflowGraphEdge {
    WorkflowGraphEdge {
        id: edge_id.into(),
        from: WorkflowNodePortRef {
            node: from_node.into(),
            port: from_port.into(),
        },
        to: WorkflowNodePortRef {
            node: to_node.into(),
            port: to_port.into(),
        },
        artifact_type: artifact_type.into(),
        dataset_value: None,
    }
}

pub fn workflow_graph(
    graph_id: impl Into<String>,
    name: impl Into<String>,
    version: impl Into<String>,
    entry_nodes: Vec<String>,
    nodes: Vec<WorkflowGraphNode>,
    edges: Vec<WorkflowGraphEdge>,
) -> WorkflowGraphDefinition {
    WorkflowGraphDefinition {
        schema_version: WORKFLOW_GRAPH_SCHEMA_VERSION.into(),
        id: graph_id.into(),
        name: name.into(),
        version: version.into(),
        description: None,
        dataset_contract: None,
        entry_nodes,
        output_nodes: Vec::new(),
        defaults: None,
        dispatch_policy: None,
        operator_fetch_plan: Vec::new(),
        placement_tags: Vec::new(),
        required_capabilities: Vec::new(),
        nodes,
        edges,
    }
}

impl WorkflowAxis {
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_semantic(mut self, semantic: impl Into<String>) -> Self {
        self.semantic = Some(semantic.into());
        self
    }
}

impl WorkflowDatasetValue {
    pub fn with_shape(mut self, shape: WorkflowShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn with_semantic_type(mut self, semantic_type: impl Into<String>) -> Self {
        self.semantic_type = Some(semantic_type.into());
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn with_encoding(mut self, encoding: impl Into<String>) -> Self {
        self.encoding = Some(encoding.into());
        self
    }

    pub fn with_schema_ref(mut self, schema_ref: WorkflowSchemaRef) -> Self {
        self.schema_ref = Some(schema_ref);
        self
    }
}

impl WorkflowDatasetContract {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl WorkflowGraphPort {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn with_cardinality(mut self, cardinality: impl Into<String>) -> Self {
        self.cardinality = Some(cardinality.into());
        self
    }

    pub fn with_dataset_value(mut self, dataset_value: impl Into<String>) -> Self {
        self.dataset_value = Some(dataset_value.into());
        self
    }
}

impl WorkflowGraphNode {
    pub fn with_operator_id(mut self, operator_id: impl Into<String>) -> Self {
        self.operator_id = Some(operator_id.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_cache_policy(mut self, cache_policy: impl Into<String>) -> Self {
        self.cache_policy = Some(cache_policy.into());
        self
    }

    pub fn with_placement_tags(mut self, placement_tags: Vec<String>) -> Self {
        self.placement_tags = placement_tags;
        self
    }

    pub fn with_required_capabilities(mut self, required_capabilities: Vec<String>) -> Self {
        self.required_capabilities = required_capabilities;
        self
    }

    pub fn with_inputs(mut self, inputs: Vec<WorkflowGraphPort>) -> Self {
        self.inputs = inputs;
        self
    }

    pub fn with_outputs(mut self, outputs: Vec<WorkflowGraphPort>) -> Self {
        self.outputs = outputs;
        self
    }
}

impl WorkflowGraphEdge {
    pub fn with_dataset_value(mut self, dataset_value: impl Into<String>) -> Self {
        self.dataset_value = Some(dataset_value.into());
        self
    }
}

impl WorkflowGraphDefinition {
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_dataset_contract(mut self, dataset_contract: WorkflowDatasetContract) -> Self {
        self.dataset_contract = Some(dataset_contract);
        self
    }

    pub fn with_output_nodes(mut self, output_nodes: Vec<String>) -> Self {
        self.output_nodes = output_nodes;
        self
    }

    pub fn with_defaults(mut self, defaults: WorkflowDefaults) -> Self {
        self.defaults = Some(defaults);
        self
    }

    pub fn with_dispatch_policy(mut self, dispatch_policy: impl Into<String>) -> Self {
        self.dispatch_policy = Some(dispatch_policy.into());
        self
    }

    pub fn with_operator_fetch_plan(
        mut self,
        operator_fetch_plan: Vec<WorkflowOperatorFetchEntry>,
    ) -> Self {
        self.operator_fetch_plan = operator_fetch_plan;
        self
    }

    pub fn with_placement_tags(mut self, placement_tags: Vec<String>) -> Self {
        self.placement_tags = placement_tags;
        self
    }

    pub fn with_required_capabilities(mut self, required_capabilities: Vec<String>) -> Self {
        self.required_capabilities = required_capabilities;
        self
    }
}

impl WorkflowDefaults {
    pub fn with_cache_policy(mut self, cache_policy: impl Into<String>) -> Self {
        self.cache_policy = Some(cache_policy.into());
        self
    }

    pub fn with_orchestrated(mut self, orchestrated: bool) -> Self {
        self.orchestrated = Some(orchestrated);
        self
    }

    pub fn with_dispatch_policy(mut self, dispatch_policy: impl Into<String>) -> Self {
        self.dispatch_policy = Some(dispatch_policy.into());
        self
    }

    pub fn with_placement_tags(mut self, placement_tags: Vec<String>) -> Self {
        self.placement_tags = placement_tags;
        self
    }

    pub fn with_required_capabilities(mut self, required_capabilities: Vec<String>) -> Self {
        self.required_capabilities = required_capabilities;
        self
    }
}

impl WorkflowOperatorFetchEntry {
    pub fn with_package_ref(mut self, package_ref: impl Into<String>) -> Self {
        self.package_ref = Some(package_ref.into());
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_integrity(mut self, integrity: impl Into<String>) -> Self {
        self.integrity = Some(integrity.into());
        self
    }

    pub fn with_cache_scope(mut self, cache_scope: impl Into<String>) -> Self {
        self.cache_scope = Some(cache_scope.into());
        self
    }
}
