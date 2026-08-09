use kyuubiki_protocol::{WorkflowEdge, WorkflowGraph};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(crate) struct WorkflowExecutionPlan<'a> {
    node_order: Vec<usize>,
    incoming_by_node: Vec<Vec<WorkflowPlannedEdge<'a>>>,
}

pub(crate) struct WorkflowPlannedEdge<'a> {
    edge: &'a WorkflowEdge,
    source_artifact_key: String,
}

pub(crate) struct WorkflowIncomingState {
    pub resolved_artifact_keys: Vec<String>,
    pub all_resolved: bool,
    pub any_resolved: bool,
}

impl<'a> WorkflowExecutionPlan<'a> {
    pub fn compile(graph: &'a WorkflowGraph) -> Result<Self, String> {
        let mut node_indices = HashMap::with_capacity(graph.nodes.len());
        for (index, node) in graph.nodes.iter().enumerate() {
            if node_indices.insert(node.id.as_str(), index).is_some() {
                return Err(format!(
                    "workflow execution plan has duplicate node id {}",
                    node.id
                ));
            }
        }

        let mut incoming_by_node = std::iter::repeat_with(Vec::new)
            .take(graph.nodes.len())
            .collect::<Vec<_>>();
        let mut outgoing_by_node = vec![Vec::new(); graph.nodes.len()];
        let mut indegree = vec![0_usize; graph.nodes.len()];
        for edge in &graph.edges {
            let source = node_indices.get(edge.from.node.as_str()).ok_or_else(|| {
                format!(
                    "workflow execution plan edge {} has unknown source {}",
                    edge.id, edge.from.node
                )
            })?;
            let target = node_indices.get(edge.to.node.as_str()).ok_or_else(|| {
                format!(
                    "workflow execution plan edge {} has unknown target {}",
                    edge.id, edge.to.node
                )
            })?;
            incoming_by_node[*target].push(WorkflowPlannedEdge {
                edge,
                source_artifact_key: format!("{}.{}", edge.from.node, edge.from.port),
            });
            outgoing_by_node[*source].push(*target);
            indegree[*target] += 1;
        }

        let mut ready = indegree
            .iter()
            .enumerate()
            .filter_map(|(index, count)| (*count == 0).then_some(index))
            .collect::<BTreeSet<_>>();
        let mut node_order = Vec::with_capacity(graph.nodes.len());
        while let Some(index) = ready.pop_first() {
            node_order.push(index);
            for target in &outgoing_by_node[index] {
                indegree[*target] -= 1;
                if indegree[*target] == 0 {
                    ready.insert(*target);
                }
            }
        }
        if node_order.len() != graph.nodes.len() {
            return Err("workflow execution plan requires an acyclic graph".to_string());
        }
        Ok(Self {
            node_order,
            incoming_by_node,
        })
    }

    pub fn node_order(&self) -> &[usize] {
        &self.node_order
    }

    pub fn incoming(&self, node_index: usize) -> &[WorkflowPlannedEdge<'a>] {
        &self.incoming_by_node[node_index]
    }

    pub fn resolve_incoming(
        &self,
        node_index: usize,
        artifacts: &BTreeMap<String, Value>,
    ) -> WorkflowIncomingState {
        let incoming = self.incoming(node_index);
        let mut resolved_artifact_keys = Vec::with_capacity(incoming.len());
        for edge in incoming {
            if artifacts.contains_key(edge.source_key()) {
                resolved_artifact_keys.push(edge.source_key().to_owned());
            }
        }
        WorkflowIncomingState {
            all_resolved: resolved_artifact_keys.len() == incoming.len(),
            any_resolved: !resolved_artifact_keys.is_empty(),
            resolved_artifact_keys,
        }
    }
}

impl WorkflowPlannedEdge<'_> {
    pub fn edge(&self) -> &WorkflowEdge {
        self.edge
    }

    pub fn source_key(&self) -> &str {
        &self.source_artifact_key
    }
}
