"use client";

import type { WorkflowGraphEdge, WorkflowGraphNode } from "@/lib/api";
import { resolveWorkflowConditionConfig } from "@/components/workbench/workflow/workbench-workflow-condition";

type WorkbenchWorkflowControlFlowHintProps = {
  node: WorkflowGraphNode;
  selectedEdges: WorkflowGraphEdge[];
};

function listOutgoingTargets(
  nodeId: string,
  portId: string,
  selectedEdges: WorkflowGraphEdge[],
) {
  return selectedEdges
    .filter((edge) => edge.from.node === nodeId && edge.from.port === portId)
    .map((edge) => `${edge.to.node}.${edge.to.port}`);
}

function listIncomingSources(
  nodeId: string,
  portId: string,
  selectedEdges: WorkflowGraphEdge[],
) {
  return selectedEdges
    .filter((edge) => edge.to.node === nodeId && edge.to.port === portId)
    .map((edge) => `${edge.from.node}.${edge.from.port}`);
}

function formatTargets(targets: string[]) {
  return targets.length > 0 ? targets.join(", ") : "--";
}

function renderConditionHint(
  node: WorkflowGraphNode,
  selectedEdges: WorkflowGraphEdge[],
) {
  const predicate = resolveWorkflowConditionConfig(node.config ?? {}).predicate ?? {};
  const trueTargets = listOutgoingTargets(node.id, "if_true", selectedEdges);
  const falseTargets = listOutgoingTargets(node.id, "if_false", selectedEdges);
  return (
    <>
      <p className="card-copy">
        Predicate: <strong>{predicate.path ?? "--"}</strong> {predicate.operator ?? "gt"}{" "}
        {predicate.value === undefined ? "--" : JSON.stringify(predicate.value)}
      </p>
      <div className="card-copy" style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap", alignItems: "center" }}>
        <span className="status-pill status-pill--good">true</span>
        <span>{formatTargets(trueTargets)}</span>
      </div>
      <div className="card-copy" style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap", alignItems: "center" }}>
        <span className="status-pill status-pill--risk">false</span>
        <span>{formatTargets(falseTargets)}</span>
      </div>
    </>
  );
}

function renderMergeHint(node: WorkflowGraphNode, selectedEdges: WorkflowGraphEdge[]) {
  const leftSources = listIncomingSources(node.id, "left", selectedEdges);
  const rightSources = listIncomingSources(node.id, "right", selectedEdges);
  const mergedTargets = listOutgoingTargets(node.id, "merged", selectedEdges);
  return (
    <>
      <p className="card-copy">Merge behavior: forward the first branch artifact that actually arrives.</p>
      <div className="card-copy" style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap", alignItems: "center" }}>
        <span className="status-pill status-pill--watch">left</span>
        <span>{formatTargets(leftSources)}</span>
      </div>
      <div className="card-copy" style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap", alignItems: "center" }}>
        <span className="status-pill status-pill--watch">right</span>
        <span>{formatTargets(rightSources)}</span>
      </div>
      <div className="card-copy" style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap", alignItems: "center" }}>
        <span className="status-pill status-pill--good">merged</span>
        <span>{formatTargets(mergedTargets)}</span>
      </div>
    </>
  );
}

export function WorkbenchWorkflowControlFlowHint({
  node,
  selectedEdges,
}: WorkbenchWorkflowControlFlowHintProps) {
  if (node.kind === "condition") {
    return <section>{renderConditionHint(node, selectedEdges)}</section>;
  }
  if (node.operator_id === "transform.first_available") {
    return <section>{renderMergeHint(node, selectedEdges)}</section>;
  }
  return null;
}
