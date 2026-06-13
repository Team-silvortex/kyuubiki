"use client";

import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import type { WorkflowTemplateChainConnection, WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";

function buildHotspotAlertTemplateChain(params: {
  id: string;
  label: string;
  summary: string;
  tags: string[];
  solveOperatorId: string;
  field: string;
  outputPrefix: string;
  title: string;
  alertSummary: string;
}): WorkflowTemplateChainDefinition {
  const templates: WorkflowNodeTemplateSelection[] = [
    { kind: "solve", operatorId: params.solveOperatorId },
    { kind: "extract", operatorId: "extract.field_hotspots", config: { source: "elements", field: params.field, output_prefix: params.outputPrefix, percentile: 90, sample_limit: 4, sample_sort: "value_desc" } },
    { kind: "export", operatorId: "export.alert_markdown", config: { title: params.title, severity: "warning", summary: params.alertSummary, fields: [`${params.outputPrefix}_threshold`, `${params.outputPrefix}_hotspot_count`, `${params.outputPrefix}_hotspot_fraction`], sample_count: 4 } },
  ];
  return { id: params.id, label: params.label, source: "built-in", summary: params.summary, tags: params.tags, templates };
}

function buildHotspotGuardTemplateChain(params: {
  id: string;
  label: string;
  summary: string;
  tags: string[];
  solveOperatorId: string;
  field: string;
  outputPrefix: string;
  alertTitle: string;
  alertSummary: string;
  clearTitle: string;
  clearSummary: string;
}): WorkflowTemplateChainDefinition {
  const templates: WorkflowNodeTemplateSelection[] = [
    { kind: "solve", operatorId: params.solveOperatorId },
    { kind: "extract", operatorId: "extract.field_hotspots", config: { source: "elements", field: params.field, output_prefix: params.outputPrefix, percentile: 90, sample_limit: 4, sample_sort: "value_desc" } },
    { kind: "condition", config: { predicate: { path: `${params.outputPrefix}_hotspot_count`, operator: "gt", value: 0 } } },
    { kind: "export", operatorId: "export.alert_markdown", config: { title: params.alertTitle, severity: "warning", summary: params.alertSummary, fields: [`${params.outputPrefix}_hotspot_count`, `${params.outputPrefix}_hotspot_fraction`, `${params.outputPrefix}_threshold`], sample_count: 4 } },
    { kind: "export", operatorId: "export.alert_markdown", config: { title: params.clearTitle, severity: "info", summary: params.clearSummary, fields: [`${params.outputPrefix}_hotspot_count`, `${params.outputPrefix}_hotspot_fraction`, `${params.outputPrefix}_threshold`], sample_count: 4 } },
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
  return { id: params.id, label: params.label, source: "built-in", summary: params.summary, tags: params.tags, templates, connections };
}

export const HOTSPOT_TEMPLATE_CHAINS: WorkflowTemplateChainDefinition[] = [
  buildHotspotAlertTemplateChain({
    id: "electrostatic_hotspot_alert",
    label: "electrostatic hotspot alert",
    summary: "Electrostatic solve, hotspot extraction, and markdown alert export.",
    tags: ["electrostatic", "hotspot", "alert", "field", "2d"],
    solveOperatorId: "solve.electrostatic_plane_quad_2d",
    field: "electric_field_magnitude",
    outputPrefix: "field",
    title: "Electrostatic Hotspot Alert",
    alertSummary: "Hotspot candidates were detected in the electrostatic field.",
  }),
  buildHotspotAlertTemplateChain({
    id: "electrostatic_triangle_hotspot_alert",
    label: "electrostatic triangle hotspot alert",
    summary:
      "Electrostatic triangle solve, hotspot extraction, and markdown alert export.",
    tags: ["electrostatic", "triangle", "hotspot", "alert", "field", "2d"],
    solveOperatorId: "solve.electrostatic_plane_triangle_2d",
    field: "electric_field_magnitude",
    outputPrefix: "field",
    title: "Electrostatic Triangle Hotspot Alert",
    alertSummary:
      "Hotspot candidates were detected in the electrostatic triangle field.",
  }),
  buildHotspotGuardTemplateChain({
    id: "electrostatic_hotspot_guard",
    label: "electrostatic hotspot guard",
    summary: "Route hotspot summaries into alert or clear markdown output.",
    tags: ["electrostatic", "hotspot", "condition", "alert", "2d"],
    solveOperatorId: "solve.electrostatic_plane_quad_2d",
    field: "electric_field_magnitude",
    outputPrefix: "field",
    alertTitle: "Electrostatic Hotspot Alert",
    alertSummary: "Hotspot candidates exceeded the workflow threshold.",
    clearTitle: "Electrostatic Field Clear",
    clearSummary: "Hotspot count stayed within the configured workflow threshold.",
  }),
  buildHotspotGuardTemplateChain({
    id: "electrostatic_triangle_hotspot_guard",
    label: "electrostatic triangle hotspot guard",
    summary:
      "Route electrostatic triangle hotspot summaries into alert or clear markdown output.",
    tags: ["electrostatic", "triangle", "hotspot", "condition", "alert", "2d"],
    solveOperatorId: "solve.electrostatic_plane_triangle_2d",
    field: "electric_field_magnitude",
    outputPrefix: "field",
    alertTitle: "Electrostatic Triangle Hotspot Alert",
    alertSummary: "Electrostatic triangle hotspots exceeded the workflow threshold.",
    clearTitle: "Electrostatic Triangle Field Clear",
    clearSummary:
      "Electrostatic triangle hotspot count stayed within the configured workflow threshold.",
  }),
  buildHotspotAlertTemplateChain({
    id: "heat_hotspot_alert",
    label: "heat hotspot alert",
    summary: "Heat solve, hotspot extraction, and markdown alert export.",
    tags: ["heat", "hotspot", "alert", "thermal", "2d"],
    solveOperatorId: "solve.heat_plane_quad_2d",
    field: "heat_flux_magnitude",
    outputPrefix: "heat_flux",
    title: "Heat Hotspot Alert",
    alertSummary: "Heat-flux hotspot candidates were detected in the thermal field.",
  }),
  buildHotspotGuardTemplateChain({
    id: "heat_hotspot_guard",
    label: "heat hotspot guard",
    summary: "Route heat hotspot summaries into alert or clear markdown output.",
    tags: ["heat", "hotspot", "condition", "thermal", "2d"],
    solveOperatorId: "solve.heat_plane_quad_2d",
    field: "heat_flux_magnitude",
    outputPrefix: "heat_flux",
    alertTitle: "Heat Hotspot Alert",
    alertSummary: "Heat-flux hotspots exceeded the workflow threshold.",
    clearTitle: "Heat Field Clear",
    clearSummary: "Heat hotspot count stayed within the configured workflow threshold.",
  }),
  buildHotspotAlertTemplateChain({
    id: "thermo_hotspot_alert",
    label: "thermo hotspot alert",
    summary: "Thermo-mechanical solve, hotspot extraction, and markdown alert export.",
    tags: ["thermo_mechanical", "hotspot", "alert", "coupled", "2d"],
    solveOperatorId: "solve.thermal_plane_quad_2d",
    field: "von_mises_stress",
    outputPrefix: "stress",
    title: "Thermo Stress Hotspot Alert",
    alertSummary: "High-stress hotspot candidates were detected in the thermo-mechanical result.",
  }),
  buildHotspotGuardTemplateChain({
    id: "thermo_hotspot_guard",
    label: "thermo hotspot guard",
    summary: "Route thermo-mechanical hotspot summaries into alert or clear markdown output.",
    tags: ["thermo_mechanical", "hotspot", "condition", "coupled", "2d"],
    solveOperatorId: "solve.thermal_plane_quad_2d",
    field: "von_mises_stress",
    outputPrefix: "stress",
    alertTitle: "Thermo Stress Hotspot Alert",
    alertSummary: "Thermo-mechanical stress hotspots exceeded the workflow threshold.",
    clearTitle: "Thermo Field Clear",
    clearSummary: "Thermo-mechanical hotspot count stayed within the configured workflow threshold.",
  }),
];
