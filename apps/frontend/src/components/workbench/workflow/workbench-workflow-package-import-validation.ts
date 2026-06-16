"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";
import type { WorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package";
import type { WorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-adapter";

function matchesArtifactNode(
  artifacts: Array<{ node_id: string; artifact_type: string }>,
  nodeId: string,
  artifactType: string,
) {
  return artifacts.some(
    (artifact) => artifact.node_id === nodeId && artifact.artifact_type === artifactType,
  );
}

export function validateImportedWorkflowPackage(
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
    (operatorId: string) => !operatorIds.has(operatorId),
  );
  if (missingOperators.length > 0) {
    diagnostics.push(...missingOperators.map((operatorId: string) => ({
      message: `Missing required operator: ${operatorId}`,
      locate: { kind: "package" as const },
    })));
  }

  const missingSampleInputs = runtimeManifest.sample_input_node_ids.filter(
    (nodeId: string) => !entryInputs.some((artifact) => artifact.node_id === nodeId),
  );
  if (missingSampleInputs.length > 0) {
    diagnostics.push(...missingSampleInputs.map((nodeId: string) => ({
      message: `Missing sample input entry node: ${nodeId}`,
      locate: { kind: "package" as const },
    })));
  }

  const invalidBridgeSeed = runtimeManifest.bridge_seed_summaries.find(
    (entry: WorkflowPackage["runtime_manifest"]["bridge_seed_summaries"][number]) =>
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
    (valueId: string) => !datasetValueIds.has(valueId),
  );
  if (missingDatasetValues.length > 0) {
    diagnostics.push(...missingDatasetValues.map((valueId: string) => ({
      message: `Missing dataset value: ${valueId}`,
      locate: { kind: "dataset" as const, datasetValueId: valueId },
    })));
  }

  const invalidEntryContract = contractManifest.entry_contracts.find(
    (entry: WorkflowPackage["contract_manifest"]["entry_contracts"][number]) =>
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
    (entry: WorkflowPackage["contract_manifest"]["output_contracts"][number]) =>
      !matchesArtifactNode(outputArtifacts, entry.node_id, entry.artifact_type) ||
      (entry.dataset_value && !datasetValueIds.has(entry.dataset_value)),
  );
  if (invalidOutputContract) {
    diagnostics.push({
      message: `Output contract mismatch at ${invalidOutputContract.node_id}:${invalidOutputContract.artifact_type}`,
      locate: { kind: "node", nodeId: invalidOutputContract.node_id },
    });
  }

  const storedContractWarnings: Record<string, string[]> | undefined =
    importedPackage.workflow.input_artifact_contract_warnings;
  if (storedContractWarnings) {
    diagnostics.push(
      ...Object.entries(storedContractWarnings).flatMap(([nodeId, lines]: [string, string[]]) =>
        lines.map((line: string) => ({
          message: `Stored export contract warning at ${nodeId}: ${line}`,
          locate: entryInputs.some((artifact) => artifact.node_id === nodeId)
            ? ({ kind: "node", nodeId } as const)
            : ({ kind: "package" } as const),
        })),
      ),
    );
  }

  return diagnostics;
}
