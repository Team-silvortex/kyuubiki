"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";
import type { StoredWorkflowSnapshotSummary } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";

export function countWorkflowContractWarnings(warnings?: Record<string, string[]>) {
  return Object.values(warnings ?? {}).reduce((total, lines) => total + lines.length, 0);
}

export function findWorkflowContractHealthTag(tags?: string[]) {
  return tags?.find((entry) => entry.startsWith("contract_health:")) ?? null;
}

export function formatWorkflowContractHealthLabel(tags?: string[]) {
  const value = findWorkflowContractHealthTag(tags)?.split(":")[1] ?? "";
  if (value === "clean") return "clean";
  if (value === "manageable") return "manageable";
  if (value === "review") return "needs review";
  return value || null;
}

export function elevateWorkflowContractHealthLabel(
  health: string | null,
  recentRunStatus?: string | null,
) {
  return recentRunStatus === "failed" || recentRunStatus === "cancelled" ? "needs review" : health;
}

export function hasWorkflowContractHealthTag(
  workflow: WorkflowCatalogEntry,
  expected: "clean" | "manageable" | "review",
) {
  const tags = [...(workflow.capability_tags ?? []), ...(workflow.local?.tags ?? [])];
  return tags.includes(`contract_health:${expected}`);
}

export function rankWorkflowContractHealth(workflow: WorkflowCatalogEntry) {
  if (hasWorkflowContractHealthTag(workflow, "clean")) return 0;
  if (hasWorkflowContractHealthTag(workflow, "manageable")) return 1;
  if (hasWorkflowContractHealthTag(workflow, "review")) return 2;
  return 1;
}

export function buildWorkflowContractHealthRunMessage(workflowName: string, tags?: string[]) {
  const health = formatWorkflowContractHealthLabel(tags);
  if (health === "needs review") return `${workflowName}: contract health needs review before or during run.`;
  return health ? `${workflowName}: contract health ${health}.` : null;
}

export function buildWorkflowContractHealthRunFeedbackMessage(
  workflowName: string,
  tags?: string[],
  recentRunStatus?: string | null,
) {
  const health = elevateWorkflowContractHealthLabel(
    formatWorkflowContractHealthLabel(tags),
    recentRunStatus,
  );
  if (health === "needs review") {
    return recentRunStatus === "failed" || recentRunStatus === "cancelled"
      ? `${workflowName}: recent run failed, contract health escalated to needs review.`
      : `${workflowName}: contract health needs review before or during run.`;
  }
  return health ? `${workflowName}: contract health ${health}.` : null;
}

export function buildWorkflowDraftContractWarningMessage(warningCount: number) {
  if (warningCount > 3) return `Draft run starting with ${warningCount} contract warnings; needs review.`;
  if (warningCount > 0) return `Draft run starting with ${warningCount} contract warning(s).`;
  return null;
}

export function formatWorkflowContractHealthSummary(warnings?: Record<string, string[]>) {
  const warningCount = countWorkflowContractWarnings(warnings);
  if (!warnings) return "--";
  if (warningCount === 0) return "clean";
  if (warningCount <= 3) return `${warningCount} warning(s), manageable`;
  return `${warningCount} warning(s), needs review`;
}

export function formatWorkflowDynamicReviewState(params: {
  warnings?: Record<string, string[]>;
  recentRunStatus?: string | null;
}) {
  const staticHealth = formatWorkflowContractHealthSummary(params.warnings);
  if (params.recentRunStatus === "failed" || params.recentRunStatus === "cancelled") {
    return `needs review (escalated after ${params.recentRunStatus})`;
  }
  return staticHealth;
}

export function readSnapshotContractWarningCount(snapshot: StoredWorkflowSnapshotSummary) {
  const line = snapshot.summary.find((entry) => entry.startsWith("contract warnings:"));
  if (!line) return null;
  const count = Number(line.split(":")[1]?.trim() ?? "");
  return Number.isFinite(count) ? count : null;
}

export function formatSnapshotContractTrend(
  current: StoredWorkflowSnapshotSummary,
  previous?: StoredWorkflowSnapshotSummary,
) {
  const currentCount = readSnapshotContractWarningCount(current);
  const previousCount = previous ? readSnapshotContractWarningCount(previous) : null;
  if (currentCount === null) return null;
  if (previousCount === null) return `contract warnings ${currentCount}`;
  const delta = currentCount - previousCount;
  if (delta === 0) return `contract warnings unchanged at ${currentCount}`;
  return delta < 0
    ? `contract warnings improved by ${Math.abs(delta)} to ${currentCount}`
    : `contract warnings increased by ${delta} to ${currentCount}`;
}

export function buildImportedWorkflowContractHealthMessage(params: {
  importSuccessLabel: string;
  currentWarnings?: Record<string, string[]>;
  importedWarnings?: Record<string, string[]>;
  hasImportedPackage: boolean;
}) {
  const currentCount = countWorkflowContractWarnings(params.currentWarnings);
  const importedCount = countWorkflowContractWarnings(params.importedWarnings);
  if (!params.hasImportedPackage) return params.importSuccessLabel;
  if (importedCount > currentCount) return `${params.importSuccessLabel} mounted package is dirtier than current draft.`;
  if (importedCount < currentCount) return `${params.importSuccessLabel} mounted package is cleaner than current draft.`;
  return `${params.importSuccessLabel} mounted package matches current contract health.`;
}
