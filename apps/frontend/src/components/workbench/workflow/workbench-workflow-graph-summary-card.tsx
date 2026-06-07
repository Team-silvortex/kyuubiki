"use client";

import type { WorkflowGraphEdge, WorkflowGraphNode } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowGraphSummaryCardProps = {
  labels: WorkflowSidebarLabels;
  selectedNodes: WorkflowGraphNode[];
  selectedEdges: WorkflowGraphEdge[];
  selectedEntryInputsCount: number;
  selectedOutputArtifactsCount: number;
  focusedNodeId?: string | null;
  focusedEdgeId?: string | null;
};

export function WorkbenchWorkflowGraphSummaryCard({
  labels,
  selectedNodes,
  selectedEdges,
  selectedEntryInputsCount,
  selectedOutputArtifactsCount,
  focusedNodeId,
  focusedEdgeId,
}: WorkbenchWorkflowGraphSummaryCardProps) {
  return (
    <>
      <div className="sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.nodesTitle}</span>
          <strong>{selectedNodes.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.edgesTitle}</span>
          <strong>{selectedEdges.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.entryInputsTitle}</span>
          <strong>{selectedEntryInputsCount}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.outputArtifactsTitle}</span>
          <strong>{selectedOutputArtifactsCount}</strong>
        </div>
      </div>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.nodesTitle}</h2>
        </div>
        <div className="sidebar-list">
          {selectedNodes.map((node) => (
            <div
              className="sidebar-list__row"
              data-workflow-node-id={node.id}
              key={node.id}
              style={
                focusedNodeId === node.id
                  ? { outline: "2px solid var(--accent, #4f46e5)", outlineOffset: "2px" }
                  : undefined
              }
            >
              <span>{node.id}</span>
              <strong>
                {labels.kindLabel}: {node.kind}
                {node.operator_id ? ` · ${labels.operatorLabel}: ${node.operator_id}` : ""}
                {node.outputs?.some((port) => port.dataset_value)
                  ? ` · ${labels.datasetValueLabel}: ${node.outputs
                      .map((port) => port.dataset_value)
                      .filter(Boolean)
                      .join(", ")}`
                  : ""}
              </strong>
            </div>
          ))}
        </div>
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.edgesTitle}</h2>
        </div>
        <div className="sidebar-list">
          {selectedEdges.map((edge) => (
            <div
              className="sidebar-list__row"
              data-workflow-edge-id={edge.id}
              key={edge.id}
              style={
                focusedEdgeId === edge.id
                  ? { outline: "2px solid var(--accent, #4f46e5)", outlineOffset: "2px" }
                  : undefined
              }
            >
              <span>
                {edge.from.node}.{edge.from.port} → {edge.to.node}.{edge.to.port}
              </span>
              <strong>
                {edge.artifact_type}
                {edge.dataset_value ? ` · ${labels.datasetValueLabel}: ${edge.dataset_value}` : ""}
              </strong>
            </div>
          ))}
        </div>
      </section>
    </>
  );
}
