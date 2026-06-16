"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";

export type WorkflowGraphValidationIssue = {
  id: string;
  level: "warning";
  message: string;
  locate?:
    | { kind: "node"; nodeId: string }
    | { kind: "edge"; edgeId: string }
    | { kind: "dataset"; datasetValueId?: string }
    | { kind: "artifact"; mode: "entry" | "output"; nodeId: string; artifactType: string };
  fix?:
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
    | {
        kind: "sync_node_template_from_operator";
        nodeId: string;
        operatorId: string;
        templateKind?: string;
      }
    | { kind: "normalize_bridge_contract_from_support"; nodeId: string; operatorId: string }
    | { kind: "clear_port_dataset_value"; nodeId: string; portId: string; direction: "inputs" | "outputs" }
    | { kind: "clear_edge_dataset_value"; edgeId: string };
};

export type WorkflowValidationFixBatchResult = {
  graph: WorkflowGraphDefinition | null;
  appliedCount: number;
  appliedIssues: WorkflowGraphValidationIssue[];
};
