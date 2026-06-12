"use client";

import type { WorkflowCatalogEntry, WorkflowGraphDefinition } from "@/lib/api";
import { asWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-import";
import { asWorkflowDraftBundle } from "@/components/workbench/workflow/workbench-workflow-draft-bundle";
import {
  asWorkflowPackage,
  buildWorkflowPackage,
  type WorkflowPackage,
} from "@/components/workbench/workflow/workbench-workflow-package";
import type { WorkflowTemplateChainPreferenceSnapshot } from "@/components/workbench/workflow/workbench-workflow-template-chain-storage";

export type ImportedWorkflowPayload = {
  graph: WorkflowGraphDefinition;
  importedPackage: WorkflowPackage | null;
  inputArtifactTexts?: Record<string, string>;
  templateChainPreferences?: WorkflowTemplateChainPreferenceSnapshot;
};

export function buildExportedWorkflowPackage(params: {
  workflow: WorkflowCatalogEntry;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  templateChainPreferences?: WorkflowTemplateChainPreferenceSnapshot;
}) {
  return buildWorkflowPackage(params);
}

export function buildPromotedWorkflowParams(params: {
  workflow: WorkflowCatalogEntry;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  importedPackage: WorkflowPackage | null;
}) {
  const { workflow, importedPackage } = params;

  return {
    sourceWorkflowId:
      importedPackage?.workflow.source_workflow_id ??
      workflow.local?.source_workflow_id ??
      workflow.id,
    workflowName: workflow.name,
    graph: params.graph,
    inputArtifactTexts: params.inputArtifactTexts,
    sourceWorkflowName:
      importedPackage?.workflow.source_workflow_name ??
      workflow.local?.source_workflow_name,
    summary: importedPackage?.summary,
    notes: importedPackage?.workflow.notes,
    tags: importedPackage?.tags,
    importedFromPackageId: importedPackage?.package_id,
    importedFromPackageVersion: importedPackage?.package_version,
    variantOfWorkflowId: importedPackage?.workflow.variant_of_workflow_id,
    variantOfWorkflowName: importedPackage?.workflow.variant_of_workflow_name,
  };
}

export function parseImportedWorkflowPayload(json: unknown): ImportedWorkflowPayload | null {
  const importedPackage = asWorkflowPackage(json);
  const bundle = asWorkflowDraftBundle(json);
  const graph =
    importedPackage?.workflow.graph ?? bundle?.graph ?? asWorkflowGraphDefinition(json);

  if (!graph) return null;

  return {
    graph,
    importedPackage,
    inputArtifactTexts:
      importedPackage?.workflow.input_artifact_texts ?? bundle?.input_artifact_texts,
    templateChainPreferences:
      importedPackage?.workflow.template_chain_preferences ??
      bundle?.template_chain_preferences,
  };
}
