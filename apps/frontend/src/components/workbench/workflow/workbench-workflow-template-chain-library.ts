"use client";

import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { createBridgeConfigForOperator } from "@/components/workbench/workflow/workbench-workflow-bridge-contract";

const WORKFLOW_TEMPLATE_CHAIN_LIBRARY_KEY =
  "kyuubiki.workflow.templateChainLibrary.v1";

export type WorkflowTemplateChainDefinition = {
  id: string;
  label: string;
  templates: WorkflowNodeTemplateSelection[];
  connections?: WorkflowTemplateChainConnection[];
  summary?: string;
  version?: string;
  tags?: string[];
  updatedAt?: string;
  source: "built-in" | "imported";
};

export type WorkflowTemplateChainConnection = {
  from: number;
  to: number;
  fromPort?: string;
  toPort?: string;
};

function buildSummaryTemplateChain(params: {
  id: string;
  label: string;
  summary: string;
  tags: string[];
  solveOperatorId: string;
  fields: string[];
}): WorkflowTemplateChainDefinition {
  return {
    id: params.id,
    label: params.label,
    source: "built-in",
    summary: params.summary,
    tags: params.tags,
    templates: [
      { kind: "solve", operatorId: params.solveOperatorId },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: { fields: params.fields },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
  };
}

const BUILT_IN_TEMPLATE_CHAINS: WorkflowTemplateChainDefinition[] = [
  buildSummaryTemplateChain({
    id: "bar_1d_summary",
    label: "bar_1d summary",
    summary: "Bar 1D solve and summary export.",
    tags: ["bar", "1d", "summary", "structural"],
    solveOperatorId: "solve.bar_1d",
    fields: ["max_displacement", "max_stress"],
  }),
  buildSummaryTemplateChain({
    id: "thermal_bar_1d_summary",
    label: "thermal_bar_1d summary",
    summary: "Thermal bar 1D solve and thermo-mechanical summary export.",
    tags: ["thermal", "bar", "1d", "summary", "coupled"],
    solveOperatorId: "solve.thermal_bar_1d",
    fields: ["max_displacement", "max_stress", "max_axial_force", "max_temperature_delta"],
  }),
  buildSummaryTemplateChain({
    id: "heat_bar_1d_summary",
    label: "heat_bar_1d summary",
    summary: "Heat bar 1D solve and summary export.",
    tags: ["heat", "bar", "1d", "summary", "thermal"],
    solveOperatorId: "solve.heat_bar_1d",
    fields: ["max_temperature", "max_heat_flux"],
  }),
  buildSummaryTemplateChain({
    id: "heat_plane_triangle_2d_summary",
    label: "heat_plane_triangle_2d summary",
    summary: "Heat plane triangle 2D solve and summary export.",
    tags: ["heat", "plane", "triangle", "2d", "summary"],
    solveOperatorId: "solve.heat_plane_triangle_2d",
    fields: ["max_temperature", "max_heat_flux"],
  }),
  buildSummaryTemplateChain({
    id: "thermal_truss_2d_summary",
    label: "thermal_truss_2d summary",
    summary: "Thermal truss 2D solve and coupled summary export.",
    tags: ["thermal", "truss", "2d", "summary", "coupled"],
    solveOperatorId: "solve.thermal_truss_2d",
    fields: ["max_displacement", "max_stress", "max_axial_force", "max_temperature_delta"],
  }),
  buildSummaryTemplateChain({
    id: "torsion_1d_summary",
    label: "torsion_1d summary",
    summary: "Torsion 1D solve and summary export.",
    tags: ["torsion", "1d", "summary", "structural"],
    solveOperatorId: "solve.torsion_1d",
    fields: ["max_rotation", "max_torque", "max_stress"],
  }),
  buildSummaryTemplateChain({
    id: "plane_triangle_2d_summary",
    label: "plane_triangle_2d summary",
    summary: "Plane triangle 2D solve and summary export.",
    tags: ["plane", "triangle", "2d", "summary", "structural"],
    solveOperatorId: "solve.plane_triangle_2d",
    fields: ["max_displacement", "max_stress"],
  }),
  buildSummaryTemplateChain({
    id: "thermal_plane_triangle_2d_summary",
    label: "thermal_plane_triangle_2d summary",
    summary: "Thermal plane triangle 2D solve and coupled summary export.",
    tags: ["thermal", "plane", "triangle", "2d", "summary", "coupled"],
    solveOperatorId: "solve.thermal_plane_triangle_2d",
    fields: ["max_displacement", "max_stress", "max_temperature_delta"],
  }),
  buildSummaryTemplateChain({
    id: "plane_quad_2d_summary",
    label: "plane_quad_2d summary",
    summary: "Plane quad 2D solve and summary export.",
    tags: ["plane", "quad", "2d", "summary", "structural"],
    solveOperatorId: "solve.plane_quad_2d",
    fields: ["max_displacement", "max_stress"],
  }),
  {
    id: "frame_2d_summary",
    label: "frame_2d summary",
    source: "built-in",
    summary: "Frame 2D solve, extract, and summary export.",
    tags: ["frame", "2d", "summary", "structural"],
    templates: [
      { kind: "solve", operatorId: "solve.frame_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: { fields: ["max_displacement", "max_rotation", "max_moment", "max_stress"] },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
  },
  {
    id: "thermal_frame_2d_summary",
    label: "thermal_frame_2d summary",
    source: "built-in",
    summary: "Thermal frame 2D solve and combined thermo-mechanical summary.",
    tags: ["thermal", "frame", "2d", "summary", "coupled"],
    templates: [
      { kind: "solve", operatorId: "solve.thermal_frame_2d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: {
          fields: ["max_displacement", "max_rotation", "max_axial_force", "max_moment", "max_stress", "max_temperature_delta", "max_temperature_gradient"],
        },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
  },
  {
    id: "truss_3d_summary",
    label: "truss_3d summary",
    source: "built-in",
    summary: "Truss 3D solve and summary export.",
    tags: ["truss", "3d", "summary", "structural"],
    templates: [
      { kind: "solve", operatorId: "solve.truss_3d" },
      {
        kind: "extract",
        operatorId: "extract.result_summary",
        config: { fields: ["max_displacement", "max_stress"] },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
  },
  {
    id: "frame_3d_summary",
    label: "frame_3d summary",
    source: "built-in",
    summary: "Frame 3D solve and summary export.",
    tags: ["frame", "3d", "summary", "structural"],
    templates: [
      { kind: "solve", operatorId: "solve.frame_3d" },
      { kind: "extract", operatorId: "extract.result_summary" },
      { kind: "export", operatorId: "export.summary_json" },
    ],
  },
  {
    id: "heat_bridge_thermo",
    label: "heat -> bridge -> thermo",
    source: "built-in",
    summary: "Heat field bridge into thermo-mechanical plane quad solve.",
    tags: ["heat", "thermal", "bridge", "coupled", "2d"],
    templates: [
      { kind: "solve", operatorId: "solve.heat_plane_quad_2d" },
      {
        kind: "transform",
        operatorId: "bridge.temperature_field_to_thermo_quad_2d",
        config: createBridgeConfigForOperator("bridge.temperature_field_to_thermo_quad_2d") ?? undefined,
      },
      { kind: "solve", operatorId: "solve.thermal_plane_quad_2d" },
    ],
  },
  {
    id: "heat_triangle_bridge_thermo_triangle",
    label: "heat triangle -> bridge -> thermo triangle",
    source: "built-in",
    summary: "Heat field bridge into thermo-mechanical plane triangle solve.",
    tags: ["heat", "thermal", "bridge", "triangle", "coupled", "2d"],
    templates: [
      { kind: "solve", operatorId: "solve.heat_plane_triangle_2d" },
      {
        kind: "transform",
        operatorId: "bridge.temperature_field_to_thermo_triangle_2d",
        config: createBridgeConfigForOperator("bridge.temperature_field_to_thermo_triangle_2d") ?? undefined,
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
    id: "electrostatic_hotspot_alert",
    label: "electrostatic hotspot alert",
    source: "built-in",
    summary: "Electrostatic solve, hotspot extraction, and markdown alert export.",
    tags: ["electrostatic", "hotspot", "alert", "field", "2d"],
    templates: [
      { kind: "solve", operatorId: "solve.electrostatic_plane_quad_2d" },
      {
        kind: "extract",
        operatorId: "extract.field_hotspots",
        config: { source: "elements", field: "electric_field_magnitude", output_prefix: "field", percentile: 90 },
      },
      {
        kind: "export",
        operatorId: "export.alert_markdown",
        config: {
          title: "Electrostatic Hotspot Alert",
          severity: "warning",
          summary: "Hotspot candidates were detected in the electrostatic field.",
          fields: ["field_threshold", "field_hotspot_count", "field_hotspot_fraction"],
        },
      },
    ],
  },
  { id: "electrostatic_hotspot_guard", label: "electrostatic hotspot guard", source: "built-in", summary: "Route hotspot summaries into alert or clear markdown output.", tags: ["electrostatic", "hotspot", "condition", "alert", "2d"], templates: [{ kind: "solve", operatorId: "solve.electrostatic_plane_quad_2d" }, { kind: "extract", operatorId: "extract.field_hotspots", config: { source: "elements", field: "electric_field_magnitude", output_prefix: "field", percentile: 90 } }, { kind: "condition", config: { predicate: { path: "field_hotspot_count", operator: "gt", value: 0 } } }, { kind: "export", operatorId: "export.alert_markdown", config: { title: "Electrostatic Hotspot Alert", severity: "warning", summary: "Hotspot candidates exceeded the workflow threshold.", fields: ["field_hotspot_count", "field_hotspot_fraction", "field_threshold"] } }, { kind: "export", operatorId: "export.alert_markdown", config: { title: "Electrostatic Field Clear", severity: "info", summary: "Hotspot count stayed within the configured workflow threshold.", fields: ["field_hotspot_count", "field_hotspot_fraction", "field_threshold"] } }, { kind: "transform", operatorId: "transform.first_available" }, { kind: "output" }], connections: [{ from: 0, to: 1 }, { from: 1, to: 2 }, { from: 2, to: 3, fromPort: "if_true" }, { from: 2, to: 4, fromPort: "if_false" }, { from: 3, to: 5, toPort: "left" }, { from: 4, to: 5, toPort: "right" }, { from: 5, to: 6 }] },
  {
    id: "condition_branch_merge_export",
    label: "condition -> branch -> merge",
    source: "built-in",
    summary: "Split a summary payload with a condition node, then merge the active branch back into one lane.",
    tags: ["condition", "branch", "merge", "control", "workflow"],
    templates: [
      { kind: "condition" },
      { kind: "transform", operatorId: "transform.first_available" },
      { kind: "output" },
    ],
  },
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asTemplateSelections(
  value: unknown,
): WorkflowNodeTemplateSelection[] | null {
  if (!Array.isArray(value)) return null;
  const templates = value.filter(
    (entry): entry is WorkflowNodeTemplateSelection =>
      isRecord(entry) &&
      typeof entry.kind === "string" &&
      (typeof entry.operatorId === "string" || entry.operatorId === undefined),
  );
  return templates.length === value.length ? templates : null;
}

function asStringArray(value: unknown): string[] | undefined {
  if (!Array.isArray(value)) return undefined;
  const tags = value.filter((entry): entry is string => typeof entry === "string");
  return tags.length === 0 ? undefined : tags;
}

function asImportedTemplateChain(
  value: unknown,
): WorkflowTemplateChainDefinition | null {
  if (!isRecord(value)) return null;
  if (typeof value.id !== "string" || typeof value.label !== "string") return null;
  const templates = asTemplateSelections(value.templates);
  if (!templates) return null;
  return {
    id: value.id,
    label: value.label,
    templates,
    summary: typeof value.summary === "string" ? value.summary : undefined,
    version: typeof value.version === "string" ? value.version : undefined,
    tags: asStringArray(value.tags),
    updatedAt: typeof value.updatedAt === "string" ? value.updatedAt : undefined,
    source: "imported",
  };
}

function readImportedTemplateChains(): WorkflowTemplateChainDefinition[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(WORKFLOW_TEMPLATE_CHAIN_LIBRARY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.flatMap((entry) => {
      const chain = asImportedTemplateChain(entry);
      return chain ? [chain] : [];
    });
  } catch {
    return [];
  }
}

function writeImportedTemplateChains(chains: WorkflowTemplateChainDefinition[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(
    WORKFLOW_TEMPLATE_CHAIN_LIBRARY_KEY,
    JSON.stringify(chains.filter((chain) => chain.source === "imported")),
  );
}

export function listBuiltInWorkflowTemplateChains() {
  return BUILT_IN_TEMPLATE_CHAINS;
}

export function listStoredWorkflowTemplateChains() {
  return readImportedTemplateChains().sort((left, right) => {
    const leftTime = left.updatedAt ?? "";
    const rightTime = right.updatedAt ?? "";
    return rightTime.localeCompare(leftTime) || left.label.localeCompare(right.label);
  });
}

export function listAllWorkflowTemplateChains() {
  return [...BUILT_IN_TEMPLATE_CHAINS, ...listStoredWorkflowTemplateChains()];
}

export function saveImportedWorkflowTemplateChain(
  chain: Omit<WorkflowTemplateChainDefinition, "source">,
) {
  const imported = listStoredWorkflowTemplateChains();
  const nextChain: WorkflowTemplateChainDefinition = {
    ...chain,
    updatedAt: new Date().toISOString(),
    source: "imported",
  };
  const next = [nextChain, ...imported.filter((entry) => entry.id !== chain.id)].slice(0, 40);
  writeImportedTemplateChains(next);
  return nextChain;
}

export function removeImportedWorkflowTemplateChain(chainId: string) {
  writeImportedTemplateChains(
    listStoredWorkflowTemplateChains().filter((entry) => entry.id !== chainId),
  );
}

export function updateImportedWorkflowTemplateChain(
  chainId: string,
  updater: (
    chain: Omit<WorkflowTemplateChainDefinition, "source">,
  ) => Omit<WorkflowTemplateChainDefinition, "source">,
) {
  writeImportedTemplateChains(
    listStoredWorkflowTemplateChains().map((entry) =>
      entry.id === chainId
        ? {
            ...updater(entry),
            updatedAt: new Date().toISOString(),
            source: "imported",
          }
        : entry,
    ),
  );
}
