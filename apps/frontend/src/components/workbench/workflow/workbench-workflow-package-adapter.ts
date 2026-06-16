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
import {
  builtInWorkflowSampleInputArtifacts,
  builtInWorkflowSampleInputSemanticTypes,
} from "@/components/workbench/workflow/workbench-workflow-sample-inputs";
import { collectWorkflowInputArtifactContractWarnings } from "@/components/workbench/workflow/workbench-workflow-fem-validation";
import { validateImportedWorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package-import-validation";

function resolveExportInputArtifactTexts(params: {
  workflow: WorkflowCatalogEntry;
  inputArtifactTexts?: Record<string, string>;
}) {
  if (params.inputArtifactTexts && Object.keys(params.inputArtifactTexts).length > 0) {
    return params.inputArtifactTexts;
  }

  const sampleInputs =
    builtInWorkflowSampleInputArtifacts(params.workflow.id) ??
    builtInWorkflowSampleInputArtifacts(params.workflow.graph?.id ?? "");
  if (!sampleInputs) return params.inputArtifactTexts;

  return Object.fromEntries(
    Object.entries(sampleInputs).map(([key, value]) => [key, JSON.stringify(value, null, 2)]),
  );
}

function resolveExportInputArtifactSemanticTypes(params: {
  workflow: WorkflowCatalogEntry;
}) {
  return (
    builtInWorkflowSampleInputSemanticTypes(params.workflow.id) ??
    builtInWorkflowSampleInputSemanticTypes(params.workflow.graph?.id ?? "") ??
    undefined
  );
}

function resolveExportInputArtifactContractWarnings(params: {
  workflow: WorkflowCatalogEntry;
  inputArtifactTexts?: Record<string, string>;
}) {
  const inputArtifactTexts = resolveExportInputArtifactTexts(params);
  return collectWorkflowInputArtifactContractWarnings({
    entryInputs: params.workflow.entry_inputs,
    inputArtifactTexts,
  });
}

export type ImportedWorkflowPayload = {
  diagnostics?: WorkflowPackageImportDiagnostic[];
  error?: string;
  graph: WorkflowGraphDefinition;
  importedPackage: WorkflowPackage | null;
  inputArtifactTexts?: Record<string, string>;
  templateChainPreferences?: WorkflowTemplateChainPreferenceSnapshot;
};

export type WorkflowPackageImportDiagnostic = {
  message: string;
  locate?:
    | { kind: "node"; nodeId: string }
    | { kind: "dataset"; datasetValueId?: string }
    | { kind: "package" };
};

export function buildExportedWorkflowPackage(params: {
  workflow: WorkflowCatalogEntry;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  templateChainPreferences?: WorkflowTemplateChainPreferenceSnapshot;
}) {
  return buildWorkflowPackage({
    ...params,
    inputArtifactTexts: resolveExportInputArtifactTexts(params),
    inputArtifactSemanticTypes: resolveExportInputArtifactSemanticTypes(params),
    inputArtifactContractWarnings: resolveExportInputArtifactContractWarnings(params),
  });
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
  if (importedPackage) {
    const diagnostics = validateImportedWorkflowPackage(importedPackage, graph);
    if (diagnostics.length > 0) {
      return {
        diagnostics,
        error: diagnostics[0]?.message,
        graph,
        importedPackage,
        inputArtifactTexts:
          importedPackage.workflow.input_artifact_texts ?? bundle?.input_artifact_texts,
        templateChainPreferences:
          importedPackage.workflow.template_chain_preferences ??
          bundle?.template_chain_preferences,
      };
    }
  }

  return {
    diagnostics: [],
    graph,
    importedPackage,
    inputArtifactTexts:
      importedPackage?.workflow.input_artifact_texts ?? bundle?.input_artifact_texts,
    templateChainPreferences:
      importedPackage?.workflow.template_chain_preferences ??
      bundle?.template_chain_preferences,
  };
}
