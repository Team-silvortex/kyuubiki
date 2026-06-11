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

function asTemplateChainPreferences(
  value: unknown,
): WorkflowTemplateChainPreferenceSnapshot | undefined {
  if (!isRecord(value)) return undefined;
  const favoriteChainIds = Array.isArray(value.favoriteChainIds)
    ? value.favoriteChainIds.filter(
        (entry): entry is string => typeof entry === "string",
      )
    : [];
  const favoriteChainAliases = asStringRecord(value.favoriteChainAliases) ?? {};
  if (favoriteChainIds.length === 0 && Object.keys(favoriteChainAliases).length === 0) {
    return undefined;
  }
  return { favoriteChainIds, favoriteChainAliases };
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
    graph: params.graph,
    input_artifact_texts: params.inputArtifactTexts,
    template_chain_preferences: params.templateChainPreferences,
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
  const graph = asWorkflowGraphDefinition(value.graph);
  if (!graph) return null;
  return {
    format: "kyuubiki.workflow-draft-bundle",
    version: 1,
    exported_at:
      typeof value.exported_at === "string" ? value.exported_at : new Date().toISOString(),
    graph,
    input_artifact_texts: asStringRecord(value.input_artifact_texts),
    template_chain_preferences: asTemplateChainPreferences(
      value.template_chain_preferences,
    ),
  };
}
