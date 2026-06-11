"use client";

import type { WorkflowCatalogEntryArtifact, WorkflowGraphDefinition, WorkflowOperatorDescriptor } from "@/lib/api";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import { getWorkflowNodeTemplateSyncImpact, listAutoReconnectEdgeIds } from "@/components/workbench/workflow/workbench-workflow-template-impact";

export type WorkflowValidationHighlightPlan = {
  nodeIds: string[];
  edgeIds: string[];
  artifactKeys: string[];
  datasetValueId: string | null;
  highlightDatasetEditor: boolean;
  firstNodeId: string | null;
  firstEdgeId: string | null;
  firstArtifactKey: string | null;
};

function pushUnique(items: string[], value: string | null | undefined) {
  if (value && !items.includes(value)) items.push(value);
}

function buildArtifactKey(
  mode: "entry" | "output",
  nodeId: string,
  artifactType: string,
  artifacts: WorkflowCatalogEntryArtifact[],
) {
  const index = artifacts.findIndex((artifact) => artifact.node_id === nodeId && artifact.artifact_type === artifactType);
  return index >= 0 ? `${mode}:${nodeId}:${artifactType}:${index}` : null;
}

export function buildWorkflowValidationHighlightPlan(
  graph: WorkflowGraphDefinition | null,
  issues: WorkflowGraphValidationIssue[],
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
): WorkflowValidationHighlightPlan {
  const nodeIds: string[] = [];
  const edgeIds: string[] = [];
  const artifactKeys: string[] = [];
  let datasetValueId: string | null = null;
  let highlightDatasetEditor = false;
  const entryInputs = graph?.entry_inputs ?? [];
  const outputArtifacts = graph?.output_artifacts ?? [];

  for (const issue of issues) {
    if (issue.locate?.kind === "node") pushUnique(nodeIds, issue.locate.nodeId);
    if (issue.locate?.kind === "edge") pushUnique(edgeIds, issue.locate.edgeId);
    if (issue.locate?.kind === "dataset") {
      highlightDatasetEditor = true;
      datasetValueId ??= issue.locate.datasetValueId ?? null;
    }
    if (issue.locate?.kind === "artifact") {
      const artifactType = issue.fix?.kind === "set_catalog_artifact_type" ? issue.fix.artifactType : issue.locate.artifactType;
      const artifactKey = buildArtifactKey(issue.locate.mode, issue.locate.nodeId, artifactType, issue.locate.mode === "entry" ? entryInputs : outputArtifacts);
      pushUnique(artifactKeys, artifactKey);
    }
    switch (issue.fix?.kind) {
      case "set_node_port_artifact_type_from_operator":
      case "set_node_port_dataset_value_from_operator":
      case "clear_port_dataset_value":
        pushUnique(nodeIds, issue.fix.nodeId);
        break;
      case "set_edge_artifact_type_from_source":
      case "set_edge_artifact_type_from_target":
      case "clear_edge_dataset_value":
        pushUnique(edgeIds, issue.fix.edgeId);
        break;
      case "set_catalog_artifact_type":
        pushUnique(artifactKeys, buildArtifactKey(issue.fix.mode, issue.fix.nodeId, issue.fix.artifactType, issue.fix.mode === "entry" ? entryInputs : outputArtifacts));
        break;
      case "sync_node_template_from_operator":
        pushUnique(nodeIds, issue.fix.nodeId);
        if (graph) {
          const impact = getWorkflowNodeTemplateSyncImpact(graph, issue.fix.nodeId, { kind: issue.fix.templateKind, operatorId: issue.fix.operatorId }, operatorDescriptors);
          for (const edgeId of listAutoReconnectEdgeIds(impact)) pushUnique(edgeIds, edgeId);
        }
        break;
    }
  }

  return {
    nodeIds,
    edgeIds,
    artifactKeys,
    datasetValueId,
    highlightDatasetEditor,
    firstNodeId: nodeIds[0] ?? null,
    firstEdgeId: edgeIds[0] ?? null,
    firstArtifactKey: artifactKeys[0] ?? null,
  };
}
