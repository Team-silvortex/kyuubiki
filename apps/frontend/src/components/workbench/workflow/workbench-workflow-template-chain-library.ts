"use client";

import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { createBridgeConfigForOperator } from "@/components/workbench/workflow/workbench-workflow-bridge-contract";
import { ELECTROMAGNETIC_TEMPLATE_CHAINS } from "@/components/workbench/workflow/workbench-workflow-template-chain-electromagnetic-presets";
import { HOTSPOT_TEMPLATE_CHAINS } from "@/components/workbench/workflow/workbench-workflow-template-chain-hotspot-presets";
import { STATISTICS_TEMPLATE_CHAINS } from "@/components/workbench/workflow/workbench-workflow-template-chain-statistics-presets";

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
  ...STATISTICS_TEMPLATE_CHAINS,
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
  ...ELECTROMAGNETIC_TEMPLATE_CHAINS,
  ...HOTSPOT_TEMPLATE_CHAINS,
  {
    id: "diagnostics_bundle_guard_report",
    label: "diagnostics -> bundle -> guard -> report",
    source: "built-in",
    summary:
      "Bundle electrostatic, thermal, and thermo diagnostics, evaluate a unified guard, and export a markdown report.",
    tags: [
      "diagnostics",
      "bundle",
      "guard",
      "report",
      "markdown",
      "headless",
    ],
    templates: [
      {
        kind: "extract",
        operatorId: "extract.electrostatic_result_diagnostics",
      },
      {
        kind: "extract",
        operatorId: "extract.thermal_result_diagnostics",
      },
      {
        kind: "extract",
        operatorId: "extract.thermo_result_diagnostics",
      },
      {
        kind: "transform",
        operatorId: "transform.compose_diagnostics_bundle",
      },
      {
        kind: "transform",
        operatorId: "transform.evaluate_diagnostics_bundle_guard",
        config: {
          rules: [
            {
              source: "thermal",
              field: "thermal_temperature_max",
              threshold: 120.0,
              severity: "warn",
              label: "thermal temperature",
            },
            {
              source: "thermo",
              field: "thermo_peak_stress",
              comparison: "gt",
              threshold: 180.0,
              severity: "block",
              label: "stress ceiling",
            },
            {
              source: "electrostatic",
              field: "electrostatic_field_peak_magnitude",
              comparison: "gt",
              threshold: 9.0,
              severity: "warn",
              label: "field ceiling",
            },
          ],
        },
      },
      {
        kind: "transform",
        operatorId: "transform.compose_diagnostics_report_payload",
      },
      {
        kind: "export",
        operatorId: "export.diagnostics_bundle_markdown",
        config: { title: "Diagnostics Bundle Report" },
      },
    ],
    connections: [
      { from: 0, to: 3, toPort: "electrostatic" },
      { from: 1, to: 3, toPort: "thermal" },
      { from: 2, to: 3, toPort: "thermo" },
      { from: 3, to: 4, toPort: "bundle" },
      { from: 3, to: 5, toPort: "bundle" },
      { from: 4, to: 5, toPort: "guard" },
      { from: 5, to: 6, toPort: "bundle" },
    ],
  },
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
