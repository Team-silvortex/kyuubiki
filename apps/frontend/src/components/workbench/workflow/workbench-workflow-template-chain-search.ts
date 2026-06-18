"use client";

import type { WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";

type WorkflowTemplateChainSearchIndex = {
  combined: string;
  label: string;
  id: string;
  summary: string;
  tags: string[];
  templateOperators: string[];
  templateKinds: string[];
};

const templateChainSearchIndexCache = new Map<string, WorkflowTemplateChainSearchIndex>();

function normalizeWorkflowTemplateChainSearchText(value?: string | null) {
  return value?.trim().toLowerCase() ?? "";
}

function tokenizeWorkflowTemplateChainSearchQuery(query: string) {
  return normalizeWorkflowTemplateChainSearchText(query).split(/\s+/).filter(Boolean);
}

function resolveWorkflowTemplateChainSearchIndex(chain: WorkflowTemplateChainDefinition) {
  const cacheKey = [
    chain.id,
    chain.label,
    chain.summary ?? "",
    chain.tags?.join("|") ?? "",
    chain.templates.map((template) => `${template.operatorId ?? ""}:${template.kind ?? ""}`).join("|"),
  ].join("::");
  const cached = templateChainSearchIndexCache.get(cacheKey);
  if (cached) return cached;
  const tags = (chain.tags ?? []).map((tag) => normalizeWorkflowTemplateChainSearchText(tag));
  const templateOperators = chain.templates
    .map((template) => normalizeWorkflowTemplateChainSearchText(template.operatorId))
    .filter(Boolean);
  const templateKinds = chain.templates
    .map((template) => normalizeWorkflowTemplateChainSearchText(template.kind))
    .filter(Boolean);
  const index = {
    combined: [
      chain.id,
      chain.label,
      chain.summary ?? "",
      chain.tags?.join(" ") ?? "",
      ...templateOperators,
      ...templateKinds,
    ]
      .filter(Boolean)
      .map((value) => normalizeWorkflowTemplateChainSearchText(value))
      .join(" "),
    label: normalizeWorkflowTemplateChainSearchText(chain.label),
    id: normalizeWorkflowTemplateChainSearchText(chain.id),
    summary: normalizeWorkflowTemplateChainSearchText(chain.summary),
    tags,
    templateOperators,
    templateKinds,
  };
  templateChainSearchIndexCache.set(cacheKey, index);
  return index;
}

export function scoreWorkflowTemplateChainSearch(
  chain: WorkflowTemplateChainDefinition,
  query: string,
) {
  const normalizedQuery = normalizeWorkflowTemplateChainSearchText(query);
  if (!normalizedQuery) return 0;
  const tokens = tokenizeWorkflowTemplateChainSearchQuery(normalizedQuery);
  if (tokens.length === 0) return 0;
  const index = resolveWorkflowTemplateChainSearchIndex(chain);
  if (!tokens.every((token) => index.combined.includes(token))) return null;

  let score = 0;
  if (index.label === normalizedQuery) score += 170;
  else if (index.label.startsWith(normalizedQuery)) score += 130;
  else if (index.label.includes(normalizedQuery)) score += 95;

  if (index.id === normalizedQuery) score += 150;
  else if (index.id.startsWith(normalizedQuery)) score += 110;
  else if (index.id.includes(normalizedQuery)) score += 80;

  if (index.summary === normalizedQuery) score += 120;
  else if (index.summary.startsWith(normalizedQuery)) score += 90;
  else if (index.summary.includes(normalizedQuery)) score += 65;

  if (index.tags.includes(normalizedQuery)) score += 70;
  if (index.templateOperators.includes(normalizedQuery)) score += 75;
  if (index.templateKinds.includes(normalizedQuery)) score += 35;

  for (const token of tokens) {
    if (index.label.startsWith(token)) score += 16;
    else if (index.label.includes(token)) score += 10;

    if (index.id.startsWith(token)) score += 14;
    else if (index.id.includes(token)) score += 9;

    if (index.summary.startsWith(token)) score += 12;
    else if (index.summary.includes(token)) score += 8;

    score += index.tags.filter((tag) => tag.includes(token)).length * 7;
    score += index.templateOperators.filter((operatorId) => operatorId.includes(token)).length * 9;
    score += index.templateKinds.filter((kind) => kind.includes(token)).length * 4;
  }

  return score;
}
