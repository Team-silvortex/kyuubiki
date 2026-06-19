"use client";

import type {
  WorkflowGraphDefinition,
  WorkflowGraphNode,
} from "@/lib/api";
import type { WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";

function buildUniqueNodeId(
  preferredId: string,
  existingIds: Set<string>,
) {
  if (!existingIds.has(preferredId)) {
    existingIds.add(preferredId);
    return preferredId;
  }

  let suffix = 2;
  while (existingIds.has(`${preferredId}_${suffix}`)) {
    suffix += 1;
  }
  const uniqueId = `${preferredId}_${suffix}`;
  existingIds.add(uniqueId);
  return uniqueId;
}

export function applyTemplateChainNodeSemantics(
  graph: WorkflowGraphDefinition,
  chain: WorkflowTemplateChainDefinition,
  createdNodes: WorkflowGraphNode[],
) {
  if (
    chain.id !== "diagnostics_bundle_guard_report" &&
    chain.id !== "peak_diagnostics_bundle_report"
  )
    return;
  if (createdNodes.length === 0) return;

  const semanticIds =
    chain.id === "peak_diagnostics_bundle_report"
      ? [
          "extract_electrostatic_peak",
          "extract_thermal_peak",
          "extract_thermo_peak",
          "compose_diagnostics_bundle",
          "evaluate_diagnostics_guard",
          "compose_diagnostics_report",
          "export_diagnostics_markdown",
        ]
      : [
          "extract_electrostatic_diagnostics",
          "extract_thermal_diagnostics",
          "extract_thermo_diagnostics",
          "compose_diagnostics_bundle",
          "evaluate_diagnostics_guard",
          "compose_diagnostics_report",
          "export_diagnostics_markdown",
        ];
  const existingIds = new Set(
    graph.nodes
      .filter((node) => !createdNodes.includes(node))
      .map((node) => node.id),
  );

  createdNodes.forEach((node, index) => {
    const semanticId = semanticIds[index];
    if (!semanticId) return;
    node.id = buildUniqueNodeId(semanticId, existingIds);
  });
}
