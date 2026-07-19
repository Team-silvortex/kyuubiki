"use client";

import { useEffect, useState } from "react";
import {
  resolveHeatPlaneTriangle2dJobInput,
  resolveHeatPlaneQuad2dJobInput,
  resolveThermalPlaneTriangle2dJobInput,
  resolveThermalPlaneQuad2dJobInput,
  type HeatPlaneTriangle2dJobInput,
  type HeatPlaneQuad2dJobInput,
  type ThermalPlaneTriangle2dJobInput,
  type ThermalPlaneQuad2dJobInput,
  type WorkflowGraphNode,
  type WorkflowOperatorDescriptor,
} from "@/lib/api";
import type { StudyKind } from "@/components/workbench/workbench-types";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import { WorkbenchWorkflowBridgeContractFields } from "@/components/workbench/workflow/workbench-workflow-bridge-contract-fields";
import {
  isWorkflowBridgeContractOperator,
  normalizeBridgeConfigForOperator,
  resolveBridgeContractForOperator,
  resolveBridgeSeedModelForOperator,
} from "@/lib/workbench/workflow-bridge-contract";
import {
  resolveBridgeContractFieldOptions,
  type WorkflowBridgeContractNormalizationAdjustment,
  normalizeBridgeConfigWithSupport,
} from "@/lib/workbench/workflow-bridge-contract-support";

type WorkbenchWorkflowBridgeContractEditorProps = {
  labels: WorkflowSidebarLabels;
  node: WorkflowGraphNode;
  operatorDescriptor?: WorkflowOperatorDescriptor;
  selectedNodes?: WorkflowGraphNode[];
  currentStudyKind: StudyKind;
  currentHeatPlaneModel: Record<string, unknown>;
  currentPlaneModel: Record<string, unknown>;
  onUpdateNode: (nodeId: string, updater: (node: WorkflowGraphNode) => WorkflowGraphNode) => void;
};
type SeedModelRecord = Record<string, unknown>;
function asObjectRecord(value: unknown): SeedModelRecord | null {
  return typeof value === "object" && value !== null ? (value as SeedModelRecord) : null;
}
function asObjectArray(value: unknown) {
  return Array.isArray(value) ? value.map((entry) => asObjectRecord(entry)).filter(Boolean) as SeedModelRecord[] : [];
}
function updateBridgeContractField(
  node: WorkflowGraphNode,
  operatorDescriptor: WorkflowOperatorDescriptor | undefined,
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
    config:
      normalizeBridgeConfigWithSupport(
        node.operator_id,
        {
          ...(node.config ?? {}),
          contract: nextContract,
        },
        operatorDescriptor,
      ) ?? {
        ...(node.config ?? {}),
        contract: nextContract,
      },
  };
}

function updateBridgeSeedModel(node: WorkflowGraphNode, nextSeedModel: Record<string, unknown>) {
  if (
    node.operator_id === "bridge.electrostatic_field_to_heat_quad_2d" ||
    node.operator_id === "bridge.electrostatic_field_to_heat_triangle_2d"
  ) {
    return {
      ...node,
      config: {
        ...(node.config ?? {}),
        seed_model: nextSeedModel,
      },
    };
  }
  if (
    node.operator_id === "bridge.temperature_field_to_thermo_quad_2d" ||
    node.operator_id === "bridge.temperature_field_to_thermo_triangle_2d"
  ) {
    return {
      ...node,
      config: normalizeBridgeConfigForOperator(
        node.operator_id,
        {
          ...(typeof node.config === "object" && node.config !== null ? node.config : {}),
          seed_model: nextSeedModel,
        },
      ) ?? {
        seed_model: nextSeedModel,
      },
    };
  }
  return node;
}
function summarizeSeedModel(seedModel: Record<string, unknown> | null) {
  const nodes = Array.isArray(seedModel?.nodes) ? seedModel.nodes.length : 0;
  const elements = Array.isArray(seedModel?.elements) ? seedModel.elements.length : 0;
  return { nodes, elements };
}
function describeSeedModelTarget(operatorId?: string | null) {
  if (operatorId === "bridge.electrostatic_field_to_heat_quad_2d") return "heat plane quad";
  if (operatorId === "bridge.electrostatic_field_to_heat_triangle_2d") return "heat plane triangle";
  if (operatorId === "bridge.temperature_field_to_thermo_triangle_2d") return "thermal plane triangle";
  return "thermal plane quad";
}

function importableBridgeWorkspaceStudyKind(operatorId?: string | null) {
  if (operatorId === "bridge.electrostatic_field_to_heat_quad_2d") return "heat_plane_quad_2d";
  if (operatorId === "bridge.electrostatic_field_to_heat_triangle_2d") return "heat_plane_triangle_2d";
  if (operatorId === "bridge.temperature_field_to_thermo_triangle_2d") return "thermal_plane_triangle_2d";
  return operatorId === "bridge.temperature_field_to_thermo_quad_2d" ? "thermal_plane_quad_2d" : null;
}
function readBridgeContractNormalizationAdjustments(node: WorkflowGraphNode) {
  const value = node.config?.contract_normalization;
  if (!Array.isArray(value)) return [] as WorkflowBridgeContractNormalizationAdjustment[];
  return value.filter((entry): entry is WorkflowBridgeContractNormalizationAdjustment => (
    typeof entry === "object" &&
    entry !== null &&
    typeof (entry as { field?: unknown }).field === "string" &&
    typeof (entry as { previous?: unknown }).previous === "string" &&
    typeof (entry as { next?: unknown }).next === "string"
  ));
}
function updateSeedModelCollectionField(node: WorkflowGraphNode, collection: "nodes" | "elements", index: number, field: string, value: unknown) {
  const seedModel = resolveBridgeSeedModelForOperator(
    node.operator_id,
    node.config as Record<string, unknown> | null | undefined,
  );
  if (!seedModel) return node;
  const nextSeedModel = structuredClone(seedModel) as SeedModelRecord;
  const rows = asObjectArray(nextSeedModel[collection]);
  const row = rows[index];
  if (!row) return node;
  row[field] = value;
  nextSeedModel[collection] = rows;
  return updateBridgeSeedModel(node, nextSeedModel);
}

export function WorkbenchWorkflowBridgeContractEditor({
  labels,
  node,
  operatorDescriptor,
  selectedNodes = [],
  currentStudyKind,
  currentHeatPlaneModel,
  currentPlaneModel,
  onUpdateNode,
}: WorkbenchWorkflowBridgeContractEditorProps) {
  if (!isWorkflowBridgeContractOperator(node.operator_id)) return null;
  const contract = resolveBridgeContractForOperator(
    node.operator_id,
    node.config as Record<string, unknown> | null | undefined,
  );
  if (!contract) return null;
  const contractSupport = operatorDescriptor?.contract_support;
  const contractNormalizationAdjustments = readBridgeContractNormalizationAdjustments(node);
  const {
    distributionOptions,
    sourceFieldOptions,
    reductionOptions,
    targetFieldOptions,
    nodeIndexFieldOptions,
  } = resolveBridgeContractFieldOptions(contract, contractSupport);
  const resolvedSeedModel = resolveBridgeSeedModelForOperator(
    node.operator_id,
    node.config as Record<string, unknown> | null | undefined,
  );
  const [seedModelDraft, setSeedModelDraft] = useState(() =>
    JSON.stringify(resolvedSeedModel ?? {}, null, 2),
  );
  const [seedModelError, setSeedModelError] = useState<string | null>(null);
  useEffect(() => {
    setSeedModelDraft(JSON.stringify(resolvedSeedModel ?? {}, null, 2));
    setSeedModelError(null);
  }, [node.id, node.operator_id, JSON.stringify(resolvedSeedModel ?? {})]);
  const downstreamEdges = selectedNodes.filter((entry) =>
    (entry.inputs ?? []).some((port) =>
      (node.outputs ?? []).some((output) => output.artifact_type === port.artifact_type),
    ),
  );
  const seedModelSummary = summarizeSeedModel(resolvedSeedModel as Record<string, unknown> | null);
  const canImportCurrentWorkspaceModel =
    importableBridgeWorkspaceStudyKind(node.operator_id) === currentStudyKind;
  const seedModelNodes = asObjectArray(resolvedSeedModel?.nodes);
  const seedModelElements = asObjectArray(resolvedSeedModel?.elements);
  const previewSummary =
    contract.source.distribution === "node_to_node"
      ? `${contract.source.field} × ${contract.transform.scale} -> ${contract.target.field}`
      : `${contract.source.field} -> ${contract.target.field} (${contract.source.distribution}, ${contract.transform.reduction}, scale ${contract.transform.scale})`;
  const normalizationFieldLabels: Record<WorkflowBridgeContractNormalizationAdjustment["field"], string> = {
    "source.field": labels.bridgeContractSourceFieldLabel,
    "source.distribution": labels.bridgeContractDistributionLabel,
    "source.node_index_fields": labels.bridgeContractNodeIndexFieldsLabel,
    "transform.reduction": labels.bridgeContractReductionLabel,
    "target.field": labels.bridgeContractTargetFieldLabel,
  };
  function applySeedModelDraft() {
    try {
      const parsed = JSON.parse(seedModelDraft) as Record<string, unknown>;
      setSeedModelError(null);
      onUpdateNode(node.id, (current) => updateBridgeSeedModel(current, parsed));
    } catch {
      setSeedModelError(labels.bridgeSeedModelInvalidLabel);
    }
  }
  function resetSeedModelDraft() {
    setSeedModelDraft(JSON.stringify(resolvedSeedModel ?? {}, null, 2));
    setSeedModelError(null);
  }
  function importCurrentWorkspaceModel() {
    if (node.operator_id === "bridge.electrostatic_field_to_heat_quad_2d" && currentStudyKind === "heat_plane_quad_2d") {
      const nextSeedModel = resolveHeatPlaneQuad2dJobInput(
        currentHeatPlaneModel as HeatPlaneQuad2dJobInput,
      ) as unknown as Record<string, unknown>;
      onUpdateNode(node.id, (current) => updateBridgeSeedModel(current, nextSeedModel));
      return;
    }
    if (node.operator_id === "bridge.electrostatic_field_to_heat_triangle_2d" && currentStudyKind === "heat_plane_triangle_2d") {
      const nextSeedModel = resolveHeatPlaneTriangle2dJobInput(
        currentHeatPlaneModel as HeatPlaneTriangle2dJobInput,
      ) as unknown as Record<string, unknown>;
      onUpdateNode(node.id, (current) => updateBridgeSeedModel(current, nextSeedModel));
      return;
    }
    if (node.operator_id === "bridge.temperature_field_to_thermo_quad_2d" && currentStudyKind === "thermal_plane_quad_2d") {
      const nextSeedModel = resolveThermalPlaneQuad2dJobInput(
        currentPlaneModel as ThermalPlaneQuad2dJobInput,
      ) as unknown as Record<string, unknown>;
      onUpdateNode(node.id, (current) => updateBridgeSeedModel(current, nextSeedModel));
      return;
    }
    if (node.operator_id === "bridge.temperature_field_to_thermo_triangle_2d" && currentStudyKind === "thermal_plane_triangle_2d") {
      const nextSeedModel = resolveThermalPlaneTriangle2dJobInput(
        currentPlaneModel as ThermalPlaneTriangle2dJobInput,
      ) as unknown as Record<string, unknown>;
      onUpdateNode(node.id, (current) => updateBridgeSeedModel(current, nextSeedModel));
    }
  }

  return (
    <div className="sidebar-stack">
      <div className="card-head">
        <h3>{labels.bridgeContractTitle}</h3>
        <span className="status-pill status-pill--watch">contract</span>
      </div>
      <WorkbenchWorkflowBridgeContractFields
        contract={contract}
        contractSupport={contractSupport}
        distributionOptions={distributionOptions}
        labels={labels}
        node={node}
        nodeIndexFieldOptions={nodeIndexFieldOptions}
        onUpdateNode={onUpdateNode}
        reductionOptions={reductionOptions}
        sourceFieldOptions={sourceFieldOptions}
        targetFieldOptions={targetFieldOptions}
        updateBridgeContractField={(currentNode, updater) =>
          updateBridgeContractField(currentNode, operatorDescriptor, updater)
        }
      />
      {contractNormalizationAdjustments.length > 0 ? (
        <div className="sidebar-stack">
          <div className="card-head">
            <h3>{labels.bridgeContractAdjustedFieldsLabel}</h3>
            <span className="status-pill status-pill--watch">{contractNormalizationAdjustments.length}</span>
          </div>
          <div className="sidebar-list">
            {contractNormalizationAdjustments.map((entry) => (
              <div className="sidebar-list__row" key={`${node.id}:contract-normalization:${entry.field}`}>
                <span>{normalizationFieldLabels[entry.field]}</span>
                <strong>{entry.previous} {"->"} {entry.next}</strong>
              </div>
            ))}
          </div>
        </div>
      ) : null}
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
      <div className="sidebar-stack">
        <div className="card-head">
          <h3>{labels.bridgeSeedModelTitle}</h3>
          <span className="status-pill status-pill--watch">seed</span>
        </div>
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{labels.bridgeSeedModelSummaryLabel}</span>
            <strong>{describeSeedModelTarget(node.operator_id)}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{labels.bridgeSeedModelNodeCountLabel}</span>
            <strong>{seedModelSummary.nodes}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{labels.bridgeSeedModelElementCountLabel}</span>
            <strong>{seedModelSummary.elements}</strong>
          </div>
        </div>
        {seedModelNodes.length > 0 ? (
          <div style={{ display: "grid", gap: "0.45rem" }}>
            <strong style={{ fontSize: "0.92rem" }}>nodes</strong>
            {seedModelNodes.map((seedNode, index) => (
              <div
                key={`${node.id}:seed-node:${index}`}
                style={{
                  display: "grid",
                  gap: "0.45rem",
                  gridTemplateColumns: "repeat(auto-fit, minmax(110px, 1fr))",
                  padding: "0.55rem",
                  borderRadius: "10px",
                  border: "1px solid var(--line)",
                }}
              >
                <label>
                  <span>id</span>
                  <input
                    onChange={(event) =>
                      onUpdateNode(node.id, (current) =>
                        updateSeedModelCollectionField(current, "nodes", index, "id", event.target.value),
                      )
                    }
                    value={String(seedNode.id ?? "")}
                  />
                </label>
                <label>
                  <span>x</span>
                  <input
                    onChange={(event) =>
                      onUpdateNode(node.id, (current) =>
                        updateSeedModelCollectionField(current, "nodes", index, "x", Number(event.target.value) || 0),
                      )
                    }
                    type="number"
                    value={typeof seedNode.x === "number" ? seedNode.x : 0}
                  />
                </label>
                <label>
                  <span>y</span>
                  <input
                    onChange={(event) =>
                      onUpdateNode(node.id, (current) =>
                        updateSeedModelCollectionField(current, "nodes", index, "y", Number(event.target.value) || 0),
                      )
                    }
                    type="number"
                    value={typeof seedNode.y === "number" ? seedNode.y : 0}
                  />
                </label>
                {node.operator_id?.startsWith("bridge.electrostatic_field_to_heat_") ? (
                  <>
                    <label>
                      <span>temperature</span>
                      <input
                        onChange={(event) =>
                          onUpdateNode(node.id, (current) =>
                            updateSeedModelCollectionField(current, "nodes", index, "temperature", Number(event.target.value) || 0),
                          )
                        }
                        type="number"
                        value={typeof seedNode.temperature === "number" ? seedNode.temperature : 0}
                      />
                    </label>
                    <label>
                      <span>heat_load</span>
                      <input
                        onChange={(event) =>
                          onUpdateNode(node.id, (current) =>
                            updateSeedModelCollectionField(current, "nodes", index, "heat_load", Number(event.target.value) || 0),
                          )
                        }
                        type="number"
                        value={typeof seedNode.heat_load === "number" ? seedNode.heat_load : 0}
                      />
                    </label>
                    <label style={{ display: "grid", alignContent: "end" }}>
                      <span>fix_temperature</span>
                      <input
                        checked={Boolean(seedNode.fix_temperature)}
                        onChange={(event) =>
                          onUpdateNode(node.id, (current) =>
                            updateSeedModelCollectionField(current, "nodes", index, "fix_temperature", event.target.checked),
                          )
                        }
                        type="checkbox"
                      />
                    </label>
                  </>
                ) : (
                  <>
                    <label>
                      <span>load_x</span>
                      <input
                        onChange={(event) =>
                          onUpdateNode(node.id, (current) =>
                            updateSeedModelCollectionField(current, "nodes", index, "load_x", Number(event.target.value) || 0),
                          )
                        }
                        type="number"
                        value={typeof seedNode.load_x === "number" ? seedNode.load_x : 0}
                      />
                    </label>
                    <label>
                      <span>load_y</span>
                      <input
                        onChange={(event) =>
                          onUpdateNode(node.id, (current) =>
                            updateSeedModelCollectionField(current, "nodes", index, "load_y", Number(event.target.value) || 0),
                          )
                        }
                        type="number"
                        value={typeof seedNode.load_y === "number" ? seedNode.load_y : 0}
                      />
                    </label>
                    <label>
                      <span>temperature_delta</span>
                      <input
                        onChange={(event) =>
                          onUpdateNode(node.id, (current) =>
                            updateSeedModelCollectionField(current, "nodes", index, "temperature_delta", Number(event.target.value) || 0),
                          )
                        }
                        type="number"
                        value={typeof seedNode.temperature_delta === "number" ? seedNode.temperature_delta : 0}
                      />
                    </label>
                    <label style={{ display: "grid", alignContent: "end" }}>
                      <span>fix_x</span>
                      <input
                        checked={Boolean(seedNode.fix_x)}
                        onChange={(event) =>
                          onUpdateNode(node.id, (current) =>
                            updateSeedModelCollectionField(current, "nodes", index, "fix_x", event.target.checked),
                          )
                        }
                        type="checkbox"
                      />
                    </label>
                    <label style={{ display: "grid", alignContent: "end" }}>
                      <span>fix_y</span>
                      <input
                        checked={Boolean(seedNode.fix_y)}
                        onChange={(event) =>
                          onUpdateNode(node.id, (current) =>
                            updateSeedModelCollectionField(current, "nodes", index, "fix_y", event.target.checked),
                          )
                        }
                        type="checkbox"
                      />
                    </label>
                  </>
                )}
              </div>
            ))}
          </div>
        ) : null}
        {seedModelElements.length > 0 ? (
          <div style={{ display: "grid", gap: "0.45rem" }}>
            <strong style={{ fontSize: "0.92rem" }}>elements</strong>
            {seedModelElements.map((seedElement, index) => (
              <div
                key={`${node.id}:seed-element:${index}`}
                style={{
                  display: "grid",
                  gap: "0.45rem",
                  gridTemplateColumns: "repeat(auto-fit, minmax(110px, 1fr))",
                  padding: "0.55rem",
                  borderRadius: "10px",
                  border: "1px solid var(--line)",
                }}
              >
                {["id", "node_i", "node_j", "node_k", "node_l", "thickness"].map((field) => (
                  <label key={field}>
                    <span>{field}</span>
                    <input
                      onChange={(event) =>
                        onUpdateNode(node.id, (current) =>
                          updateSeedModelCollectionField(
                            current,
                            "elements",
                            index,
                            field,
                            field === "id" ? event.target.value : Number(event.target.value) || 0,
                          ),
                        )
                      }
                      type={field === "id" ? "text" : "number"}
                      value={field === "id" ? String(seedElement[field] ?? "") : typeof seedElement[field] === "number" ? Number(seedElement[field]) : 0}
                    />
                  </label>
                ))}
                {(node.operator_id?.startsWith("bridge.electrostatic_field_to_heat_")
                  ? ["conductivity"]
                  : ["youngs_modulus", "poisson_ratio", "thermal_expansion", "material_id"]
                ).map((field) => (
                  <label key={field}>
                    <span>{field}</span>
                    <input
                      onChange={(event) =>
                        onUpdateNode(node.id, (current) =>
                          updateSeedModelCollectionField(
                            current,
                            "elements",
                            index,
                            field,
                            field === "material_id" ? event.target.value : Number(event.target.value) || 0,
                          ),
                        )
                      }
                      type={field === "material_id" ? "text" : "number"}
                      value={field === "material_id" ? String(seedElement[field] ?? "") : typeof seedElement[field] === "number" ? Number(seedElement[field]) : 0}
                    />
                  </label>
                ))}
              </div>
            ))}
          </div>
        ) : null}
        <label style={{ display: "grid", gap: "0.35rem" }}>
          <span>{labels.bridgeSeedModelDraftLabel}</span>
          <textarea
            onChange={(event) => setSeedModelDraft(event.target.value)}
            rows={12}
            spellCheck={false}
            value={seedModelDraft}
          />
        </label>
        {seedModelError ? <p className="card-copy" style={{ color: "var(--status-risk, #ff8f8f)" }}>{seedModelError}</p> : null}
        {!canImportCurrentWorkspaceModel ? (
          <p className="card-copy">{labels.bridgeSeedModelImportWorkspaceUnavailableLabel}</p>
        ) : null}
        <div className="button-row">
          <button disabled={!canImportCurrentWorkspaceModel} onClick={importCurrentWorkspaceModel} type="button">{labels.bridgeSeedModelImportWorkspaceLabel}</button>
          <button onClick={applySeedModelDraft} type="button">{labels.bridgeSeedModelApplyLabel}</button>
          <button onClick={resetSeedModelDraft} type="button">{labels.bridgeSeedModelResetLabel}</button>
        </div>
      </div>
    </div>
  );
}
