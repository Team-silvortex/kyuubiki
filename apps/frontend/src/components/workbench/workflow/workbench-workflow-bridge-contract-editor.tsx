"use client";

import type { WorkflowGraphNode } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import {
  isWorkflowBridgeContractOperator,
  resolveBridgeContractForOperator,
} from "@/components/workbench/workflow/workbench-workflow-bridge-contract";

type WorkbenchWorkflowBridgeContractEditorProps = {
  labels: WorkflowSidebarLabels;
  node: WorkflowGraphNode;
  selectedNodes?: WorkflowGraphNode[];
  onUpdateNode: (nodeId: string, updater: (node: WorkflowGraphNode) => WorkflowGraphNode) => void;
};

function updateBridgeContractField(
  node: WorkflowGraphNode,
  updater: (contract: NonNullable<ReturnType<typeof resolveBridgeContractForOperator>>) => void,
) {
  const contract = resolveBridgeContractForOperator(
    node.operator_id,
    node.config as Record<string, unknown> | null | undefined,
  );
  if (!contract) return node;
  const nextContract = JSON.parse(JSON.stringify(contract)) as typeof contract;
  updater(nextContract);
  return {
    ...node,
    config: {
      ...(node.config ?? {}),
      contract: nextContract,
    },
  };
}

export function WorkbenchWorkflowBridgeContractEditor({
  labels,
  node,
  selectedNodes = [],
  onUpdateNode,
}: WorkbenchWorkflowBridgeContractEditorProps) {
  if (!isWorkflowBridgeContractOperator(node.operator_id)) return null;
  const contract = resolveBridgeContractForOperator(
    node.operator_id,
    node.config as Record<string, unknown> | null | undefined,
  );
  if (!contract) return null;
  const downstreamEdges = selectedNodes.filter((entry) =>
    (entry.inputs ?? []).some((port) =>
      (node.outputs ?? []).some((output) => output.artifact_type === port.artifact_type),
    ),
  );
  const previewSummary =
    contract.source.distribution === "node_to_node"
      ? `${contract.source.field} × ${contract.transform.scale} -> ${contract.target.field}`
      : `${contract.source.field} -> ${contract.target.field} (${contract.source.distribution}, ${contract.transform.reduction}, scale ${contract.transform.scale})`;

  return (
    <div className="sidebar-stack">
      <div className="card-head">
        <h3>{labels.bridgeContractTitle}</h3>
        <span className="status-pill status-pill--watch">contract</span>
      </div>
      <div className="form-grid compact">
        <label>
          <span>{labels.bridgeContractSourceFieldLabel}</span>
          <input
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.source.field = event.target.value;
                }),
              )
            }
            value={contract.source.field}
          />
        </label>
        <label>
          <span>{labels.bridgeContractDistributionLabel}</span>
          <input
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.source.distribution = event.target.value;
                }),
              )
            }
            value={contract.source.distribution}
          />
        </label>
        <label className="field-span-2">
          <span>{labels.bridgeContractNodeIndexFieldsLabel}</span>
          <input
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.source.node_index_fields = event.target.value
                    .split(",")
                    .map((value) => value.trim())
                    .filter(Boolean);
                }),
              )
            }
            value={contract.source.node_index_fields.join(", ")}
          />
        </label>
        <label>
          <span>{labels.bridgeContractScaleLabel}</span>
          <input
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.transform.scale = Number(event.target.value) || 0;
                }),
              )
            }
            type="number"
            value={contract.transform.scale}
          />
        </label>
        <label>
          <span>{labels.bridgeContractReductionLabel}</span>
          <input
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.transform.reduction = event.target.value;
                }),
              )
            }
            value={contract.transform.reduction}
          />
        </label>
        <label>
          <span>{labels.bridgeContractDefaultValueLabel}</span>
          <input
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.transform.default_value = Number(event.target.value) || 0;
                }),
              )
            }
            type="number"
            value={contract.transform.default_value}
          />
        </label>
        <label>
          <span>{labels.bridgeContractTargetFieldLabel}</span>
          <input
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.target.field = event.target.value;
                }),
              )
            }
            value={contract.target.field}
          />
        </label>
      </div>
      <div className="sidebar-stack">
        <div className="card-head">
          <h3>{labels.bridgeContractPreviewTitle}</h3>
          <span className="status-pill status-pill--watch">live</span>
        </div>
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{labels.bridgeContractPreviewSummaryLabel}</span>
            <strong>{previewSummary}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{labels.bridgeContractPreviewDownstreamLabel}</span>
            <strong>
              {downstreamEdges.length > 0
                ? downstreamEdges
                    .map((entry) => entry.operator_id || entry.id)
                    .join(", ")
                : "--"}
            </strong>
          </div>
        </div>
      </div>
    </div>
  );
}
