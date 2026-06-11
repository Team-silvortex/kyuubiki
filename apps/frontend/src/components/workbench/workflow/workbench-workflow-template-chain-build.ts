"use client";

import type { WorkflowGraphNode } from "@/lib/api";
import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";

function slugifyTemplateChainId(label: string) {
  const slug = label
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug || `template-chain-${Date.now()}`;
}

function compactOperatorName(operatorId?: string) {
  if (!operatorId) return "custom";
  return operatorId.split(".").pop() || operatorId;
}

function buildTemplateChainTagsFromNodes(nodes: WorkflowGraphNode[]) {
  const tags = new Set<string>();
  for (const node of nodes) {
    const operatorId = node.operator_id?.toLowerCase() ?? "";
    const kind = node.kind?.toLowerCase();
    if (kind) tags.add(kind);
    if (operatorId.includes("thermal")) tags.add("thermal");
    if (operatorId.includes("heat")) tags.add("heat");
    if (operatorId.includes("electrostatic")) tags.add("electrostatic");
    if (operatorId.includes("frame")) tags.add("frame");
    if (operatorId.includes("truss")) tags.add("truss");
    if (operatorId.includes("spring")) tags.add("spring");
    if (operatorId.includes("beam")) tags.add("beam");
    if (operatorId.includes("bridge")) tags.add("bridge");
    if (operatorId.includes("summary")) tags.add("summary");
    if (operatorId.includes("3d")) tags.add("3d");
    if (operatorId.includes("2d")) tags.add("2d");
    if (operatorId.includes("1d")) tags.add("1d");
  }
  return [...tags];
}

export function buildSuggestedTemplateChainLabel(nodes: WorkflowGraphNode[]) {
  const parts = nodes
    .slice(0, 3)
    .map((node) => compactOperatorName(node.operator_id));
  const suffix = nodes.length > 3 ? ` +${nodes.length - 3}` : "";
  return `${parts.join(" -> ")}${suffix}` || `chain-${nodes.length}`;
}

export function buildSuggestedTemplateChainSummary(nodes: WorkflowGraphNode[]) {
  const kinds = [...new Set(nodes.map((node) => node.kind))];
  return `Saved from ${nodes.length} selected workflow nodes (${kinds.join(", ")}).`;
}

export function buildTemplateSelectionsFromNodes(
  nodes: WorkflowGraphNode[],
): WorkflowNodeTemplateSelection[] {
  return nodes.map((node) => ({
    kind: node.kind,
    operatorId: node.operator_id,
    config: node.config ? structuredClone(node.config) : undefined,
  }));
}

export function buildImportedTemplateChainFromNodes(params: {
  label: string;
  nodes: WorkflowGraphNode[];
}) {
  const trimmedLabel = params.label.trim();
  return {
    id: slugifyTemplateChainId(trimmedLabel),
    label: trimmedLabel,
    summary: buildSuggestedTemplateChainSummary(params.nodes),
    tags: buildTemplateChainTagsFromNodes(params.nodes),
    version: "1.0.0",
    templates: buildTemplateSelectionsFromNodes(params.nodes),
  };
}
