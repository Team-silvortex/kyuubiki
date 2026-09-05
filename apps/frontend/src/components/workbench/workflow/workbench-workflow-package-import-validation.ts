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

function uniqueStrings(values: string[]) {
  return [...new Set(values)];
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

  if (importedPackage.workflow.id !== graph.id) {
    diagnostics.push({
      message: `Workflow id mismatch: expected ${importedPackage.workflow.id}, got ${graph.id}`,
      locate: { kind: "package" },
    });
  }

  const missingOperators = runtimeManifest.required_operator_ids.filter(
    (operatorId: string) => !operatorIds.has(operatorId),
  );
  if (missingOperators.length > 0) {
    diagnostics.push(...missingOperators.map((operatorId: string) => ({
      message: `Missing required operator: ${operatorId}`,
      locate: { kind: "package" as const },
    })));
  }

  const declaredOperatorIds = new Set(runtimeManifest.required_operator_ids);
  for (const operatorId of operatorIds) {
    if (declaredOperatorIds.has(operatorId)) continue;
    diagnostics.push({
      message: `Undeclared workflow operator: ${operatorId}`,
      locate: { kind: "package" },
    });
  }

  const fetchPlanCounts = new Map<string, number>();
  for (const entry of runtimeManifest.operator_fetch_plan) {
    fetchPlanCounts.set(entry.operator_id, (fetchPlanCounts.get(entry.operator_id) ?? 0) + 1);
    if (!declaredOperatorIds.has(entry.operator_id)) {
      diagnostics.push({
        message: `Unexpected operator fetch plan entry: ${entry.operator_id}`,
        locate: { kind: "package" },
      });
    }
  }
  for (const operatorId of uniqueStrings(runtimeManifest.required_operator_ids)) {
    const count = fetchPlanCounts.get(operatorId) ?? 0;
    if (count === 0) {
      diagnostics.push({
        message: `Missing operator fetch plan: ${operatorId}`,
        locate: { kind: "package" },
      });
    } else if (count > 1) {
      diagnostics.push({
        message: `Duplicate operator fetch plan: ${operatorId}`,
        locate: { kind: "package" },
      });
    }
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

  const declaredSampleInputIds = new Set(runtimeManifest.sample_input_node_ids);
  for (const nodeId of uniqueStrings(entryInputs.map((artifact) => artifact.node_id))) {
    if (declaredSampleInputIds.has(nodeId)) continue;
    diagnostics.push({
      message: `Undeclared sample input entry node: ${nodeId}`,
      locate: { kind: "node", nodeId },
    });
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

  if (graph.dataset_contract?.id && contractManifest.dataset_contract_id !== graph.dataset_contract.id) {
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

  const declaredDatasetValueIds = new Set(contractManifest.dataset_value_ids);
  for (const valueId of datasetValueIds) {
    if (declaredDatasetValueIds.has(valueId)) continue;
    diagnostics.push({
      message: `Undeclared dataset value: ${valueId}`,
      locate: { kind: "dataset", datasetValueId: valueId },
    });
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

  for (const entry of entryInputs) {
    if (matchesArtifactNode(contractManifest.entry_contracts, entry.node_id, entry.artifact_type)) continue;
    diagnostics.push({
      message: `Missing entry contract declaration at ${entry.node_id}:${entry.artifact_type}`,
      locate: { kind: "node", nodeId: entry.node_id },
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

  for (const entry of outputArtifacts) {
    if (matchesArtifactNode(contractManifest.output_contracts, entry.node_id, entry.artifact_type)) continue;
    diagnostics.push({
      message: `Missing output contract declaration at ${entry.node_id}:${entry.artifact_type}`,
      locate: { kind: "node", nodeId: entry.node_id },
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
