"use client";

import type { WorkflowCatalogEntry, WorkflowGraphDefinition } from "@/lib/api";
import { asWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-import";

const WORKBENCH_LOCAL_WORKFLOWS_KEY = "kyuubiki.workbench.workflowLibrary.v1";

export type StoredLocalWorkflow = {
  id: string;
  sourceWorkflowId: string;
  sourceWorkflowName?: string;
  name: string;
  summary: string;
  version: string;
  promotedAt: string;
  variantOfWorkflowId?: string;
  variantOfWorkflowName?: string;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
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

function readStoredLocalWorkflows(): StoredLocalWorkflow[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(WORKBENCH_LOCAL_WORKFLOWS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.flatMap((entry) => {
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
      return [
        {
          id: entry.id,
          sourceWorkflowId: entry.sourceWorkflowId,
          sourceWorkflowName: typeof entry.sourceWorkflowName === "string" ? entry.sourceWorkflowName : undefined,
          name: entry.name,
          summary: entry.summary,
          version: entry.version,
          promotedAt: entry.promotedAt,
          variantOfWorkflowId:
            typeof entry.variantOfWorkflowId === "string" ? entry.variantOfWorkflowId : undefined,
          variantOfWorkflowName:
            typeof entry.variantOfWorkflowName === "string" ? entry.variantOfWorkflowName : undefined,
          graph,
          inputArtifactTexts: asStringRecord(entry.inputArtifactTexts),
        },
      ];
    });
  } catch {
    return [];
  }
}

function writeStoredLocalWorkflows(records: StoredLocalWorkflow[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(WORKBENCH_LOCAL_WORKFLOWS_KEY, JSON.stringify(records));
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
}): StoredLocalWorkflow {
  const baseName = params.graph.name?.trim() || params.workflowName.trim() || params.graph.id;
  const nextId = buildLocalWorkflowId(baseName);
  const nextGraph = cloneWorkflowGraph(params.graph);
  nextGraph.id = nextId;
  nextGraph.name = `${baseName} Local`;
  nextGraph.version = "tamamono 1.4.0 local";

  const nextRecord: StoredLocalWorkflow = {
    id: nextId,
    sourceWorkflowId: params.sourceWorkflowId,
    sourceWorkflowName: params.workflowName,
    name: nextGraph.name,
    summary: `Local workflow promoted from ${params.workflowName}.`,
    version: "local",
    promotedAt: new Date().toISOString(),
    graph: nextGraph,
    inputArtifactTexts: params.inputArtifactTexts,
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

export function duplicateStoredLocalWorkflow(workflowId: string): StoredLocalWorkflow | null {
  const current = findStoredLocalWorkflow(workflowId);
  if (!current) return null;
  const baseName = `${current.name} Variant`;
  const nextId = buildLocalWorkflowId(baseName);
  const nextGraph = cloneWorkflowGraph(current.graph);
  nextGraph.id = nextId;
  nextGraph.name = baseName;
  nextGraph.version = "tamamono 1.4.0 local";
  const nextRecord: StoredLocalWorkflow = {
    id: nextId,
    sourceWorkflowId: current.sourceWorkflowId,
    sourceWorkflowName: current.sourceWorkflowName,
    name: baseName,
    summary: `Local workflow variant duplicated from ${current.name}.`,
    version: "local",
    promotedAt: new Date().toISOString(),
    variantOfWorkflowId: current.id,
    variantOfWorkflowName: current.name,
    graph: nextGraph,
    inputArtifactTexts: current.inputArtifactTexts,
  };
  const next = [nextRecord, ...readStoredLocalWorkflows()].slice(0, 40);
  writeStoredLocalWorkflows(next);
  return nextRecord;
}

export function findStoredLocalWorkflow(workflowId: string): StoredLocalWorkflow | null {
  return readStoredLocalWorkflows().find((entry) => entry.id === workflowId) ?? null;
}

export function buildStoredLocalWorkflowCatalogEntries(): WorkflowCatalogEntry[] {
  return listStoredLocalWorkflows().map((entry) => ({
    id: entry.id,
    name: entry.name,
    version: entry.version,
    summary: entry.summary,
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
    },
  }));
}
