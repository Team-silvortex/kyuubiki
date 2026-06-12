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
import { builtInWorkflowSampleInputArtifacts } from "@/components/workbench/workflow/workbench-workflow-sample-inputs";

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

function matchesArtifactNode(
  artifacts: Array<{ node_id: string; artifact_type: string }>,
  nodeId: string,
  artifactType: string,
) {
  return artifacts.some(
    (artifact) => artifact.node_id === nodeId && artifact.artifact_type === artifactType,
  );
}

function validateImportedWorkflowPackage(
  importedPackage: WorkflowPackage,
  graph: WorkflowGraphDefinition,
) {
  const runtimeManifest = importedPackage.runtime_manifest;
  const contractManifest = importedPackage.contract_manifest;
  const operatorIds = new Set(
    graph.nodes
      .map((node) => node.operator_id)
      .filter((value): value is string => typeof value === "string"),
  );
  const datasetValueIds = new Set(
    graph.dataset_contract?.values.map((value) => value.id) ?? [],
  );
  const entryInputs = graph.entry_inputs ?? [];
  const outputArtifacts = graph.output_artifacts ?? [];
  const diagnostics: WorkflowPackageImportDiagnostic[] = [];

  const missingOperators = runtimeManifest.required_operator_ids.filter(
    (operatorId) => !operatorIds.has(operatorId),
  );
  if (missingOperators.length > 0) {
    diagnostics.push(...missingOperators.map((operatorId) => ({
      message: `Missing required operator: ${operatorId}`,
      locate: { kind: "package" as const },
    })));
  }

  const missingSampleInputs = runtimeManifest.sample_input_node_ids.filter(
    (nodeId) => !entryInputs.some((artifact) => artifact.node_id === nodeId),
  );
  if (missingSampleInputs.length > 0) {
    diagnostics.push(...missingSampleInputs.map((nodeId) => ({
      message: `Missing sample input entry node: ${nodeId}`,
      locate: { kind: "package" as const },
    })));
  }

  const invalidBridgeSeed = runtimeManifest.bridge_seed_summaries.find(
    (entry) =>
      !operatorIds.has(entry.operator_id) ||
      entry.node_count <= 0 ||
      entry.element_count <= 0,
  );
  if (invalidBridgeSeed) {
    diagnostics.push({
      message: `Invalid bridge seed summary for ${invalidBridgeSeed.operator_id}`,
      locate: {
        kind: "node",
        nodeId:
          graph.nodes.find((node) => node.operator_id === invalidBridgeSeed.operator_id)?.id ??
          invalidBridgeSeed.operator_id,
      },
    });
  }

  if (
    contractManifest.dataset_contract_id &&
    graph.dataset_contract?.id &&
    contractManifest.dataset_contract_id !== graph.dataset_contract.id
  ) {
    diagnostics.push({
      message: `Dataset contract id mismatch: expected ${contractManifest.dataset_contract_id}, got ${graph.dataset_contract.id}`,
      locate: { kind: "dataset" },
    });
  }

  const missingDatasetValues = contractManifest.dataset_value_ids.filter(
    (valueId) => !datasetValueIds.has(valueId),
  );
  if (missingDatasetValues.length > 0) {
    diagnostics.push(...missingDatasetValues.map((valueId) => ({
      message: `Missing dataset value: ${valueId}`,
      locate: { kind: "dataset" as const, datasetValueId: valueId },
    })));
  }

  const invalidEntryContract = contractManifest.entry_contracts.find(
    (entry) =>
      !matchesArtifactNode(entryInputs, entry.node_id, entry.artifact_type) ||
      (entry.dataset_value && !datasetValueIds.has(entry.dataset_value)),
  );
  if (invalidEntryContract) {
    diagnostics.push({
      message: `Entry contract mismatch at ${invalidEntryContract.node_id}:${invalidEntryContract.artifact_type}`,
      locate: { kind: "node", nodeId: invalidEntryContract.node_id },
    });
  }

  const invalidOutputContract = contractManifest.output_contracts.find(
    (entry) =>
      !matchesArtifactNode(outputArtifacts, entry.node_id, entry.artifact_type) ||
      (entry.dataset_value && !datasetValueIds.has(entry.dataset_value)),
  );
  if (invalidOutputContract) {
    diagnostics.push({
      message: `Output contract mismatch at ${invalidOutputContract.node_id}:${invalidOutputContract.artifact_type}`,
      locate: { kind: "node", nodeId: invalidOutputContract.node_id },
    });
  }

  return diagnostics;
}

export function buildExportedWorkflowPackage(params: {
  workflow: WorkflowCatalogEntry;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  templateChainPreferences?: WorkflowTemplateChainPreferenceSnapshot;
}) {
  return buildWorkflowPackage({
    ...params,
    inputArtifactTexts: resolveExportInputArtifactTexts(params),
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
