"use client";

import type { WorkflowOperatorDescriptor } from "@/lib/api";
import type { WorkflowNodeTemplatePreset } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import {
  buildWorkflowSearchSuggestions,
  findWorkflowSearchMatches,
  includesAllWorkflowSearchTokens,
  normalizeWorkflowSearchText,
  scoreWorkflowSearchFields,
  tokenizeWorkflowSearchQuery,
  type WorkflowSearchWeightedField,
} from "@/components/workbench/workflow/workbench-workflow-search-score";

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

export type WorkflowNodeTemplateSearchSuggestion = {
  preset: WorkflowNodeTemplatePreset;
  score: number;
  matchSummary: string[];
};

const operatorSearchIndexCache = new Map<string, WorkflowNodeTemplateSearchIndex>();

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
  if (!includesAllWorkflowSearchTokens(index.combined, tokens)) return null;
  const weightedFields: WorkflowSearchWeightedField[] = [
    { value: index.label, exact: 160, prefix: 120, contains: 90 },
    { value: index.operatorId, exact: 150, prefix: 110, contains: 80 },
    { value: index.summary, exact: 120, prefix: 90, contains: 70 },
  ];
  return (
    scoreWorkflowSearchFields(
      weightedFields,
      [index.family, index.domain, index.kind, ...index.capabilities],
      normalizedQuery,
      tokens,
    ) +
    (index.family === normalizedQuery ? 50 : 0) +
    (index.domain === normalizedQuery ? 45 : 0) +
    (index.kind === normalizedQuery ? 35 : 0) +
    (index.capabilities.includes(normalizedQuery) ? 40 : 0)
  );
}

export function describeWorkflowNodeTemplatePresetSearchMatches(
  preset: WorkflowNodeTemplatePreset,
  descriptor: WorkflowOperatorDescriptor | undefined,
  query: string,
) {
  const normalizedQuery = normalizeWorkflowSearchText(query);
  if (!normalizedQuery) return [];
  const tokens = tokenizeWorkflowSearchQuery(normalizedQuery);
  if (tokens.length === 0) return [];
  const index = resolveWorkflowNodeTemplateSearchIndex(preset, descriptor);
  const matches: string[] = [];
  const hasTokenMatch = (values: string[]) => findWorkflowSearchMatches(values, tokens);

  if (hasTokenMatch([index.label])) matches.push(`label: ${preset.label}`);
  if (hasTokenMatch([index.operatorId])) matches.push(`operator: ${preset.operatorId}`);
  if (hasTokenMatch([index.summary])) matches.push("summary");
  if (hasTokenMatch([index.family])) matches.push(`family: ${descriptor?.family ?? preset.kind}`);
  if (hasTokenMatch([index.domain])) matches.push(`domain: ${descriptor?.domain ?? "other"}`);
  const matchedCapabilities = (descriptor?.capability_tags ?? []).filter((value) =>
    tokens.every((token) => normalizeWorkflowSearchText(value).includes(token)),
  );
  if (matchedCapabilities.length > 0) {
    matches.push(`capability: ${matchedCapabilities.slice(0, 2).join(", ")}`);
  }
  return matches.slice(0, 3);
}

export function suggestWorkflowNodeTemplatePresets(
  presets: WorkflowNodeTemplatePreset[],
  operatorDescriptorMap: Map<string, WorkflowOperatorDescriptor>,
  query: string,
) {
  const normalizedQuery = normalizeWorkflowSearchText(query);
  return buildWorkflowSearchSuggestions(
    presets,
    query,
    (preset, normalizedQuery) => {
      const descriptor = preset.operatorId
        ? operatorDescriptorMap.get(preset.operatorId)
        : undefined;
      return {
        score: scoreWorkflowNodeTemplatePresetSearch(preset, descriptor, normalizedQuery),
        matchSummary: describeWorkflowNodeTemplatePresetSearchMatches(
          preset,
          descriptor,
          normalizedQuery,
        ),
      };
    },
    (preset) => preset.label,
  ).map((entry) => ({
    preset: entry.item,
    score: entry.score,
    matchSummary: entry.matchSummary,
  }));
}
