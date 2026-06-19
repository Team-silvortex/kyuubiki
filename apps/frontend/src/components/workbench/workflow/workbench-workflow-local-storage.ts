"use client";

import type { WorkflowCatalogEntry, WorkflowGraphDefinition } from "@/lib/api";
import { asWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-import";
import { summarizeWorkflowInputArtifactContractHealth } from "@/components/workbench/workflow/workbench-workflow-fem-validation";
import { buildWorkflowPackageSearchIndex } from "@/components/workbench/workflow/workbench-workflow-package";

export const WORKBENCH_LOCAL_WORKFLOWS_KEY = "kyuubiki.workbench.workflowLibrary.v1";

export type StoredLocalWorkflow = {
  id: string;
  sourceWorkflowId: string;
  sourceWorkflowName?: string;
  name: string;
  summary: string;
  notes?: string;
  version: string;
  promotedAt: string;
  variantOfWorkflowId?: string;
  variantOfWorkflowName?: string;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  tags?: string[];
  importedFromPackageId?: string;
  importedFromPackageVersion?: string;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asStringRecord(value: unknown): Record<string, string> | undefined {
  if (!isRecord(value)) return undefined;
  return Object.fromEntries(
    Object.entries(value).filter(
      ([key, entryValue]) => typeof key === "string" && typeof entryValue === "string",
    ),
  ) as Record<string, string>;
}

function stripLegacyLocalWorkflowInputs(records: StoredLocalWorkflow[]) {
  const sanitized = records.map(({ inputArtifactTexts: _inputArtifactTexts, ...entry }) => entry);
  return sanitized as StoredLocalWorkflow[];
}

function readStoredLocalWorkflows(): StoredLocalWorkflow[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(WORKBENCH_LOCAL_WORKFLOWS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    let hadLegacyInputs = false;
    const records = parsed.flatMap((entry) => {
      if (!isRecord(entry)) return [];
      if (
        typeof entry.id !== "string" ||
        typeof entry.sourceWorkflowId !== "string" ||
        typeof entry.name !== "string" ||
        typeof entry.summary !== "string" ||
        typeof entry.version !== "string" ||
        typeof entry.promotedAt !== "string"
      ) {
        return [];
      }
      const graph = asWorkflowGraphDefinition(entry.graph);
      if (!graph) return [];
      if (asStringRecord(entry.inputArtifactTexts)) {
        hadLegacyInputs = true;
      }
      return [
        {
          id: entry.id,
          sourceWorkflowId: entry.sourceWorkflowId,
          sourceWorkflowName: typeof entry.sourceWorkflowName === "string" ? entry.sourceWorkflowName : undefined,
          name: entry.name,
          summary: entry.summary,
          notes: typeof entry.notes === "string" ? entry.notes : undefined,
          version: entry.version,
          promotedAt: entry.promotedAt,
          variantOfWorkflowId:
            typeof entry.variantOfWorkflowId === "string" ? entry.variantOfWorkflowId : undefined,
          variantOfWorkflowName:
            typeof entry.variantOfWorkflowName === "string" ? entry.variantOfWorkflowName : undefined,
          graph,
          tags:
            Array.isArray(entry.tags) && entry.tags.every((value) => typeof value === "string")
              ? (entry.tags as string[])
              : undefined,
          importedFromPackageId:
            typeof entry.importedFromPackageId === "string"
              ? entry.importedFromPackageId
              : undefined,
          importedFromPackageVersion:
            typeof entry.importedFromPackageVersion === "string"
              ? entry.importedFromPackageVersion
              : undefined,
        },
      ];
    });
    if (hadLegacyInputs) {
      writeStoredLocalWorkflows(records);
    }
    return records;
  } catch {
    return [];
  }
}

function writeStoredLocalWorkflows(records: StoredLocalWorkflow[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(
    WORKBENCH_LOCAL_WORKFLOWS_KEY,
    JSON.stringify(stripLegacyLocalWorkflowInputs(records)),
  );
}

function buildLocalWorkflowId(baseName: string) {
  const slug = baseName
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "") || "workflow";
  return `workflow.local.${slug}.${Date.now()}`;
}

function cloneWorkflowGraph(graph: WorkflowGraphDefinition): WorkflowGraphDefinition {
  return JSON.parse(JSON.stringify(graph)) as WorkflowGraphDefinition;
}

export function listStoredLocalWorkflows(): StoredLocalWorkflow[] {
  return readStoredLocalWorkflows().sort((left, right) => right.promotedAt.localeCompare(left.promotedAt));
}

export function saveStoredLocalWorkflow(params: {
  sourceWorkflowId: string;
  workflowName: string;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  summary?: string;
  notes?: string;
  tags?: string[];
  importedFromPackageId?: string;
  importedFromPackageVersion?: string;
  sourceWorkflowName?: string;
  variantOfWorkflowId?: string;
  variantOfWorkflowName?: string;
}): StoredLocalWorkflow {
  const baseName = params.graph.name?.trim() || params.workflowName.trim() || params.graph.id;
  const nextId = buildLocalWorkflowId(baseName);
  const nextGraph = cloneWorkflowGraph(params.graph);
  nextGraph.id = nextId;
  nextGraph.name = `${baseName} Local`;
  nextGraph.version = "tamamono 1.8.0 local";

  const nextRecord: StoredLocalWorkflow = {
    id: nextId,
    sourceWorkflowId: params.sourceWorkflowId,
    sourceWorkflowName: params.sourceWorkflowName ?? params.workflowName,
    name: nextGraph.name,
    summary: params.summary?.trim() || `Local workflow promoted from ${params.workflowName}.`,
    notes: params.notes?.trim() ?? "",
    version: "local",
    promotedAt: new Date().toISOString(),
    variantOfWorkflowId: params.variantOfWorkflowId,
    variantOfWorkflowName: params.variantOfWorkflowName,
    graph: nextGraph,
    tags: params.tags,
    importedFromPackageId: params.importedFromPackageId,
    importedFromPackageVersion: params.importedFromPackageVersion,
  };
  const next = [nextRecord, ...readStoredLocalWorkflows()].slice(0, 40);
  writeStoredLocalWorkflows(next);
  return nextRecord;
}

export function removeStoredLocalWorkflow(workflowId: string) {
  writeStoredLocalWorkflows(readStoredLocalWorkflows().filter((entry) => entry.id !== workflowId));
}

export function renameStoredLocalWorkflow(workflowId: string, nextName: string) {
  const trimmedName = nextName.trim();
  if (!trimmedName) return;
  writeStoredLocalWorkflows(
    readStoredLocalWorkflows().map((entry) => {
      if (entry.id !== workflowId) return entry;
      const nextGraph = cloneWorkflowGraph(entry.graph);
      nextGraph.name = trimmedName;
      return {
        ...entry,
        name: trimmedName,
        graph: nextGraph,
      };
    }),
  );
}

export function updateStoredLocalWorkflowMetadata(
  workflowId: string,
  params: { summary: string; notes: string },
) {
  writeStoredLocalWorkflows(
    readStoredLocalWorkflows().map((entry) =>
      entry.id === workflowId
        ? {
            ...entry,
            summary: params.summary.trim(),
            notes: params.notes.trim(),
          }
        : entry,
    ),
  );
}

export function duplicateStoredLocalWorkflow(workflowId: string): StoredLocalWorkflow | null {
  const current = findStoredLocalWorkflow(workflowId);
  if (!current) return null;
  const baseName = `${current.name} Variant`;
  const nextId = buildLocalWorkflowId(baseName);
  const nextGraph = cloneWorkflowGraph(current.graph);
  nextGraph.id = nextId;
  nextGraph.name = baseName;
  nextGraph.version = "tamamono 1.8.0 local";
  const nextRecord: StoredLocalWorkflow = {
    id: nextId,
    sourceWorkflowId: current.sourceWorkflowId,
    sourceWorkflowName: current.sourceWorkflowName,
    name: baseName,
    summary: `Local workflow variant duplicated from ${current.name}.`,
    notes: current.notes,
    version: "local",
    promotedAt: new Date().toISOString(),
    variantOfWorkflowId: current.id,
    variantOfWorkflowName: current.name,
    graph: nextGraph,
    tags: current.tags,
    importedFromPackageId: current.importedFromPackageId,
    importedFromPackageVersion: current.importedFromPackageVersion,
  };
  const next = [nextRecord, ...readStoredLocalWorkflows()].slice(0, 40);
  writeStoredLocalWorkflows(next);
  return nextRecord;
}

export function findStoredLocalWorkflow(workflowId: string): StoredLocalWorkflow | null {
  return readStoredLocalWorkflows().find((entry) => entry.id === workflowId) ?? null;
}

export function buildStoredLocalWorkflowCatalogEntries(): WorkflowCatalogEntry[] {
  return listStoredLocalWorkflows().map((entry) => {
    const contractHealth = summarizeWorkflowInputArtifactContractHealth({
      entryInputs: entry.graph.entry_inputs ?? [],
      inputArtifactTexts: entry.inputArtifactTexts,
    });
    const searchIndex = buildWorkflowPackageSearchIndex({
      graph: entry.graph,
      tags: [...(entry.tags ?? []), ...contractHealth.tags],
    });

    return {
      id: entry.id,
      name: entry.name,
      version: entry.version,
      summary: entry.summary,
      domains: searchIndex.domains,
      capability_tags: searchIndex.capability_tags,
      graph: cloneWorkflowGraph(entry.graph),
      entry_inputs: entry.graph.entry_inputs ?? [],
      output_artifacts: entry.graph.output_artifacts ?? [],
      local: {
        storage_id: entry.id,
        source_workflow_id: entry.sourceWorkflowId,
        source_workflow_name: entry.sourceWorkflowName,
        input_artifact_texts: entry.inputArtifactTexts,
        promoted_at: entry.promotedAt,
        variant_of_workflow_id: entry.variantOfWorkflowId,
        variant_of_workflow_name: entry.variantOfWorkflowName,
        notes: entry.notes,
        tags: [...(entry.tags ?? []), ...contractHealth.tags],
        imported_from_package_id: entry.importedFromPackageId,
        imported_from_package_version: entry.importedFromPackageVersion,
      },
    };
  });
}
