"use client";

export type WorkflowSearchWeightedField = {
  value: string;
  exact: number;
  prefix: number;
  contains: number;
};

export function normalizeWorkflowSearchText(value?: string | null) {
  return value?.trim().toLowerCase() ?? "";
}

export function tokenizeWorkflowSearchQuery(query: string) {
  return normalizeWorkflowSearchText(query).split(/[^a-z0-9]+/u).filter(Boolean);
}

export function includesAllWorkflowSearchTokens(
  combined: string,
  tokens: string[],
) {
  return tokens.every((token) => combined.includes(token));
}

export function scoreWorkflowSearchFields(
  weightedFields: WorkflowSearchWeightedField[],
  exactFields: string[],
  query: string,
  tokens: string[],
) {
  let score = 0;
  for (const field of weightedFields) {
    if (field.value === query) score += field.exact;
    else if (field.value.startsWith(query)) score += field.prefix;
    else if (field.value.includes(query)) score += field.contains;
  }
  for (const token of tokens) {
    for (const field of weightedFields) {
      if (field.value.startsWith(token)) score += Math.max(1, Math.floor(field.prefix / 8));
      else if (field.value.includes(token)) score += Math.max(1, Math.floor(field.contains / 8));
    }
    for (const value of exactFields) {
      if (value.includes(token)) score += 1;
    }
  }
  return score;
}

export function findWorkflowSearchMatches(
  values: string[],
  tokens: string[],
) {
  return tokens.every((token) => values.some((value) => value.includes(token)));
}

export function buildWorkflowSearchSuggestions<T>(
  items: T[],
  query: string,
  resolve: (
    item: T,
    normalizedQuery: string,
    tokens: string[],
  ) => { score: number | null; matchSummary: string[] },
  sortLabel: (item: T) => string,
) {
  const normalizedQuery = normalizeWorkflowSearchText(query);
  if (!normalizedQuery) {
    return items.map((item) => ({
      item,
      score: 0,
      matchSummary: [] as string[],
    }));
  }
  return items
    .flatMap((item) => {
      const resolved = resolve(item, normalizedQuery, tokenizeWorkflowSearchQuery(normalizedQuery));
      if (resolved.score == null) return [];
      return [{ item, score: resolved.score, matchSummary: resolved.matchSummary }];
    })
    .sort((left, right) => {
      const scoreDiff = right.score - left.score;
      if (scoreDiff !== 0) return scoreDiff;
      return sortLabel(left.item).localeCompare(sortLabel(right.item));
    });
}
