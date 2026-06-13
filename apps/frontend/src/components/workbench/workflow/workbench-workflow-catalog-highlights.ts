"use client";

import type { WorkflowCatalogEntry, WorkflowGraphDefinition } from "@/lib/api";

export const PINNED_WORKFLOW_IDS = [
  "workflow.electrostatic-plane-quad-2d",
  "workflow.electrostatic-plane-quad-field-statistics-json",
  "workflow.electrostatic-preheat-guard-markdown",
  "workflow.electrostatic-preheat-guard-heat-json",
  "workflow.electrostatic-preheat-guard-heat-thermo-json",
  "workflow.electrostatic-plane-triangle-summary-json",
  "workflow.electrostatic-triangle-preheat-guard-markdown",
  "workflow.electrostatic-triangle-preheat-guard-heat-json",
  "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
  "workflow.electrostatic-to-heat-quad-2d",
  "workflow.electrostatic-quad-triangle-compare-json",
  "workflow.electrostatic-to-heat-triangle-2d",
  "workflow.electrostatic-heat-thermo-summary-json",
  "workflow.electrostatic-heat-thermo-triangle-summary-json",
  "workflow.heat-to-thermo-quad-2d",
  "workflow.plane-quad-2d-summary-json",
] as const;

export type WorkflowCatalogHighlight = {
  label: string;
  value: string;
};

function includesTag(tags: string[], value: string) {
  return tags.includes(value);
}

function detectShape(tags: string[]) {
  if (includesTag(tags, "triangle")) return "triangle 2d";
  if (includesTag(tags, "quad")) return "quad 2d";
  if (includesTag(tags, "3d")) return "3d";
  if (includesTag(tags, "2d")) return "2d";
  if (includesTag(tags, "1d")) return "1d";
  return null;
}

function detectChain(tags: string[]) {
  const hasElectrostatic = includesTag(tags, "electrostatic");
  const hasHeat = includesTag(tags, "heat");
  const hasThermal = includesTag(tags, "thermal") || includesTag(tags, "thermo_mechanical");

  if (hasElectrostatic && hasHeat && hasThermal) return "electrostatic -> heat -> thermo";
  if (hasElectrostatic && hasHeat) return "electrostatic -> heat";
  if (hasHeat && hasThermal) return "heat -> thermo";
  if (includesTag(tags, "summary")) return "solve -> summary";
  return null;
}

function detectMaturity(tags: string[]) {
  if (includesTag(tags, "workflow_bridge") && includesTag(tags, "summary")) return "coupled path";
  if (includesTag(tags, "workflow_bridge")) return "bridge-ready";
  if (includesTag(tags, "default_template")) return "baseline";
  return "catalog";
}

function countKind(nodes: WorkflowGraphDefinition["nodes"] | undefined, kind: string) {
  return nodes?.filter((node) => node.kind === kind).length ?? 0;
}

export function deriveWorkflowCatalogHighlights(
  workflow: WorkflowCatalogEntry,
): WorkflowCatalogHighlight[] {
  const tags = workflow.capability_tags ?? workflow.local?.tags ?? [];
  const highlights: WorkflowCatalogHighlight[] = [];
  const chain = detectChain(tags);
  const shape = detectShape(tags);
  const maturity = detectMaturity(tags);
  const graph = workflow.graph;

  if (chain) highlights.push({ label: "chain", value: chain });
  if (shape) highlights.push({ label: "shape", value: shape });
  if (maturity) highlights.push({ label: "mode", value: maturity });

  if (graph?.nodes?.length) {
    const solveCount = countKind(graph.nodes, "solve");
    const transformCount = countKind(graph.nodes, "transform");
    const extractCount = countKind(graph.nodes, "extract");

    if (solveCount > 1 || transformCount > 0 || extractCount > 0) {
      highlights.push({
        label: "stages",
        value: `${solveCount} solve / ${transformCount} bridge / ${extractCount} extract`,
      });
    }
  }

  return highlights.slice(0, 4);
}
