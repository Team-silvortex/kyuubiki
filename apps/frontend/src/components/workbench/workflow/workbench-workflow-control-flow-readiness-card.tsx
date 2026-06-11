"use client";

import type { WorkflowGraphEdge, WorkflowGraphNode } from "@/lib/api";
import { isWorkflowNodeSupportedInRuntime } from "@/components/workbench/workflow/workbench-workflow-runtime-support";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowControlFlowReadinessCardProps = {
  labels: WorkflowSidebarLabels;
  selectedNodes: WorkflowGraphNode[];
  selectedEdges: WorkflowGraphEdge[];
  validationIssues: WorkflowGraphValidationIssue[];
  invalidInputCount: number;
};

function countConnected(nodeId: string, portId: string, edges: WorkflowGraphEdge[], side: "from" | "to") {
  return edges.filter((edge) => edge[side].node === nodeId && edge[side].port === portId).length;
}

export function WorkbenchWorkflowControlFlowReadinessCard({
  labels,
  selectedNodes,
  selectedEdges,
  validationIssues,
  invalidInputCount,
}: WorkbenchWorkflowControlFlowReadinessCardProps) {
  const unsupportedNodes = selectedNodes.filter((node) => !isWorkflowNodeSupportedInRuntime(node));
  const conditionNodes = selectedNodes.filter((node) => node.kind === "condition");
  const mergeNodes = selectedNodes.filter((node) => node.operator_id === "transform.first_available");
  const wiredConditions = conditionNodes.filter((node) => countConnected(node.id, "if_true", selectedEdges, "from") > 0 && countConnected(node.id, "if_false", selectedEdges, "from") > 0).length;
  const wiredMerges = mergeNodes.filter((node) => countConnected(node.id, "left", selectedEdges, "to") > 0 && countConnected(node.id, "right", selectedEdges, "to") > 0 && countConnected(node.id, "merged", selectedEdges, "from") > 0).length;
  const blockingMessages = [
    ...unsupportedNodes.map((node) => `unsupported runtime node: ${node.id}`),
    ...validationIssues.slice(0, 3).map((issue) => issue.message),
    ...(invalidInputCount > 0 ? [`${invalidInputCount} invalid input artifact draft(s)`] : []),
  ].slice(0, 4);
  const ready = unsupportedNodes.length === 0 && validationIssues.length === 0 && invalidInputCount === 0;

  return (
    <section className="sidebar-card sidebar-card--compact runtime-overview-card">
      <div className="card-head">
        <h2>Workflow readiness</h2>
        <span className={`status-pill status-pill--${ready ? "good" : "risk"}`}>{ready ? labels.statusReadyLabel : labels.statusBusyLabel}</span>
      </div>
      <div className="sidebar-list">
        <div className="sidebar-list__row"><span>runtime-supported nodes</span><strong>{selectedNodes.length - unsupportedNodes.length}/{selectedNodes.length}</strong></div>
        <div className="sidebar-list__row"><span>condition branches wired</span><strong>{wiredConditions}/{conditionNodes.length}</strong></div>
        <div className="sidebar-list__row"><span>merge lanes wired</span><strong>{wiredMerges}/{mergeNodes.length}</strong></div>
        <div className="sidebar-list__row"><span>blocking issues</span><strong>{validationIssues.length + invalidInputCount}</strong></div>
      </div>
      {blockingMessages.length > 0 ? (
        <div style={{ display: "grid", gap: "0.35rem", marginTop: "0.75rem" }}>
          {blockingMessages.map((message) => (
            <span className="card-copy" key={message}>{message}</span>
          ))}
        </div>
      ) : (
        <p className="card-copy" style={{ marginTop: "0.75rem" }}>
          Control-flow graph is aligned with the current runtime executor.
        </p>
      )}
    </section>
  );
}
