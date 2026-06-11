"use client";

import type { WorkflowGraphNode } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import {
  conditionOperatorNeedsValue,
  formatWorkflowConditionValue,
  isWorkflowConditionNode,
  parseWorkflowConditionValue,
  resolveWorkflowConditionConfig,
  WORKFLOW_CONDITION_OPERATORS,
} from "@/components/workbench/workflow/workbench-workflow-condition";

type WorkbenchWorkflowConditionEditorProps = {
  labels: WorkflowSidebarLabels;
  node: WorkflowGraphNode;
  onUpdateNode: (
    nodeId: string,
    updater: (node: WorkflowGraphNode) => WorkflowGraphNode,
  ) => void;
};

function updateConditionConfig(
  node: WorkflowGraphNode,
  onUpdateNode: WorkbenchWorkflowConditionEditorProps["onUpdateNode"],
  updater: (
    predicate: NonNullable<
      ReturnType<typeof resolveWorkflowConditionConfig>["predicate"]
    >,
  ) => void,
) {
  const nextConfig = resolveWorkflowConditionConfig(
    node.config as Record<string, unknown> | null | undefined,
  );
  updater(nextConfig.predicate ?? {});
  onUpdateNode(node.id, (current) => ({
    ...current,
    config: {
      ...(current.config ?? {}),
      predicate: nextConfig.predicate,
    },
  }));
}

export function WorkbenchWorkflowConditionEditor({
  labels,
  node,
  onUpdateNode,
}: WorkbenchWorkflowConditionEditorProps) {
  if (!isWorkflowConditionNode(node)) return null;
  const config = resolveWorkflowConditionConfig(
    node.config as Record<string, unknown> | null | undefined,
  );
  const predicate = config.predicate ?? {};
  const operator = predicate.operator ?? "gt";

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h3>{labels.validationTitle} Condition</h3>
        <span className="status-pill status-pill--watch">branch</span>
      </div>
      <div className="form-grid compact">
        <label>
          <span>Path</span>
          <input
            onChange={(event) =>
              updateConditionConfig(node, onUpdateNode, (next) => {
                next.path = event.target.value;
              })
            }
            placeholder="summary.max_displacement"
            value={predicate.path ?? ""}
          />
        </label>
        <label>
          <span>Operator</span>
          <select
            onChange={(event) =>
              updateConditionConfig(node, onUpdateNode, (next) => {
                next.operator = event.target.value as (typeof WORKFLOW_CONDITION_OPERATORS)[number];
              })
            }
            value={operator}
          >
            {WORKFLOW_CONDITION_OPERATORS.map((entry) => (
              <option key={entry} value={entry}>
                {entry}
              </option>
            ))}
          </select>
        </label>
        {conditionOperatorNeedsValue(operator) ? (
          <label>
            <span>Value</span>
            <input
              onChange={(event) =>
                updateConditionConfig(node, onUpdateNode, (next) => {
                  next.value = parseWorkflowConditionValue(event.target.value);
                })
              }
              placeholder='0, true, "warn"'
              value={formatWorkflowConditionValue(predicate.value)}
            />
          </label>
        ) : null}
      </div>
    </section>
  );
}
