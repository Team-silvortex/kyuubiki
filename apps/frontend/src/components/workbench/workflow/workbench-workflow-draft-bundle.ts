"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";
import { asWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-import";
import type { WorkflowTemplateChainPreferenceSnapshot } from "@/components/workbench/workflow/workbench-workflow-template-chain-storage";

export type WorkflowDraftBundle = {
  format: "kyuubiki.workflow-draft-bundle";
  version: 1;
  exported_at: string;
  graph: WorkflowGraphDefinition;
  input_artifact_texts?: Record<string, string>;
  template_chain_preferences?: WorkflowTemplateChainPreferenceSnapshot;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringRecord(value: unknown): value is Record<string, string> {
  return isRecord(value) && Object.values(value).every((entry) => typeof entry === "string");
}

function asTemplateChainPreferences(
  value: unknown,
): WorkflowTemplateChainPreferenceSnapshot | null {
  if (!isRecord(value)) return null;
  if (!Array.isArray(value.favoriteChainIds) || !value.favoriteChainIds.every((entry) => typeof entry === "string")) return null;
  if (!isStringRecord(value.favoriteChainAliases)) return null;
  return {
    favoriteChainIds: [...value.favoriteChainIds],
    favoriteChainAliases: { ...value.favoriteChainAliases },
  };
}

export function buildWorkflowDraftBundle(params: {
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  templateChainPreferences?: WorkflowTemplateChainPreferenceSnapshot;
}): WorkflowDraftBundle {
  return {
    format: "kyuubiki.workflow-draft-bundle",
    version: 1,
    exported_at: new Date().toISOString(),
    graph: structuredClone(params.graph),
    input_artifact_texts: params.inputArtifactTexts
      ? { ...params.inputArtifactTexts }
      : undefined,
    template_chain_preferences: params.templateChainPreferences
      ? structuredClone(params.templateChainPreferences)
      : undefined,
  };
}

export function asWorkflowDraftBundle(value: unknown): WorkflowDraftBundle | null {
  if (!isRecord(value)) return null;
  if (
    value.format !== "kyuubiki.workflow-draft-bundle" ||
    value.version !== 1
  ) {
    return null;
  }
  if (typeof value.exported_at !== "string" || !Number.isFinite(Date.parse(value.exported_at))) return null;
  const graph = asWorkflowGraphDefinition(value.graph);
  if (!graph) return null;
  if (value.input_artifact_texts !== undefined && !isStringRecord(value.input_artifact_texts)) return null;
  const templateChainPreferences = value.template_chain_preferences === undefined
    ? undefined
    : asTemplateChainPreferences(value.template_chain_preferences);
  if (templateChainPreferences === null) return null;
  return {
    format: "kyuubiki.workflow-draft-bundle",
    version: 1,
    exported_at: value.exported_at,
    graph: structuredClone(graph),
    input_artifact_texts: value.input_artifact_texts
      ? { ...value.input_artifact_texts }
      : undefined,
    template_chain_preferences: templateChainPreferences ?? undefined,
  };
}
