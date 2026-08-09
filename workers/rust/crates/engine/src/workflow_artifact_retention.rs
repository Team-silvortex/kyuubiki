use crate::workflow_execution_plan::WorkflowPlannedEdge;
use crate::workflow_executor::artifact_key;
use kyuubiki_protocol::{WorkflowCachePolicy, WorkflowGraph, WorkflowNode, WorkflowNodeKind};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) struct WorkflowArtifactRetention {
    remaining_consumers: HashMap<String, usize>,
    ephemeral_artifacts: HashSet<String>,
}

impl WorkflowArtifactRetention {
    pub fn compile(graph: &WorkflowGraph) -> Self {
        let default_policy = graph
            .defaults
            .cache_policy
            .unwrap_or(WorkflowCachePolicy::Cached);
        let ephemeral_artifacts = graph
            .nodes
            .iter()
            .filter(|node| is_ephemeral(node, default_policy))
            .flat_map(|node| {
                node.outputs
                    .iter()
                    .map(|port| artifact_key(&node.id, &port.id))
            })
            .collect();
        let mut remaining_consumers = HashMap::with_capacity(graph.edges.len());
        for edge in &graph.edges {
            *remaining_consumers
                .entry(artifact_key(&edge.from.node, &edge.from.port))
                .or_insert(0) += 1;
        }
        Self {
            remaining_consumers,
            ephemeral_artifacts,
        }
    }

    pub fn take_if_last_ephemeral(
        &self,
        artifact_key: &str,
        artifacts: &mut BTreeMap<String, Value>,
    ) -> Option<Value> {
        (self.ephemeral_artifacts.contains(artifact_key)
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
                if *remaining == 0 && self.ephemeral_artifacts.contains(key) {
                    releasable.push(key.to_owned());
                }
            }
        }
        for key in produced_artifacts {
            if self.ephemeral_artifacts.contains(key)
                && self.remaining_consumers.get(key).copied().unwrap_or(0) == 0
            {
                releasable.push(key.clone());
            }
        }
        releasable
    }
}

fn is_ephemeral(node: &WorkflowNode, default_policy: WorkflowCachePolicy) -> bool {
    node.kind != WorkflowNodeKind::Output
        && node.cache_policy.unwrap_or(default_policy) == WorkflowCachePolicy::Ephemeral
}
