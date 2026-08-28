"use client";

import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import type {
  WorkflowTemplateChainConnection,
  WorkflowTemplateChainDefinition,
} from "@/components/workbench/workflow/workbench-workflow-template-chain-library";
import {
  asWorkflowTemplateChainConnections,
  asWorkflowTemplateChainSelections,
} from "@/components/workbench/workflow/workbench-workflow-template-chain-contract";

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
  connections?: WorkflowTemplateChainConnection[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
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
    tags: chain.tags ? [...chain.tags] : undefined,
    package_version: chain.version ?? "1.0.0",
    exported_at: new Date().toISOString(),
    templates: structuredClone(chain.templates),
    connections: chain.connections ? structuredClone(chain.connections) : undefined,
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
    value.package_id.trim().length === 0 ||
    typeof value.name !== "string" ||
    value.name.trim().length === 0 ||
    typeof value.exported_at !== "string" ||
    !Number.isFinite(Date.parse(value.exported_at))
  ) {
    return null;
  }
  const templates = asWorkflowTemplateChainSelections(value.templates);
  if (!templates) return null;
  const connections = asWorkflowTemplateChainConnections(value.connections, templates.length);
  if (connections === null) return null;
  return {
    format: "kyuubiki.workflow-template-chain-package",
    version: 1,
    package_id: value.package_id,
    name: value.name,
    summary: typeof value.summary === "string" ? value.summary : undefined,
    tags: asStringArray(value.tags),
    package_version:
      typeof value.package_version === "string" ? value.package_version : undefined,
    exported_at: new Date(Date.parse(value.exported_at)).toISOString(),
    templates,
    connections: connections as WorkflowTemplateChainConnection[] | undefined,
  };
}

export function packageToWorkflowTemplateChainDefinition(
  pkg: WorkflowTemplateChainPackage,
): Omit<WorkflowTemplateChainDefinition, "source"> {
  return {
    id: pkg.package_id,
    label: pkg.name,
    summary: pkg.summary,
    tags: pkg.tags ? [...pkg.tags] : undefined,
    version: pkg.package_version,
    templates: structuredClone(pkg.templates),
    connections: pkg.connections ? structuredClone(pkg.connections) : undefined,
  };
}
