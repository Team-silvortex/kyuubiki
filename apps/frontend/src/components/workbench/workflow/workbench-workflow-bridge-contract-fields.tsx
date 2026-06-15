"use client";

import type { WorkflowGraphNode } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowBridgeContract, WorkflowBridgeContractSupport } from "@/lib/workbench/workflow-bridge-contract";
import { applyBridgeDistributionDefaults } from "@/lib/workbench/workflow-bridge-contract-support";

type WorkbenchWorkflowBridgeContractFieldsProps = {
  labels: WorkflowSidebarLabels;
  node: WorkflowGraphNode;
  contract: WorkflowBridgeContract;
  contractSupport?: WorkflowBridgeContractSupport | null;
  sourceFieldOptions: string[];
  distributionOptions: string[];
  nodeIndexFieldOptions: string[];
  reductionOptions: string[];
  targetFieldOptions: string[];
  onUpdateNode: (nodeId: string, updater: (node: WorkflowGraphNode) => WorkflowGraphNode) => void;
  updateBridgeContractField: (
    node: WorkflowGraphNode,
    updater: (contract: WorkflowBridgeContract) => void,
  ) => WorkflowGraphNode;
};

export function WorkbenchWorkflowBridgeContractFields({
  labels,
  node,
  contract,
  contractSupport,
  sourceFieldOptions,
  distributionOptions,
  nodeIndexFieldOptions,
  reductionOptions,
  targetFieldOptions,
  onUpdateNode,
  updateBridgeContractField,
}: WorkbenchWorkflowBridgeContractFieldsProps) {
  return (
    <div className="form-grid compact">
      <label>
        <span>{labels.bridgeContractSourceFieldLabel}</span>
        {sourceFieldOptions.length > 0 ? (
          <select
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.source.field = event.target.value;
                }),
              )
            }
            value={contract.source.field}
          >
            {sourceFieldOptions.map((value) => (
              <option key={value} value={value}>{value}</option>
            ))}
          </select>
        ) : (
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
        )}
      </label>
      <label>
        <span>{labels.bridgeContractDistributionLabel}</span>
        {distributionOptions.length > 0 ? (
          <select
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  Object.assign(nextContract, applyBridgeDistributionDefaults(nextContract, event.target.value, contractSupport));
                }),
              )
            }
            value={contract.source.distribution}
          >
            {distributionOptions.map((value) => (
              <option key={value} value={value}>{value}</option>
            ))}
          </select>
        ) : (
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
        )}
      </label>
      <label className="field-span-2">
        <span>{labels.bridgeContractNodeIndexFieldsLabel}</span>
        {nodeIndexFieldOptions.length > 0 ? (
          <select
            multiple
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.source.node_index_fields = Array.from(event.target.selectedOptions).map((option) => option.value);
                }),
              )
            }
            value={contract.source.node_index_fields}
          >
            {nodeIndexFieldOptions.map((value) => (
              <option key={value} value={value}>{value}</option>
            ))}
          </select>
        ) : (
          <input
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.source.node_index_fields = event.target.value.split(",").map((value) => value.trim()).filter(Boolean);
                }),
              )
            }
            value={contract.source.node_index_fields.join(", ")}
          />
        )}
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
        {reductionOptions.length > 0 ? (
          <select
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.transform.reduction = event.target.value;
                }),
              )
            }
            value={contract.transform.reduction}
          >
            {reductionOptions.map((value) => (
              <option key={value} value={value}>{value}</option>
            ))}
          </select>
        ) : (
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
        )}
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
        {targetFieldOptions.length > 0 ? (
          <select
            onChange={(event) =>
              onUpdateNode(node.id, (current) =>
                updateBridgeContractField(current, (nextContract) => {
                  nextContract.target.field = event.target.value;
                }),
              )
            }
            value={contract.target.field}
          >
            {targetFieldOptions.map((value) => (
              <option key={value} value={value}>{value}</option>
            ))}
          </select>
        ) : (
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
        )}
      </label>
    </div>
  );
}
