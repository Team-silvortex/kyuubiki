"use client";

type WorkflowFixGraphPort = {
  id: string;
  artifact_type: string;
  dataset_value?: string;
};

type WorkflowFixGraphNode = {
  id: string;
  config?: Record<string, unknown>;
  inputs?: WorkflowFixGraphPort[];
  outputs?: WorkflowFixGraphPort[];
};

type WorkflowFixGraphArtifact = {
  node_id: string;
  artifact_type: string;
};

type WorkflowFixGraphEdge = {
  id: string;
  artifact_type: string;
  dataset_value?: string;
};

type WorkflowFixGraph = {
  nodes: WorkflowFixGraphNode[];
  edges?: WorkflowFixGraphEdge[];
  entry_inputs?: WorkflowFixGraphArtifact[];
  output_artifacts?: WorkflowFixGraphArtifact[];
};

export type WorkflowGraphValidationFix =
  | { kind: "set_edge_artifact_type_from_source"; edgeId: string; artifactType: string }
  | { kind: "set_edge_artifact_type_from_target"; edgeId: string; artifactType: string }
  | {
      kind: "set_catalog_artifact_type";
      mode: "entry" | "output";
      nodeId: string;
      currentArtifactType: string;
      artifactType: string;
    }
  | {
      kind: "set_node_port_artifact_type_from_operator";
      nodeId: string;
      portId: string;
      direction: "inputs" | "outputs";
      artifactType: string;
    }
  | {
      kind: "set_node_port_dataset_value_from_operator";
      nodeId: string;
      portId: string;
      direction: "inputs" | "outputs";
      datasetValue?: string;
    }
  | { kind: "clear_port_dataset_value"; nodeId: string; portId: string; direction: "inputs" | "outputs" }
  | { kind: "clear_edge_dataset_value"; edgeId: string };

export function applyWorkflowGraphFix(
  graph: WorkflowFixGraph | null,
  fix: WorkflowGraphValidationFix | undefined,
) {
  if (!graph || !fix) return graph;
  const next = structuredClone(graph) as WorkflowFixGraph;

  switch (fix.kind) {
    case "set_edge_artifact_type_from_source":
    case "set_edge_artifact_type_from_target": {
      const edge = next.edges?.find((entry) => entry.id === fix.edgeId);
      if (edge) edge.artifact_type = fix.artifactType;
      break;
    }
    case "set_catalog_artifact_type": {
      const artifacts = next[fix.mode === "entry" ? "entry_inputs" : "output_artifacts"] ?? [];
      const artifact = artifacts.find(
        (entry) => entry.node_id === fix.nodeId && entry.artifact_type === fix.currentArtifactType,
      );
      if (artifact) artifact.artifact_type = fix.artifactType;
      break;
    }
    case "set_node_port_artifact_type_from_operator":
    case "set_node_port_dataset_value_from_operator":
    case "clear_port_dataset_value": {
      const node = next.nodes.find((entry) => entry.id === fix.nodeId);
      const port = node?.[fix.direction]?.find((entry) => entry.id === fix.portId);
      if (!port) break;
      if (fix.kind === "set_node_port_artifact_type_from_operator") port.artifact_type = fix.artifactType;
      else if (fix.kind === "set_node_port_dataset_value_from_operator") port.dataset_value = fix.datasetValue;
      else port.dataset_value = undefined;
      break;
    }
    case "clear_edge_dataset_value": {
      const edge = next.edges?.find((entry) => entry.id === fix.edgeId);
      if (edge) edge.dataset_value = undefined;
      break;
    }
  }

  return next;
}
