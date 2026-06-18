"use client";

import type { WorkflowOperatorDescriptor } from "@/lib/api";
import type { WorkflowNodeTemplatePreset } from "@/components/workbench/workflow/workbench-workflow-node-templates";

type WorkflowNodeTemplateSearchIndex = {
  combined: string;
  label: string;
  operatorId: string;
  kind: string;
  summary: string;
  family: string;
  domain: string;
  capabilities: string[];
};

const operatorSearchIndexCache = new Map<string, WorkflowNodeTemplateSearchIndex>();

function normalizeWorkflowSearchText(value?: string | null) {
  return value?.trim().toLowerCase() ?? "";
}

function tokenizeWorkflowSearchQuery(query: string) {
  return normalizeWorkflowSearchText(query).split(/\s+/).filter(Boolean);
}

function resolveWorkflowNodeTemplateSearchIndex(
  preset: WorkflowNodeTemplatePreset,
  descriptor?: WorkflowOperatorDescriptor,
) {
  const cacheKey = [
    preset.id,
    preset.label,
    preset.kind,
    preset.operatorId ?? "",
    descriptor?.summary ?? "",
    descriptor?.family ?? "",
    descriptor?.domain ?? "",
    descriptor?.capability_tags.join(" ") ?? "",
    preset.inputs.map((port) => `${port.id}:${port.artifact_type}:${port.description ?? ""}`).join("|"),
    preset.outputs.map((port) => `${port.id}:${port.artifact_type}:${port.description ?? ""}`).join("|"),
  ].join("::");
  const cached = operatorSearchIndexCache.get(cacheKey);
  if (cached) return cached;
  const capabilities = descriptor?.capability_tags.map((tag) => normalizeWorkflowSearchText(tag)) ?? [];
  const index = {
    combined: [
      preset.label,
      preset.operatorId,
      preset.kind,
      descriptor?.summary,
      descriptor?.family,
      descriptor?.domain,
      descriptor?.capability_tags.join(" "),
      ...preset.inputs.flatMap((port) => [port.id, port.artifact_type, port.description]),
      ...preset.outputs.flatMap((port) => [port.id, port.artifact_type, port.description]),
    ]
      .filter(Boolean)
      .map((value) => normalizeWorkflowSearchText(value))
      .join(" "),
    label: normalizeWorkflowSearchText(preset.label),
    operatorId: normalizeWorkflowSearchText(preset.operatorId),
    kind: normalizeWorkflowSearchText(preset.kind),
    summary: normalizeWorkflowSearchText(descriptor?.summary),
    family: normalizeWorkflowSearchText(descriptor?.family),
    domain: normalizeWorkflowSearchText(descriptor?.domain),
    capabilities,
  };
  operatorSearchIndexCache.set(cacheKey, index);
  return index;
}

export function scoreWorkflowNodeTemplatePresetSearch(
  preset: WorkflowNodeTemplatePreset,
  descriptor: WorkflowOperatorDescriptor | undefined,
  query: string,
) {
  const normalizedQuery = normalizeWorkflowSearchText(query);
  if (!normalizedQuery) return 0;
  const tokens = tokenizeWorkflowSearchQuery(normalizedQuery);
  if (tokens.length === 0) return 0;
  const index = resolveWorkflowNodeTemplateSearchIndex(preset, descriptor);
  if (!tokens.every((token) => index.combined.includes(token))) return null;

  let score = 0;
  if (index.label === normalizedQuery) score += 160;
  else if (index.label.startsWith(normalizedQuery)) score += 120;
  else if (index.label.includes(normalizedQuery)) score += 90;

  if (index.operatorId === normalizedQuery) score += 150;
  else if (index.operatorId.startsWith(normalizedQuery)) score += 110;
  else if (index.operatorId.includes(normalizedQuery)) score += 80;

  if (index.summary === normalizedQuery) score += 120;
  else if (index.summary.startsWith(normalizedQuery)) score += 90;
  else if (index.summary.includes(normalizedQuery)) score += 70;

  if (index.family === normalizedQuery) score += 50;
  if (index.domain === normalizedQuery) score += 45;
  if (index.kind === normalizedQuery) score += 35;
  if (index.capabilities.includes(normalizedQuery)) score += 40;

  for (const token of tokens) {
    if (index.label.startsWith(token)) score += 16;
    else if (index.label.includes(token)) score += 10;

    if (index.operatorId.startsWith(token)) score += 14;
    else if (index.operatorId.includes(token)) score += 9;

    if (index.summary.startsWith(token)) score += 12;
    else if (index.summary.includes(token)) score += 8;

    if (index.family.includes(token)) score += 6;
    if (index.domain.includes(token)) score += 6;
    if (index.kind.includes(token)) score += 4;
    if (index.capabilities.some((tag) => tag.includes(token))) score += 7;
  }

  return score;
}
