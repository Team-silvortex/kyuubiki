"use client";

import type { DisplayTruss3dElement, DisplayTruss3dNode } from "@/components/workbench/workbench-viewport-core";

export type Truss3dReadoutStudyKind = "truss_3d" | "thermal_truss_3d" | "spring_3d";

export type Truss3dReadout =
  | { kind: "node"; title: string; lines: string[] }
  | { kind: "element"; title: string; lines: string[] }
  | null;

export function formatReadoutMetric(value: number, digits = 3) {
  if (!Number.isFinite(value)) return "--";
  const abs = Math.abs(value);
  return abs >= 1.0e3 || (abs > 0 && abs < 1.0e-3) ? value.toExponential(2) : value.toFixed(digits);
}

function nodeTitle(kind: Truss3dReadoutStudyKind, node: DisplayTruss3dNode) {
  return `${node.id} · ${kind === "spring_3d" ? "selected spring node" : kind === "thermal_truss_3d" ? "selected thermal node" : "selected node"}`;
}

function elementTitle(kind: Truss3dReadoutStudyKind, element: DisplayTruss3dElement) {
  return `${element.id} · ${kind === "spring_3d" ? "selected spring member" : kind === "thermal_truss_3d" ? "selected thermo member" : "selected member"}`;
}

export function buildNodeReadout(kind: Truss3dReadoutStudyKind, node: DisplayTruss3dNode): Truss3dReadout {
  const thermalLine = kind === "thermal_truss_3d" ? [`dT ${formatReadoutMetric(node.temperature_delta ?? 0)} K`] : [];
  return {
    kind: "node",
    title: nodeTitle(kind, node),
    lines: [
      `|u| ${formatReadoutMetric(Math.hypot(node.ux, node.uy, node.uz))} m`,
      `ux ${formatReadoutMetric(node.ux)} · uy ${formatReadoutMetric(node.uy)} · uz ${formatReadoutMetric(node.uz)} m`,
      `x ${formatReadoutMetric(node.x)} · y ${formatReadoutMetric(node.y)} · z ${formatReadoutMetric(node.z)} m`,
      ...thermalLine,
    ],
  };
}

export function buildElementReadout(kind: Truss3dReadoutStudyKind, element: DisplayTruss3dElement): Truss3dReadout {
  const primary = kind === "spring_3d" ? `force ${formatReadoutMetric(element.axial_force)} N` : `stress ${formatReadoutMetric(element.stress)} Pa`;
  const secondary =
    kind === "spring_3d"
      ? `extension ${formatReadoutMetric(element.strain)} m`
      : kind === "thermal_truss_3d"
        ? `axial ${formatReadoutMetric(element.axial_force)} N · thermo strain ${formatReadoutMetric(element.strain)}`
        : `axial ${formatReadoutMetric(element.axial_force)} N`;
  const thermalLine =
    kind === "thermal_truss_3d"
      ? [`dT ${formatReadoutMetric(element.average_temperature_delta ?? 0)} K · mech ${formatReadoutMetric(element.mechanical_strain ?? 0)}`]
      : [];
  return {
    kind: "element",
    title: elementTitle(kind, element),
    lines: [
      primary,
      secondary,
      `L ${formatReadoutMetric(element.length)} m · strain ${formatReadoutMetric(element.strain)}`,
      ...thermalLine,
    ],
  };
}

export function buildSelectionSummary(kind: Truss3dReadoutStudyKind, nodes: DisplayTruss3dNode[]): Truss3dReadout {
  const magnitudes = nodes.map((node) => Math.hypot(node.ux, node.uy, node.uz));
  const average = magnitudes.reduce((sum, value) => sum + value, 0) / Math.max(nodes.length, 1);
  const max = Math.max(...magnitudes, 0);
  return {
    kind: "node",
    title: `${nodes.length} ${kind === "spring_3d" ? "spring" : kind === "thermal_truss_3d" ? "thermal" : ""} nodes selected`.replace("  ", " "),
    lines: [
      `avg |u| ${formatReadoutMetric(average)} m`,
      `max |u| ${formatReadoutMetric(max)} m`,
      `first ${nodes[0]?.id ?? "--"} · last ${nodes.at(-1)?.id ?? "--"}`,
    ],
  };
}
