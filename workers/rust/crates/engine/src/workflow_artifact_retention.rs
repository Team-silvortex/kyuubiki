use crate::workflow_execution_plan::WorkflowPlannedEdge;
use crate::workflow_executor::artifact_key;
use kyuubiki_protocol::{
    WorkflowArtifactProjection, WorkflowCachePolicy, WorkflowGraph, WorkflowNode, WorkflowNodeKind,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) struct WorkflowArtifactRetention {
    remaining_consumers: HashMap<String, usize>,
    transient_artifacts: HashSet<String>,
}

impl WorkflowArtifactRetention {
    pub fn compile(graph: &WorkflowGraph, projection: WorkflowArtifactProjection) -> Self {
        let default_policy = graph
            .defaults
            .cache_policy
            .unwrap_or(WorkflowCachePolicy::Cached);
        let mut transient_artifacts = graph
            .nodes
            .iter()
            .filter(|node| is_transient(node, default_policy, projection))
            .flat_map(|node| {
                node.outputs
                    .iter()
                    .map(|port| artifact_key(&node.id, &port.id))
            })
            .collect();
        collect_transient_output_artifacts(graph, projection, &mut transient_artifacts);
        let mut remaining_consumers = HashMap::with_capacity(graph.edges.len());
        for edge in &graph.edges {
            *remaining_consumers
                .entry(artifact_key(&edge.from.node, &edge.from.port))
                .or_insert(0) += 1;
        }
        Self {
            remaining_consumers,
            transient_artifacts,
        }
    }

    pub fn take_if_last_transient(
        &self,
        artifact_key: &str,
        artifacts: &mut BTreeMap<String, Value>,
    ) -> Option<Value> {
        (self.transient_artifacts.contains(artifact_key)
            && self.remaining_consumers.get(artifact_key) == Some(&1))
        .then(|| artifacts.remove(artifact_key))
        .flatten()
    }

    pub fn finish_node(
        &mut self,
        incoming: &[WorkflowPlannedEdge<'_>],
        produced_artifacts: &[String],
    ) -> Vec<String> {
        let mut releasable = Vec::new();
        for edge in incoming {
            let key = edge.source_key();
            if let Some(remaining) = self.remaining_consumers.get_mut(key) {
                *remaining = remaining.saturating_sub(1);
                if *remaining == 0 && self.transient_artifacts.contains(key) {
                    releasable.push(key.to_owned());
                }
            }
        }
        for key in produced_artifacts {
            if self.transient_artifacts.contains(key)
                && self.remaining_consumers.get(key).copied().unwrap_or(0) == 0
            {
                releasable.push(key.clone());
            }
        }
        releasable
    }
}

fn is_transient(
    node: &WorkflowNode,
    default_policy: WorkflowCachePolicy,
    projection: WorkflowArtifactProjection,
) -> bool {
    match projection {
        WorkflowArtifactProjection::All => {
            node.kind != WorkflowNodeKind::Output
                && node.cache_policy.unwrap_or(default_policy) == WorkflowCachePolicy::Ephemeral
        }
        WorkflowArtifactProjection::Outputs | WorkflowArtifactProjection::None => true,
    }
}

fn collect_transient_output_artifacts(
    graph: &WorkflowGraph,
    projection: WorkflowArtifactProjection,
    transient_artifacts: &mut HashSet<String>,
) {
    if projection == WorkflowArtifactProjection::All {
        return;
    }
    let declared_outputs = graph
        .output_nodes
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let output_nodes = graph
        .nodes
        .iter()
        .filter(|node| node.kind == WorkflowNodeKind::Output)
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    for edge in &graph.edges {
        if !output_nodes.contains(edge.to.node.as_str()) {
            continue;
        }
        let retain = projection == WorkflowArtifactProjection::Outputs
            && declared_outputs.contains(edge.to.node.as_str());
        if !retain {
            transient_artifacts.insert(artifact_key(&edge.to.node, &edge.to.port));
        }
    }
}
