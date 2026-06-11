"use client";

import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import type { WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";

export type WorkflowTemplateChainPackage = {
  format: "kyuubiki.workflow-template-chain-package";
  version: 1;
  package_id: string;
  name: string;
  summary?: string;
  tags?: string[];
  package_version?: string;
  exported_at: string;
  templates: WorkflowNodeTemplateSelection[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asTemplateSelections(
  value: unknown,
): WorkflowNodeTemplateSelection[] | null {
  if (!Array.isArray(value)) return null;
  const templates = value.filter(
    (entry): entry is WorkflowNodeTemplateSelection =>
      isRecord(entry) &&
      typeof entry.kind === "string" &&
      (typeof entry.operatorId === "string" || entry.operatorId === undefined),
  );
  return templates.length === value.length ? templates : null;
}

function asStringArray(value: unknown): string[] | undefined {
  if (!Array.isArray(value)) return undefined;
  const tags = value.filter((entry): entry is string => typeof entry === "string");
  return tags.length === 0 ? undefined : tags;
}

export function buildWorkflowTemplateChainPackage(
  chain: WorkflowTemplateChainDefinition,
): WorkflowTemplateChainPackage {
  return {
    format: "kyuubiki.workflow-template-chain-package",
    version: 1,
    package_id: chain.id,
    name: chain.label,
    summary: chain.summary,
    tags: chain.tags,
    package_version: chain.version ?? "1.0.0",
    exported_at: new Date().toISOString(),
    templates: chain.templates,
  };
}

export function asWorkflowTemplateChainPackage(
  value: unknown,
): WorkflowTemplateChainPackage | null {
  if (!isRecord(value)) return null;
  if (
    value.format !== "kyuubiki.workflow-template-chain-package" ||
    value.version !== 1 ||
    typeof value.package_id !== "string" ||
    typeof value.name !== "string"
  ) {
    return null;
  }
  const templates = asTemplateSelections(value.templates);
  if (!templates) return null;
  return {
    format: "kyuubiki.workflow-template-chain-package",
    version: 1,
    package_id: value.package_id,
    name: value.name,
    summary: typeof value.summary === "string" ? value.summary : undefined,
    tags: asStringArray(value.tags),
    package_version:
      typeof value.package_version === "string" ? value.package_version : undefined,
    exported_at:
      typeof value.exported_at === "string" ? value.exported_at : new Date().toISOString(),
    templates,
  };
}

export function packageToWorkflowTemplateChainDefinition(
  pkg: WorkflowTemplateChainPackage,
): Omit<WorkflowTemplateChainDefinition, "source"> {
  return {
    id: pkg.package_id,
    label: pkg.name,
    summary: pkg.summary,
    tags: pkg.tags,
    version: pkg.package_version,
    templates: pkg.templates,
  };
}
