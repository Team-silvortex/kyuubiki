"use client";

import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";

function summarizeIssue(issue: WorkflowGraphValidationIssue) {
  switch (issue.fix?.kind) {
    case "sync_node_template_from_operator":
      return `同步节点模板 ${issue.fix.nodeId} -> ${issue.fix.operatorId}`;
    case "set_node_port_artifact_type_from_operator":
      return `对齐端口类型 ${issue.fix.nodeId}.${issue.fix.portId} -> ${issue.fix.artifactType}`;
    case "set_node_port_dataset_value_from_operator":
      return issue.fix.datasetValue
        ? `绑定数据集 ${issue.fix.nodeId}.${issue.fix.portId} -> ${issue.fix.datasetValue}`
        : `清除数据集绑定 ${issue.fix.nodeId}.${issue.fix.portId}`;
    case "clear_port_dataset_value":
      return `清除端口数据集 ${issue.fix.nodeId}.${issue.fix.portId}`;
    case "set_edge_artifact_type_from_source":
    case "set_edge_artifact_type_from_target":
      return `对齐连线类型 ${issue.fix.edgeId} -> ${issue.fix.artifactType}`;
    case "clear_edge_dataset_value":
      return `清除连线数据集 ${issue.fix.edgeId}`;
    case "set_catalog_artifact_type":
      return `回填工件类型 ${issue.fix.nodeId} -> ${issue.fix.artifactType}`;
    default:
      return issue.message;
  }
}

export function buildWorkflowValidationFixSummary(issues: WorkflowGraphValidationIssue[], limit = 4) {
  return Array.from(new Set(issues.map(summarizeIssue))).slice(0, limit);
}
