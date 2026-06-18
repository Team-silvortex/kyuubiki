"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";
import {
  buildWorkflowSearchSuggestions,
  findWorkflowSearchMatches,
  includesAllWorkflowSearchTokens,
  normalizeWorkflowSearchText,
  scoreWorkflowSearchFields,
  tokenizeWorkflowSearchQuery,
  type WorkflowSearchWeightedField,
} from "@/components/workbench/workflow/workbench-workflow-search-score";

type WorkflowCatalogSearchIndex = {
  combined: string;
  name: string;
  id: string;
  summary: string;
  domains: string[];
  capabilities: string[];
  operatorIds: string[];
  artifacts: string[];
  localTags: string[];
};

const workflowCatalogSearchIndexCache = new Map<string, WorkflowCatalogSearchIndex>();

function resolveWorkflowCatalogSearchIndex(workflow: WorkflowCatalogEntry) {
  const operatorIds = [
    ...(workflow.runtime_manifest?.required_operator_ids ?? []),
    ...(workflow.graph?.nodes.map((node) => node.operator_id ?? "").filter(Boolean) ?? []),
  ].map((value) => normalizeWorkflowSearchText(value));
  const artifacts = [
    ...workflow.entry_inputs.map((artifact) => artifact.artifact_type),
    ...workflow.output_artifacts.map((artifact) => artifact.artifact_type),
  ].map((value) => normalizeWorkflowSearchText(value));
  const cacheKey = [
    workflow.id,
    workflow.name,
    workflow.summary,
    workflow.version,
    workflow.domains?.join("|") ?? "",
    workflow.capability_tags?.join("|") ?? "",
    workflow.local?.tags?.join("|") ?? "",
    workflow.local?.notes ?? "",
    workflow.local?.source_workflow_id ?? "",
    workflow.local?.source_workflow_name ?? "",
    workflow.local?.variant_of_workflow_id ?? "",
    workflow.local?.variant_of_workflow_name ?? "",
    workflow.local?.imported_from_package_id ?? "",
    workflow.local?.imported_from_package_version ?? "",
    operatorIds.join("|"),
    artifacts.join("|"),
  ].join("::");
  const cached = workflowCatalogSearchIndexCache.get(cacheKey);
  if (cached) return cached;
  const domains = (workflow.domains ?? []).map((value) => normalizeWorkflowSearchText(value));
  const capabilities = (workflow.capability_tags ?? []).map((value) =>
    normalizeWorkflowSearchText(value),
  );
  const localTags = (workflow.local?.tags ?? []).map((value) =>
    normalizeWorkflowSearchText(value),
  );
  const index = {
    combined: [
      workflow.id,
      workflow.name,
      workflow.summary,
      workflow.version,
      workflow.domains?.join(" ") ?? "",
      workflow.capability_tags?.join(" ") ?? "",
      workflow.local?.tags?.join(" ") ?? "",
      workflow.local?.notes ?? "",
      workflow.local?.source_workflow_id ?? "",
      workflow.local?.source_workflow_name ?? "",
      workflow.local?.variant_of_workflow_id ?? "",
      workflow.local?.variant_of_workflow_name ?? "",
      workflow.local?.imported_from_package_id ?? "",
      workflow.local?.imported_from_package_version ?? "",
      ...operatorIds,
      ...artifacts,
    ]
      .filter(Boolean)
      .map((value) => normalizeWorkflowSearchText(value))
      .join(" "),
    name: normalizeWorkflowSearchText(workflow.name),
    id: normalizeWorkflowSearchText(workflow.id),
    summary: normalizeWorkflowSearchText(workflow.summary),
    domains,
    capabilities,
    operatorIds,
    artifacts,
    localTags,
  };
  workflowCatalogSearchIndexCache.set(cacheKey, index);
  return index;
}

export function scoreWorkflowCatalogEntrySearch(
  workflow: WorkflowCatalogEntry,
  query: string,
) {
  const normalizedQuery = normalizeWorkflowSearchText(query);
  if (!normalizedQuery) return 0;
  const tokens = tokenizeWorkflowSearchQuery(normalizedQuery);
  if (tokens.length === 0) return 0;
  const index = resolveWorkflowCatalogSearchIndex(workflow);
  if (!includesAllWorkflowSearchTokens(index.combined, tokens)) return null;
  const weightedFields: WorkflowSearchWeightedField[] = [
    { value: index.name, exact: 180, prefix: 140, contains: 100 },
    { value: index.id, exact: 160, prefix: 120, contains: 85 },
    { value: index.summary, exact: 110, prefix: 85, contains: 60 },
  ];
  return (
    scoreWorkflowSearchFields(
      weightedFields,
      [...index.domains, ...index.capabilities, ...index.localTags, ...index.operatorIds, ...index.artifacts],
      normalizedQuery,
      tokens,
    ) +
    (index.domains.includes(normalizedQuery) ? 60 : 0) +
    (index.capabilities.includes(normalizedQuery) ? 55 : 0) +
    (index.localTags.includes(normalizedQuery) ? 55 : 0) +
    (index.operatorIds.includes(normalizedQuery) ? 70 : 0) +
    (index.artifacts.includes(normalizedQuery) ? 50 : 0)
  );
}

export function describeWorkflowCatalogEntrySearchMatches(
  workflow: WorkflowCatalogEntry,
  query: string,
) {
  const normalizedQuery = normalizeWorkflowSearchText(query);
  if (!normalizedQuery) return [];
  const tokens = tokenizeWorkflowSearchQuery(normalizedQuery);
  if (tokens.length === 0) return [];
  const index = resolveWorkflowCatalogSearchIndex(workflow);

  const matches: string[] = [];
  const hasTokenMatch = (values: string[]) => findWorkflowSearchMatches(values, tokens);

  if (hasTokenMatch([index.name])) matches.push(`name: ${workflow.name}`);
  if (hasTokenMatch([index.id])) matches.push(`id: ${workflow.id}`);
  if (hasTokenMatch([index.summary])) matches.push(`summary`);

  const matchedDomains = (workflow.domains ?? []).filter((value) =>
    tokens.every((token) => normalizeWorkflowSearchText(value).includes(token)),
  );
  if (matchedDomains.length > 0) matches.push(`domain: ${matchedDomains.slice(0, 2).join(", ")}`);

  const matchedCapabilities = (workflow.capability_tags ?? []).filter((value) =>
    tokens.every((token) => normalizeWorkflowSearchText(value).includes(token)),
  );
  if (matchedCapabilities.length > 0) {
    matches.push(`capability: ${matchedCapabilities.slice(0, 2).join(", ")}`);
  }

  const matchedLocalTags = (workflow.local?.tags ?? []).filter((value) =>
    tokens.every((token) => normalizeWorkflowSearchText(value).includes(token)),
  );
  if (matchedLocalTags.length > 0) matches.push(`tag: ${matchedLocalTags.slice(0, 2).join(", ")}`);

  const matchedOperators = [
    ...(workflow.runtime_manifest?.required_operator_ids ?? []),
    ...(workflow.graph?.nodes.map((node) => node.operator_id).filter(Boolean) ?? []),
  ].filter((value): value is string =>
    tokens.every((token) => normalizeWorkflowSearchText(value).includes(token)),
  );
  if (matchedOperators.length > 0) {
    matches.push(`operator: ${matchedOperators.slice(0, 2).join(", ")}`);
  }

  const matchedArtifacts = [
    ...workflow.entry_inputs.map((artifact) => artifact.artifact_type),
    ...workflow.output_artifacts.map((artifact) => artifact.artifact_type),
  ].filter((value) =>
    tokens.every((token) => normalizeWorkflowSearchText(value).includes(token)),
  );
  if (matchedArtifacts.length > 0) {
    matches.push(`artifact: ${matchedArtifacts.slice(0, 2).join(", ")}`);
  }

  return matches.slice(0, 3);
}

export function suggestWorkflowCatalogEntries(
  workflows: WorkflowCatalogEntry[],
  query: string,
) {
  return buildWorkflowSearchSuggestions(
    workflows,
    query,
    (workflow, normalizedQuery) => ({
      score: scoreWorkflowCatalogEntrySearch(workflow, normalizedQuery),
      matchSummary: describeWorkflowCatalogEntrySearchMatches(workflow, normalizedQuery),
    }),
    (workflow) => workflow.name,
  ).map((entry) => ({
    workflow: entry.item,
    score: entry.score,
    matchSummary: entry.matchSummary,
  }));
}
