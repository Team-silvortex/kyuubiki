"use client";

import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import {
  createBridgeConfigForOperator,
} from "@/lib/workbench/workflow-bridge-contract";
import type {
  WorkflowTemplateChainConnection,
  WorkflowTemplateChainDefinition,
} from "@/components/workbench/workflow/workbench-workflow-template-chain-library";

function buildElectrostaticComparisonTemplateChain(): WorkflowTemplateChainDefinition {
  return {
    id: "electrostatic_quad_triangle_compare",
    label: "electrostatic quad vs triangle compare",
    source: "built-in",
    summary:
      "Run electrostatic quad and triangle solves, normalize shared summary fields, compare them, and export the benchmark delta.",
    tags: ["electrostatic", "triangle", "quad", "compare", "benchmark", "2d"],
    templates: [
      { kind: "solve", operatorId: "solve.electrostatic_plane_quad_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: { fields: ["max_potential", "max_electric_field", "max_flux_density"] },
      },
      { kind: "solve", operatorId: "solve.electrostatic_plane_triangle_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: { fields: ["max_potential", "max_electric_field", "max_flux_density"] },
      },
      {
        kind: "transform",
        operatorId: "transform.normalize_summary_fields",
        config: {
          copy_unmapped: true,
          rules: [
            { source: "max_potential", target: "potential_peak" },
            { source: "max_electric_field", target: "electric_field_peak" },
            { source: "max_flux_density", target: "flux_density_peak" },
          ],
        },
      },
      {
        kind: "transform",
        operatorId: "transform.normalize_summary_fields",
        config: {
          copy_unmapped: true,
          rules: [
            { source: "max_potential", target: "potential_peak" },
            { source: "max_electric_field", target: "electric_field_peak" },
            { source: "max_flux_density", target: "flux_density_peak" },
          ],
        },
      },
      {
        kind: "transform",
        operatorId: "transform.compare_summary_pair",
        config: {
          left_prefix: "quad",
          right_prefix: "triangle",
          delta_prefix: "delta",
          ratio_prefix: "ratio",
          percent_prefix: "percent_change",
          include_originals: true,
          include_delta: true,
          include_ratio: true,
          include_percent_change: true,
          include_shared_field_count: true,
        },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
    connections: [
      { from: 0, to: 1 },
      { from: 2, to: 3 },
      { from: 1, to: 4 },
      { from: 3, to: 5 },
      { from: 4, to: 6, toPort: "left" },
      { from: 5, to: 6, toPort: "right" },
      { from: 6, to: 7 },
    ],
  };
}

function buildElectrostaticPreheatGuardTemplateChain(params: {
  id: string;
  label: string;
  summary: string;
  tags: string[];
  solveOperatorId: string;
  titlePrefix: string;
}): WorkflowTemplateChainDefinition {
  const templates: WorkflowNodeTemplateSelection[] = [
    { kind: "solve", operatorId: params.solveOperatorId },
    {
      kind: "extract",
      operatorId: "extract.field_hotspots",
      config: {
        source: "elements",
        field: "electric_field_magnitude",
        output_prefix: "field",
        percentile: 90,
        sample_limit: 4,
        sample_sort: "value_desc",
      },
    },
    {
      kind: "condition",
      config: { predicate: { path: "field_hotspot_count", operator: "gt", value: 0 } },
    },
    {
      kind: "export",
      operatorId: "export.alert_markdown",
      config: {
        title: `${params.titlePrefix} Hotspot Hold`,
        severity: "warning",
        summary:
          "Hotspot candidates were detected. Review the electrostatic field before heat coupling.",
        fields: ["field_hotspot_count", "field_hotspot_fraction", "field_threshold"],
        sample_count: 4,
      },
    },
    {
      kind: "export",
      operatorId: "export.alert_markdown",
      config: {
        title: `${params.titlePrefix} Ready For Heat Bridge`,
        severity: "info",
        summary:
          "No hotspot candidates exceeded the configured guard threshold. The electrostatic result is ready for heat-bridge review.",
        fields: ["field_hotspot_count", "field_hotspot_fraction", "field_threshold"],
        sample_count: 4,
      },
    },
    { kind: "transform", operatorId: "transform.first_available" },
    { kind: "output" },
  ];
  const connections: WorkflowTemplateChainConnection[] = [
    { from: 0, to: 1 },
    { from: 1, to: 2 },
    { from: 2, to: 3, fromPort: "if_true" },
    { from: 2, to: 4, fromPort: "if_false" },
    { from: 3, to: 5, toPort: "left" },
    { from: 4, to: 5, toPort: "right" },
    { from: 5, to: 6 },
  ];
  return {
    id: params.id,
    label: params.label,
    source: "built-in",
    summary: params.summary,
    tags: params.tags,
    templates,
    connections,
  };
}

export const ELECTROMAGNETIC_TEMPLATE_CHAINS: WorkflowTemplateChainDefinition[] = [
  buildElectrostaticPreheatGuardTemplateChain({
    id: "electrostatic_preheat_guard",
    label: "electrostatic pre-heat guard",
    summary:
      "Evaluate electrostatic field hotspots before thermal coupling and emit a readiness or hold alert.",
    tags: ["electrostatic", "heat", "guard", "readiness", "hotspot", "2d"],
    solveOperatorId: "solve.electrostatic_plane_quad_2d",
    titlePrefix: "Electrostatic Pre-Heat",
  }),
  buildElectrostaticPreheatGuardTemplateChain({
    id: "electrostatic_triangle_preheat_guard",
    label: "electrostatic triangle pre-heat guard",
    summary:
      "Evaluate electrostatic triangle field hotspots before thermal coupling and emit a readiness or hold alert.",
    tags: ["electrostatic", "triangle", "heat", "guard", "readiness", "hotspot", "2d"],
    solveOperatorId: "solve.electrostatic_plane_triangle_2d",
    titlePrefix: "Electrostatic Triangle Pre-Heat",
  }),
  {
    id: "electrostatic_bridge_heat",
    label: "electrostatic -> bridge -> heat",
    source: "built-in",
    summary: "Electrostatic field bridge into heat plane quad solve.",
    tags: ["electrostatic", "heat", "bridge", "coupled", "2d"],
    templates: [
      { kind: "solve", operatorId: "solve.electrostatic_plane_quad_2d" },
      {
        kind: "transform",
        operatorId: "bridge.electrostatic_field_to_heat_quad_2d",
        config: createBridgeConfigForOperator("bridge.electrostatic_field_to_heat_quad_2d") ?? undefined,
      },
      { kind: "solve", operatorId: "solve.heat_plane_quad_2d" },
    ],
  },
  {
    id: "electrostatic_triangle_bridge_heat_triangle",
    label: "electrostatic triangle -> bridge -> heat triangle",
    source: "built-in",
    summary: "Electrostatic triangle field bridge into heat plane triangle solve.",
    tags: ["electrostatic", "heat", "bridge", "triangle", "coupled", "2d"],
    templates: [
      { kind: "solve", operatorId: "solve.electrostatic_plane_triangle_2d" },
      {
        kind: "transform",
        operatorId: "bridge.electrostatic_field_to_heat_triangle_2d",
        config:
          createBridgeConfigForOperator("bridge.electrostatic_field_to_heat_triangle_2d") ??
          undefined,
      },
      { kind: "solve", operatorId: "solve.heat_plane_triangle_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: { fields: ["max_temperature", "max_heat_flux"] },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
  },
  {
    id: "electrostatic_triangle_heat_thermo_triangle_summary",
    label: "electrostatic triangle -> heat triangle -> thermo triangle summary",
    source: "built-in",
    summary:
      "Full coupled triangle chain from electrostatic field, through heat loading, into thermo-mechanical solve and summary export.",
    tags: [
      "electromagnetic",
      "electrostatic",
      "heat",
      "thermal",
      "thermo_mechanical",
      "bridge",
      "triangle",
      "coupled",
      "summary",
      "2d",
    ],
    templates: [
      { kind: "solve", operatorId: "solve.electrostatic_plane_triangle_2d" },
      {
        kind: "transform",
        operatorId: "bridge.electrostatic_field_to_heat_triangle_2d",
        config:
          createBridgeConfigForOperator("bridge.electrostatic_field_to_heat_triangle_2d") ??
          undefined,
      },
      { kind: "solve", operatorId: "solve.heat_plane_triangle_2d" },
      {
        kind: "transform",
        operatorId: "bridge.temperature_field_to_thermo_triangle_2d",
        config:
          createBridgeConfigForOperator("bridge.temperature_field_to_thermo_triangle_2d") ??
          undefined,
      },
      { kind: "solve", operatorId: "solve.thermal_plane_triangle_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: { fields: ["max_displacement", "max_stress", "max_temperature_delta"] },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
  },
  {
    id: "electrostatic_heat_thermo_summary",
    label: "electrostatic -> heat -> thermo summary",
    source: "built-in",
    summary:
      "Full coupled chain from electrostatic field, through heat loading, into thermo-mechanical solve and summary export.",
    tags: [
      "electromagnetic",
      "electrostatic",
      "heat",
      "thermal",
      "thermo_mechanical",
      "bridge",
      "coupled",
      "summary",
      "2d",
    ],
    templates: [
      { kind: "solve", operatorId: "solve.electrostatic_plane_quad_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: { fields: ["max_potential", "max_electric_field"] },
      },
      {
        kind: "transform",
        operatorId: "bridge.electrostatic_field_to_heat_quad_2d",
        config:
          createBridgeConfigForOperator("bridge.electrostatic_field_to_heat_quad_2d") ??
          undefined,
      },
      { kind: "solve", operatorId: "solve.heat_plane_quad_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: { fields: ["max_temperature", "max_heat_flux"] },
      },
      {
        kind: "transform",
        operatorId: "bridge.temperature_field_to_thermo_quad_2d",
        config:
          createBridgeConfigForOperator("bridge.temperature_field_to_thermo_quad_2d") ??
          undefined,
      },
      { kind: "solve", operatorId: "solve.thermal_plane_quad_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: {
          fields: [
            "max_displacement",
            "max_stress",
            "max_temperature_delta",
            "max_temperature_gradient",
          ],
        },
      },
      {
        kind: "transform",
        operatorId: "transform.merge_summary_pair",
        config: { left_prefix: "electrostatic", right_prefix: "heat", include_source_count: false },
      },
      {
        kind: "transform",
        operatorId: "transform.merge_summary_pair",
        config: { left_prefix: "", right_prefix: "thermo", include_source_count: false },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
    connections: [
      { from: 0, to: 1 },
      { from: 0, to: 2 },
      { from: 2, to: 3 },
      { from: 3, to: 4 },
      { from: 3, to: 5 },
      { from: 5, to: 6 },
      { from: 6, to: 7 },
      { from: 1, to: 8, toPort: "left" },
      { from: 4, to: 8, toPort: "right" },
      { from: 8, to: 9, toPort: "left" },
      { from: 7, to: 9, toPort: "right" },
      { from: 9, to: 10 },
    ],
  },
  {
    id: "electrostatic_summary",
    label: "electrostatic summary",
    source: "built-in",
    summary: "Electrostatic solve and field summary export.",
    tags: ["electrostatic", "summary", "field", "2d"],
    templates: [
      { kind: "solve", operatorId: "solve.electrostatic_plane_quad_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: {
          fields: ["max_potential", "max_electric_field", "max_flux_density"],
        },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
  },
  {
    id: "electrostatic_triangle_summary",
    label: "electrostatic triangle summary",
    source: "built-in",
    summary: "Electrostatic triangle solve and field summary export.",
    tags: ["electrostatic", "triangle", "summary", "field", "2d"],
    templates: [
      { kind: "solve", operatorId: "solve.electrostatic_plane_triangle_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: {
          fields: ["max_potential", "max_electric_field", "max_flux_density"],
        },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
  },
  buildElectrostaticComparisonTemplateChain(),
];
