"use client";

import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import type { WorkflowCatalogEntryArtifact, WorkflowGraphDefinition, WorkflowOperatorDescriptor } from "@/lib/api";
import { getWorkflowNodeTemplateSyncImpact, listAutoReconnectEdgeIds } from "@/components/workbench/workflow/workbench-workflow-template-impact";

export type WorkflowValidationFixSummaryEntry = {
  id: string;
  title: string;
  detail: string;
  nodeIds: string[];
  edgeIds: string[];
  portKeys: string[];
  artifactKeys: string[];
};

function pushUnique(values: string[], value?: string | null) {
  if (value && !values.includes(value)) values.push(value);
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

function summarizeIssue(
  issue: WorkflowGraphValidationIssue,
  graph: WorkflowGraphDefinition | null,
  operatorDescriptors: WorkflowOperatorDescriptor[],
): WorkflowValidationFixSummaryEntry {
  const nodeIds: string[] = [];
  const edgeIds: string[] = [];
  const portKeys: string[] = [];
  const artifactKeys: string[] = [];
  const entryInputs = graph?.entry_inputs ?? [];
  const outputArtifacts = graph?.output_artifacts ?? [];

  switch (issue.fix?.kind) {
    case "sync_node_template_from_operator":
      pushUnique(nodeIds, issue.fix.nodeId);
      if (graph) {
        const impact = getWorkflowNodeTemplateSyncImpact(
          graph,
          issue.fix.nodeId,
          { kind: issue.fix.templateKind, operatorId: issue.fix.operatorId },
          operatorDescriptors,
        );
        for (const edgeId of listAutoReconnectEdgeIds(impact)) pushUnique(edgeIds, edgeId);
      }
      return {
        id: issue.id,
        title: `同步节点模板 ${issue.fix.nodeId} -> ${issue.fix.operatorId}`,
        detail: `按算子契约重建端口，并尽量保留现有 bridge 配置与可自动重连的连线。`,
        nodeIds,
        edgeIds,
        portKeys,
        artifactKeys,
      };
    case "set_node_port_artifact_type_from_operator":
      pushUnique(nodeIds, issue.fix.nodeId);
      pushUnique(portKeys, `${issue.fix.nodeId}:${issue.fix.direction}:${issue.fix.portId}`);
      return {
        id: issue.id,
        title: `对齐端口类型 ${issue.fix.nodeId}.${issue.fix.portId}`,
        detail: `端口工件类型回收到 ${issue.fix.artifactType}。`,
        nodeIds,
        edgeIds,
        portKeys,
        artifactKeys,
      };
    case "set_node_port_dataset_value_from_operator":
      pushUnique(nodeIds, issue.fix.nodeId);
      pushUnique(portKeys, `${issue.fix.nodeId}:${issue.fix.direction}:${issue.fix.portId}`);
      return issue.fix.datasetValue
        ? {
            id: issue.id,
            title: `绑定数据集 ${issue.fix.nodeId}.${issue.fix.portId}`,
            detail: `节点端口会改绑到 ${issue.fix.datasetValue}。`,
            nodeIds,
            edgeIds,
            portKeys,
            artifactKeys,
          }
        : {
            id: issue.id,
            title: `清除数据集绑定 ${issue.fix.nodeId}.${issue.fix.portId}`,
            detail: `移除与算子契约不一致的端口数据集绑定。`,
            nodeIds,
            edgeIds,
            portKeys,
            artifactKeys,
          };
    case "clear_port_dataset_value":
      pushUnique(nodeIds, issue.fix.nodeId);
      pushUnique(portKeys, `${issue.fix.nodeId}:${issue.fix.direction}:${issue.fix.portId}`);
      return {
        id: issue.id,
        title: `清除端口数据集 ${issue.fix.nodeId}.${issue.fix.portId}`,
        detail: `删除失效的端口数据集引用。`,
        nodeIds,
        edgeIds,
        portKeys,
        artifactKeys,
      };
    case "set_edge_artifact_type_from_source":
    case "set_edge_artifact_type_from_target":
      pushUnique(edgeIds, issue.fix.edgeId);
      return {
        id: issue.id,
        title: `对齐连线类型 ${issue.fix.edgeId}`,
        detail: `连线工件类型会统一到 ${issue.fix.artifactType}。`,
        nodeIds,
        edgeIds,
        portKeys,
        artifactKeys,
      };
    case "clear_edge_dataset_value":
      pushUnique(edgeIds, issue.fix.edgeId);
      return {
        id: issue.id,
        title: `清除连线数据集 ${issue.fix.edgeId}`,
        detail: `删除失效的连线数据集引用。`,
        nodeIds,
        edgeIds,
        portKeys,
        artifactKeys,
      };
    case "set_catalog_artifact_type":
      pushUnique(
        artifactKeys,
        buildArtifactKey(
          issue.fix.mode,
          issue.fix.nodeId,
          issue.fix.artifactType,
          issue.fix.mode === "entry" ? entryInputs : outputArtifacts,
        ),
      );
      return {
        id: issue.id,
        title: `回填工件类型 ${issue.fix.nodeId}`,
        detail: `${issue.fix.mode === "entry" ? "入口" : "输出"}工件会改成 ${issue.fix.artifactType}。`,
        nodeIds,
        edgeIds,
        portKeys,
        artifactKeys,
      };
    default:
      return {
        id: issue.id,
        title: issue.message,
        detail: `该修复会把当前节点或连线收回到已知的工作流契约。`,
        nodeIds,
        edgeIds,
        portKeys,
        artifactKeys,
      };
  }
}

export function buildWorkflowValidationFixSummary(
  issues: WorkflowGraphValidationIssue[],
  graph: WorkflowGraphDefinition | null = null,
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
  limit = 4,
) {
  const entries = issues.map((issue) => summarizeIssue(issue, graph, operatorDescriptors));
  const deduped: WorkflowValidationFixSummaryEntry[] = [];
  const seen = new Set<string>();

  for (const entry of entries) {
    const key = `${entry.title}::${entry.detail}`;
    if (seen.has(key)) continue;
    seen.add(key);
    deduped.push(entry);
    if (deduped.length >= limit) break;
  }

  return deduped;
}
