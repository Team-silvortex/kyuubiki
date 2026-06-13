"use client";

import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import type { WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";

function buildStatisticsTemplateChain(params: {
  id: string;
  label: string;
  summary: string;
  tags: string[];
  solveOperatorId: string;
  source: "nodes" | "elements";
  field: string;
  outputPrefix: string;
  percentiles: number[];
}) {
  const templates: WorkflowNodeTemplateSelection[] = [
    { kind: "solve", operatorId: params.solveOperatorId },
    {
      kind: "extract",
      operatorId: "extract.field_statistics",
      config: {
        source: params.source,
        field: params.field,
        output_prefix: params.outputPrefix,
        percentiles: params.percentiles,
      },
    },
    { kind: "export", operatorId: "export.summary_json" },
  ];
  return {
    id: params.id,
    label: params.label,
    source: "built-in",
    summary: params.summary,
    tags: params.tags,
    templates,
  } satisfies WorkflowTemplateChainDefinition;
}

export const STATISTICS_TEMPLATE_CHAINS: WorkflowTemplateChainDefinition[] = [
  buildStatisticsTemplateChain({
    id: "electrostatic_field_statistics",
    label: "electrostatic field statistics",
    summary: "Electrostatic solve, field statistics extraction, and summary JSON export.",
    tags: ["electrostatic", "statistics", "field", "2d"],
    solveOperatorId: "solve.electrostatic_plane_quad_2d",
    source: "elements",
    field: "electric_field_magnitude",
    outputPrefix: "field",
    percentiles: [50, 90, 99],
  }),
  buildStatisticsTemplateChain({
    id: "electrostatic_triangle_field_statistics",
    label: "electrostatic triangle field statistics",
    summary:
      "Electrostatic triangle solve, field statistics extraction, and summary JSON export.",
    tags: ["electrostatic", "triangle", "statistics", "field", "2d"],
    solveOperatorId: "solve.electrostatic_plane_triangle_2d",
    source: "elements",
    field: "electric_field_magnitude",
    outputPrefix: "field",
    percentiles: [50, 90, 99],
  }),
  buildStatisticsTemplateChain({
    id: "heat_flux_statistics",
    label: "heat flux statistics",
    summary: "Heat solve, heat-flux statistics extraction, and summary JSON export.",
    tags: ["heat", "statistics", "flux", "2d"],
    solveOperatorId: "solve.heat_plane_quad_2d",
    source: "elements",
    field: "heat_flux_magnitude",
    outputPrefix: "heat_flux",
    percentiles: [50, 90, 99],
  }),
  buildStatisticsTemplateChain({
    id: "thermo_stress_statistics",
    label: "thermo stress statistics",
    summary: "Thermo-mechanical solve, stress statistics extraction, and summary JSON export.",
    tags: ["thermo_mechanical", "statistics", "stress", "2d"],
    solveOperatorId: "solve.thermal_plane_quad_2d",
    source: "elements",
    field: "von_mises_stress",
    outputPrefix: "stress",
    percentiles: [50, 90, 99],
  }),
];
