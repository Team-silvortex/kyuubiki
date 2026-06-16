"use client";

import type {
  WorkflowGraphArtifactValue,
  WorkflowGraphDefinition,
  WorkflowGraphJobResult,
} from "@/lib/api";
import { resolveBridgeContractForOperator } from "@/lib/workbench/workflow-bridge-contract";

export type WorkflowBridgeRuntimeValidationIssue = {
  id: string;
  level: "warning";
  message: string;
  nodeId: string;
  artifactKey?: string;
  upstreamNodeId?: string;
  inputEdgeId?: string;
  outputEdgeId?: string;
  downstreamNodeIds?: string[];
  outputEdgeIds?: string[];
};

export type WorkflowBridgeRuntimeInspectionRecord = {
  nodeId: string;
  operatorId: string;
  upstreamNodeId?: string;
  downstreamNodeIds: string[];
  inputArtifactKey?: string;
  outputArtifactKey?: string;
  sourceField: string;
  targetField: string;
  reduction: string;
  scale: number;
  sourceFieldExposed: boolean;
  targetFieldExposed: boolean;
};

export type WorkflowBridgeRuntimeNodeStatus = {
  label: "aligned" | "drift" | "missing-runtime";
  tone: "good" | "watch" | "risk";
};

type WorkflowBridgeRuntimeExpectation = {
  inputPortId: string;
  outputPortId: string;
};

const BRIDGE_RUNTIME_EXPECTATIONS: Record<string, WorkflowBridgeRuntimeExpectation> = {
  "bridge.electrostatic_field_to_heat_quad_2d": {
    inputPortId: "electrostatic_result",
    outputPortId: "heat_model",
  },
  "bridge.electrostatic_field_to_heat_triangle_2d": {
    inputPortId: "electrostatic_result",
    outputPortId: "heat_model",
  },
  "bridge.temperature_field_to_thermo_quad_2d": {
    inputPortId: "heat_result",
    outputPortId: "thermo_model",
  },
  "bridge.temperature_field_to_thermo_triangle_2d": {
    inputPortId: "heat_result",
    outputPortId: "thermo_model",
  },
};

type WorkflowResolvedRuntimeArtifact = {
  artifactKey: string;
  nodeId?: string;
  portId?: string;
  payload: Record<string, unknown>;
};

type WorkflowBridgeRuntimeGraph = Pick<WorkflowGraphDefinition, "nodes" | "edges">;

export function validateWorkflowBridgeRuntimeContracts(
  graph: WorkflowBridgeRuntimeGraph | null,
  result?: WorkflowGraphJobResult | null,
): WorkflowBridgeRuntimeValidationIssue[] {
  if (!graph || !result) return [];

  const issues: WorkflowBridgeRuntimeValidationIssue[] = [];
  const artifacts = resolveRuntimeArtifacts(result);

  for (const node of graph.nodes) {
    const operatorId = node.operator_id ?? "";
    const expectation = BRIDGE_RUNTIME_EXPECTATIONS[operatorId];
    if (!expectation) continue;

    const contract = resolveBridgeContractForOperator(
      operatorId,
      node.config as Record<string, unknown> | null | undefined,
    );
    if (!contract) continue;

    const inputEdge = (graph.edges ?? []).find(
      (edge) => edge.to.node === node.id && edge.to.port === expectation.inputPortId,
    );
    if (!inputEdge) continue;
    const outputEdges = (graph.edges ?? []).filter(
      (edge) => edge.from.node === node.id && edge.from.port === expectation.outputPortId,
    );
    const downstreamNodeIds = outputEdges.map((edge) => edge.to.node);
    const outputEdgeIds = outputEdges.map((edge) => edge.id);

    const upstreamArtifact = artifacts.find(
      (artifact) =>
        artifact.nodeId === inputEdge.from.node && artifact.portId === inputEdge.from.port,
    );
    if (!upstreamArtifact) {
      issues.push({
        id: `bridge:runtime:missing-input:${node.id}`,
        level: "warning",
        message: `Bridge node "${node.id}" has no resolved upstream runtime artifact for "${inputEdge.from.node}.${inputEdge.from.port}".`,
        nodeId: node.id,
        upstreamNodeId: inputEdge.from.node,
        inputEdgeId: inputEdge.id,
        outputEdgeId: outputEdgeIds[0],
        downstreamNodeIds,
        outputEdgeIds,
      });
    } else if (!payloadExposesField(upstreamArtifact.payload, contract.source.field)) {
      issues.push({
        id: `bridge:runtime:source-field:${node.id}`,
        level: "warning",
        message: `Bridge node "${node.id}" expects source field "${contract.source.field}" in upstream artifact "${upstreamArtifact.artifactKey}", but the runtime payload does not expose it.`,
        nodeId: node.id,
        artifactKey: upstreamArtifact.artifactKey,
        upstreamNodeId: inputEdge.from.node,
        inputEdgeId: inputEdge.id,
        outputEdgeId: outputEdgeIds[0],
        downstreamNodeIds,
        outputEdgeIds,
      });
    }

    const outputArtifact = artifacts.find(
      (artifact) =>
        artifact.nodeId === node.id && artifact.portId === expectation.outputPortId,
    );
    if (!outputArtifact) {
      issues.push({
        id: `bridge:runtime:missing-output:${node.id}`,
        level: "warning",
        message: `Bridge node "${node.id}" did not emit a runtime artifact for output "${expectation.outputPortId}".`,
        nodeId: node.id,
        upstreamNodeId: inputEdge.from.node,
        inputEdgeId: inputEdge.id,
        outputEdgeId: outputEdgeIds[0],
        downstreamNodeIds,
        outputEdgeIds,
      });
    } else if (!payloadExposesField(outputArtifact.payload, contract.target.field)) {
      issues.push({
        id: `bridge:runtime:target-field:${node.id}`,
        level: "warning",
        message: `Bridge node "${node.id}" targets field "${contract.target.field}" but runtime artifact "${outputArtifact.artifactKey}" does not expose it.`,
        nodeId: node.id,
        artifactKey: outputArtifact.artifactKey,
        upstreamNodeId: inputEdge.from.node,
        inputEdgeId: inputEdge.id,
        outputEdgeId: outputEdgeIds[0],
        downstreamNodeIds,
        outputEdgeIds,
      });
    }
  }

  return issues;
}

export function inspectWorkflowBridgeRuntimePaths(
  graph: WorkflowBridgeRuntimeGraph | null,
  result?: WorkflowGraphJobResult | null,
): WorkflowBridgeRuntimeInspectionRecord[] {
  if (!graph || !result) return [];

  const artifacts = resolveRuntimeArtifacts(result);
  const records: WorkflowBridgeRuntimeInspectionRecord[] = [];

  for (const node of graph.nodes) {
    const operatorId = node.operator_id ?? "";
    const expectation = BRIDGE_RUNTIME_EXPECTATIONS[operatorId];
    if (!expectation) continue;
    const contract = resolveBridgeContractForOperator(
      operatorId,
      node.config as Record<string, unknown> | null | undefined,
    );
    if (!contract) continue;
    const inputEdge = (graph.edges ?? []).find(
      (edge) => edge.to.node === node.id && edge.to.port === expectation.inputPortId,
    );
    const outputEdges = (graph.edges ?? []).filter(
      (edge) => edge.from.node === node.id && edge.from.port === expectation.outputPortId,
    );
    const upstreamArtifact = inputEdge
      ? artifacts.find(
          (artifact) =>
            artifact.nodeId === inputEdge.from.node && artifact.portId === inputEdge.from.port,
        )
      : null;
    const outputArtifact = artifacts.find(
      (artifact) =>
        artifact.nodeId === node.id && artifact.portId === expectation.outputPortId,
    );
    records.push({
      nodeId: node.id,
      operatorId,
      upstreamNodeId: inputEdge?.from.node,
      downstreamNodeIds: outputEdges.map((edge) => edge.to.node),
      inputArtifactKey: upstreamArtifact?.artifactKey,
      outputArtifactKey: outputArtifact?.artifactKey,
      sourceField: contract.source.field,
      targetField: contract.target.field,
      reduction: contract.transform.reduction,
      scale: contract.transform.scale,
      sourceFieldExposed: upstreamArtifact
        ? payloadExposesField(upstreamArtifact.payload, contract.source.field)
        : false,
      targetFieldExposed: outputArtifact
        ? payloadExposesField(outputArtifact.payload, contract.target.field)
        : false,
    });
  }

  return records;
}

export function buildWorkflowBridgeRuntimeStatusMap(
  graph: WorkflowBridgeRuntimeGraph | null,
  result?: WorkflowGraphJobResult | null,
) {
  return new Map<string, WorkflowBridgeRuntimeNodeStatus>(
    inspectWorkflowBridgeRuntimePaths(graph, result).map((record) => {
      const status: WorkflowBridgeRuntimeNodeStatus = !record.inputArtifactKey && !record.outputArtifactKey
        ? { label: "missing-runtime", tone: "risk" }
        : record.sourceFieldExposed && record.targetFieldExposed
          ? { label: "aligned", tone: "good" }
          : { label: "drift", tone: "watch" };
      return [record.nodeId, status] as const;
    }),
  );
}

export function summarizeWorkflowBridgeRuntimeStatuses(
  graph: WorkflowBridgeRuntimeGraph | null,
  result?: WorkflowGraphJobResult | null,
) {
  const counts = { aligned: 0, drift: 0, "missing-runtime": 0 };
  for (const status of buildWorkflowBridgeRuntimeStatusMap(graph, result).values()) {
    if (status.label === "aligned") counts.aligned += 1;
    else if (status.label === "drift") counts.drift += 1;
    else counts["missing-runtime"] += 1;
  }
  return counts;
}

function resolveRuntimeArtifacts(result: WorkflowGraphJobResult) {
  return Object.entries(result.artifacts ?? {})
    .map(([artifactKey, artifact]) => resolveRuntimeArtifact(artifactKey, artifact))
    .filter(
      (artifact): artifact is WorkflowResolvedRuntimeArtifact => Boolean(artifact),
    );
}

function resolveRuntimeArtifact(
  artifactKey: string,
  artifact: WorkflowGraphArtifactValue,
): WorkflowResolvedRuntimeArtifact | null {
  if (typeof artifact !== "object" || artifact === null) return null;
  const envelope = artifact as Record<string, unknown>;
  const payload = readPayloadRecord(envelope.payload) ?? readPayloadRecord(envelope.content);
  if (!payload) return null;
  return {
    artifactKey,
    nodeId: typeof envelope.node_id === "string" ? envelope.node_id : undefined,
    portId: typeof envelope.port_id === "string" ? envelope.port_id : undefined,
    payload,
  };
}

function readPayloadRecord(value: unknown): Record<string, unknown> | null {
  if (typeof value === "string") {
    try {
      const parsed = JSON.parse(value) as unknown;
      return typeof parsed === "object" && parsed !== null
        ? (parsed as Record<string, unknown>)
        : null;
    } catch {
      return null;
    }
  }
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : null;
}

function payloadExposesField(
  payload: Record<string, unknown>,
  field: string,
): boolean {
  if (field in payload) return true;
  if (collectionExposesField(payload.nodes, field)) return true;
  if (collectionExposesField(payload.elements, field)) return true;
  return false;
}

function collectionExposesField(value: unknown, field: string) {
  if (!Array.isArray(value)) return false;
  return value.some(
    (entry) => typeof entry === "object" && entry !== null && field in entry,
  );
}
