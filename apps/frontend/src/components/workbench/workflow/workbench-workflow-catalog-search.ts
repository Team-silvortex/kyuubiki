"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";

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

function normalizeWorkflowCatalogSearchText(value?: string | null) {
  return value?.trim().toLowerCase() ?? "";
}

function tokenizeWorkflowCatalogSearchQuery(query: string) {
  return normalizeWorkflowCatalogSearchText(query).split(/\s+/).filter(Boolean);
}

function resolveWorkflowCatalogSearchIndex(workflow: WorkflowCatalogEntry) {
  const operatorIds = [
    ...(workflow.runtime_manifest?.required_operator_ids ?? []),
    ...(workflow.graph?.nodes.map((node) => node.operator_id ?? "").filter(Boolean) ?? []),
  ].map((value) => normalizeWorkflowCatalogSearchText(value));
  const artifacts = [
    ...workflow.entry_inputs.map((artifact) => artifact.artifact_type),
    ...workflow.output_artifacts.map((artifact) => artifact.artifact_type),
  ].map((value) => normalizeWorkflowCatalogSearchText(value));
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
  const domains = (workflow.domains ?? []).map((value) => normalizeWorkflowCatalogSearchText(value));
  const capabilities = (workflow.capability_tags ?? []).map((value) =>
    normalizeWorkflowCatalogSearchText(value),
  );
  const localTags = (workflow.local?.tags ?? []).map((value) =>
    normalizeWorkflowCatalogSearchText(value),
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
      .map((value) => normalizeWorkflowCatalogSearchText(value))
      .join(" "),
    name: normalizeWorkflowCatalogSearchText(workflow.name),
    id: normalizeWorkflowCatalogSearchText(workflow.id),
    summary: normalizeWorkflowCatalogSearchText(workflow.summary),
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
  const normalizedQuery = normalizeWorkflowCatalogSearchText(query);
  if (!normalizedQuery) return 0;
  const tokens = tokenizeWorkflowCatalogSearchQuery(normalizedQuery);
  if (tokens.length === 0) return 0;
  const index = resolveWorkflowCatalogSearchIndex(workflow);
  if (!tokens.every((token) => index.combined.includes(token))) return null;

  let score = 0;
  if (index.name === normalizedQuery) score += 180;
  else if (index.name.startsWith(normalizedQuery)) score += 140;
  else if (index.name.includes(normalizedQuery)) score += 100;

  if (index.id === normalizedQuery) score += 160;
  else if (index.id.startsWith(normalizedQuery)) score += 120;
  else if (index.id.includes(normalizedQuery)) score += 85;

  if (index.summary === normalizedQuery) score += 110;
  else if (index.summary.startsWith(normalizedQuery)) score += 85;
  else if (index.summary.includes(normalizedQuery)) score += 60;

  if (index.domains.includes(normalizedQuery)) score += 60;
  if (index.capabilities.includes(normalizedQuery)) score += 55;
  if (index.localTags.includes(normalizedQuery)) score += 55;
  if (index.operatorIds.includes(normalizedQuery)) score += 70;
  if (index.artifacts.includes(normalizedQuery)) score += 50;

  for (const token of tokens) {
    if (index.name.startsWith(token)) score += 16;
    else if (index.name.includes(token)) score += 10;
    if (index.id.startsWith(token)) score += 14;
    else if (index.id.includes(token)) score += 9;
    if (index.summary.startsWith(token)) score += 12;
    else if (index.summary.includes(token)) score += 8;
    score += index.domains.filter((value) => value.includes(token)).length * 7;
    score += index.capabilities.filter((value) => value.includes(token)).length * 7;
    score += index.localTags.filter((value) => value.includes(token)).length * 7;
    score += index.operatorIds.filter((value) => value.includes(token)).length * 8;
    score += index.artifacts.filter((value) => value.includes(token)).length * 6;
  }

  return score;
}

export function describeWorkflowCatalogEntrySearchMatches(
  workflow: WorkflowCatalogEntry,
  query: string,
) {
  const normalizedQuery = normalizeWorkflowCatalogSearchText(query);
  if (!normalizedQuery) return [];
  const tokens = tokenizeWorkflowCatalogSearchQuery(normalizedQuery);
  if (tokens.length === 0) return [];
  const index = resolveWorkflowCatalogSearchIndex(workflow);

  const matches: string[] = [];
  const hasTokenMatch = (values: string[]) =>
    tokens.every((token) => values.some((value) => value.includes(token)));

  if (hasTokenMatch([index.name])) matches.push(`name: ${workflow.name}`);
  if (hasTokenMatch([index.id])) matches.push(`id: ${workflow.id}`);
  if (hasTokenMatch([index.summary])) matches.push(`summary`);

  const matchedDomains = (workflow.domains ?? []).filter((value) =>
    tokens.every((token) => normalizeWorkflowCatalogSearchText(value).includes(token)),
  );
  if (matchedDomains.length > 0) matches.push(`domain: ${matchedDomains.slice(0, 2).join(", ")}`);

  const matchedCapabilities = (workflow.capability_tags ?? []).filter((value) =>
    tokens.every((token) => normalizeWorkflowCatalogSearchText(value).includes(token)),
  );
  if (matchedCapabilities.length > 0) {
    matches.push(`capability: ${matchedCapabilities.slice(0, 2).join(", ")}`);
  }

  const matchedLocalTags = (workflow.local?.tags ?? []).filter((value) =>
    tokens.every((token) => normalizeWorkflowCatalogSearchText(value).includes(token)),
  );
  if (matchedLocalTags.length > 0) matches.push(`tag: ${matchedLocalTags.slice(0, 2).join(", ")}`);

  const matchedOperators = [
    ...(workflow.runtime_manifest?.required_operator_ids ?? []),
    ...(workflow.graph?.nodes.map((node) => node.operator_id).filter(Boolean) ?? []),
  ].filter((value): value is string =>
    tokens.every((token) => normalizeWorkflowCatalogSearchText(value).includes(token)),
  );
  if (matchedOperators.length > 0) {
    matches.push(`operator: ${matchedOperators.slice(0, 2).join(", ")}`);
  }

  const matchedArtifacts = [
    ...workflow.entry_inputs.map((artifact) => artifact.artifact_type),
    ...workflow.output_artifacts.map((artifact) => artifact.artifact_type),
  ].filter((value) =>
    tokens.every((token) => normalizeWorkflowCatalogSearchText(value).includes(token)),
  );
  if (matchedArtifacts.length > 0) {
    matches.push(`artifact: ${matchedArtifacts.slice(0, 2).join(", ")}`);
  }

  return matches.slice(0, 3);
}
