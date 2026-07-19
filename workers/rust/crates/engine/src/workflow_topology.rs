use kyuubiki_protocol::WorkflowGraph;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// Submission-time topology metrics for workflow scheduling and capacity checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTopologyProfile {
    pub node_count: usize,
    pub edge_count: usize,
    pub dependency_layers: usize,
    pub max_parallel_width: usize,
    pub entry_node_count: usize,
    pub output_node_count: usize,
}

/// Analyzes an acyclic workflow graph without executing operators.
pub fn analyze_workflow_topology(graph: &WorkflowGraph) -> Result<WorkflowTopologyProfile, String> {
    let mut indegree = HashMap::with_capacity(graph.nodes.len());
    let mut outgoing = HashMap::<String, Vec<String>>::with_capacity(graph.nodes.len());
    for node in &graph.nodes {
        if indegree.insert(node.id.clone(), 0_usize).is_some() {
            return Err(format!(
                "workflow topology has duplicate node id {}",
                node.id
            ));
        }
        outgoing.insert(node.id.clone(), Vec::new());
    }

    for edge in &graph.edges {
        if !indegree.contains_key(&edge.from.node) {
            return Err(format!(
                "workflow topology edge {} references missing source node {}",
                edge.id, edge.from.node
            ));
        }
        let Some(target_indegree) = indegree.get_mut(&edge.to.node) else {
            return Err(format!(
                "workflow topology edge {} references missing target node {}",
                edge.id, edge.to.node
            ));
        };
        *target_indegree += 1;
        outgoing
            .get_mut(&edge.from.node)
            .expect("source node was checked above")
            .push(edge.to.node.clone());
    }

    let mut frontier = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    let mut depths = frontier
        .iter()
        .map(|id| (id.clone(), 1_usize))
        .collect::<HashMap<_, _>>();
    let mut processed = 0_usize;
    let mut max_parallel_width = 0_usize;
    let mut dependency_layers = 0_usize;

    while !frontier.is_empty() {
        max_parallel_width = max_parallel_width.max(frontier.len());
        let current = std::mem::take(&mut frontier);
        for node_id in current {
            processed += 1;
            let node_depth = depths[&node_id];
            dependency_layers = dependency_layers.max(node_depth);
            for target in &outgoing[&node_id] {
                let target_depth = depths.entry(target.clone()).or_insert(0);
                *target_depth = (*target_depth).max(node_depth + 1);
                let target_indegree = indegree
                    .get_mut(target)
                    .expect("target node was checked above");
                *target_indegree -= 1;
                if *target_indegree == 0 {
                    frontier.insert(target.clone());
                }
            }
        }
    }

    if processed != graph.nodes.len() {
        return Err("workflow topology must be acyclic".to_string());
    }

    Ok(WorkflowTopologyProfile {
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        dependency_layers,
        max_parallel_width,
        entry_node_count: graph.entry_nodes.len(),
        output_node_count: graph.output_nodes.len(),
    })
}
