"use client";

import type { WorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-adapter";
import type { WorkflowPackageResidualRecord } from "@/components/workbench/workflow/workbench-workflow-package-install-report";

export type WorkflowPackageMaintenanceSearchEntry = {
  at: string;
  kind: "scan" | "repair";
  lines: string[];
};

function normalizeWorkflowPackageDiagnosticsSearchText(value?: string | null) {
  return value?.trim().toLowerCase() ?? "";
}

function tokenizeWorkflowPackageDiagnosticsSearchQuery(query: string) {
  return normalizeWorkflowPackageDiagnosticsSearchText(query).split(/\s+/).filter(Boolean);
}

function scoreWorkflowPackageDiagnosticsSearch(values: string[], query: string) {
  const normalizedQuery = normalizeWorkflowPackageDiagnosticsSearchText(query);
  if (!normalizedQuery) return 0;
  const tokens = tokenizeWorkflowPackageDiagnosticsSearchQuery(normalizedQuery);
  if (tokens.length === 0) return 0;
  const normalizedValues = values.map((value) => normalizeWorkflowPackageDiagnosticsSearchText(value));
  const combined = normalizedValues.join(" ");
  if (!tokens.every((token) => combined.includes(token))) return null;

  let score = 0;
  for (const value of normalizedValues) {
    if (value === normalizedQuery) score += 120;
    else if (value.startsWith(normalizedQuery)) score += 80;
    else if (value.includes(normalizedQuery)) score += 45;
  }
  for (const token of tokens) {
    for (const value of normalizedValues) {
      if (value.startsWith(token)) score += 12;
      else if (value.includes(token)) score += 6;
    }
  }
  return score;
}

export function scoreWorkflowPackageImportDiagnosticSearch(
  diagnostic: WorkflowPackageImportDiagnostic,
  query: string,
) {
  return scoreWorkflowPackageDiagnosticsSearch(
    [
      diagnostic.message,
      diagnostic.locate?.kind ?? "",
      diagnostic.locate?.kind === "node" ? diagnostic.locate.nodeId : "",
      diagnostic.locate?.kind === "dataset" ? diagnostic.locate.datasetValueId ?? "" : "",
    ],
    query,
  );
}

export function scoreWorkflowPackageResidualSearch(
  residual: WorkflowPackageResidualRecord,
  query: string,
) {
  return scoreWorkflowPackageDiagnosticsSearch(
    [
      residual.id,
      residual.kind,
      residual.locate,
      residual.severity,
      residual.auto_fixable ? "auto" : "manual",
      residual.message,
    ],
    query,
  );
}

export function scoreWorkflowPackageMaintenanceEntrySearch(
  entry: WorkflowPackageMaintenanceSearchEntry,
  query: string,
) {
  return scoreWorkflowPackageDiagnosticsSearch([entry.kind, entry.at, ...entry.lines], query);
}
