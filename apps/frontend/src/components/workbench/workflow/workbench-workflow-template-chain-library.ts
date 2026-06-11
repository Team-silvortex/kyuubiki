"use client";

import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { createElectrostaticToHeatBridgeContract } from "@/components/workbench/workflow/workbench-workflow-bridge-contract";

const WORKFLOW_TEMPLATE_CHAIN_LIBRARY_KEY =
  "kyuubiki.workflow.templateChainLibrary.v1";

export type WorkflowTemplateChainDefinition = {
  id: string;
  label: string;
  templates: WorkflowNodeTemplateSelection[];
  summary?: string;
  version?: string;
  tags?: string[];
  updatedAt?: string;
  source: "built-in" | "imported";
};

const BUILT_IN_TEMPLATE_CHAINS: WorkflowTemplateChainDefinition[] = [
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
      { kind: "transform", operatorId: "bridge.temperature_field_to_thermo_quad_2d" },
      { kind: "solve", operatorId: "solve.thermal_plane_quad_2d" },
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
        config: { contract: createElectrostaticToHeatBridgeContract() },
      },
      { kind: "solve", operatorId: "solve.heat_plane_quad_2d" },
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
