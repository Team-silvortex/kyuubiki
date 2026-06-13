"use client";

import { buildElectrostaticElementReadout, buildElectrostaticNodeReadout, type ElectromagneticReadout } from "@/components/workbench/workbench-electromagnetic-readout";
import type { PlaneElement, PlaneNode } from "@/components/workbench/workbench-viewport-core";

export type PlaneReadout = ElectromagneticReadout & { kind: "node" | "element" };

function formatPlaneMetric(value: number, digits = 3) {
  if (!Number.isFinite(value)) return "--";
  const abs = Math.abs(value);
  return abs >= 1.0e3 || (abs > 0 && abs < 1.0e-3) ? value.toExponential(2) : value.toFixed(digits);
}

function isElectrostaticNode(node: PlaneNode) {
  return typeof node.potential === "number" || typeof node.charge_density === "number";
}

function isElectrostaticElement(element: PlaneElement) {
  return (
    typeof element.average_potential === "number" ||
    typeof element.electric_field_magnitude === "number" ||
    typeof element.electric_flux_density_magnitude === "number"
  );
}

function isThermalElement(studyKind: string, element: PlaneElement) {
  return (
    studyKind === "heat_plane_triangle_2d" ||
    studyKind === "heat_plane_quad_2d" ||
    studyKind === "thermal_plane_triangle_2d" ||
    studyKind === "thermal_plane_quad_2d" ||
    typeof element.average_temperature === "number" ||
    typeof element.average_temperature_delta === "number" ||
    typeof element.heat_flux_magnitude === "number"
  );
}

export function buildPlaneNodeReadout(studyKind: string, node: PlaneNode): PlaneReadout {
  if (isElectrostaticNode(node)) {
    return {
      kind: "node",
      ...buildElectrostaticNodeReadout(
        { ...node, potential: node.potential ?? 0, charge_density: node.charge_density ?? 0 },
        { title: `${node.id} · plane node` },
      ),
    };
  }

  const isThermal =
    studyKind === "heat_plane_triangle_2d" ||
    studyKind === "heat_plane_quad_2d" ||
    studyKind === "thermal_plane_triangle_2d" ||
    studyKind === "thermal_plane_quad_2d" ||
    typeof node.ux === "number" ||
    typeof node.uy === "number";

  return {
    kind: "node",
    title: `${node.id} · ${isThermal ? "plane node" : "structural node"}`,
    lines: [
      `x ${formatPlaneMetric(node.x)} · y ${formatPlaneMetric(node.y)} m`,
      `ux ${formatPlaneMetric(node.ux)} · uy ${formatPlaneMetric(node.uy)} m`,
      `|u| ${formatPlaneMetric(Math.hypot(node.ux, node.uy))} m`,
    ],
  };
}

export function buildPlaneElementReadout(studyKind: string, element: PlaneElement): PlaneReadout {
  if (isElectrostaticElement(element)) {
    return {
      kind: "element",
      ...buildElectrostaticElementReadout(element, { title: `${element.id} · plane field`, dimensionality: "2d" }),
    };
  }

  if (isThermalElement(studyKind, element)) {
    const isHeat = studyKind === "heat_plane_triangle_2d" || studyKind === "heat_plane_quad_2d";
    return {
      kind: "element",
      title: `${element.id} · ${isHeat ? "heat plane" : "thermoelastic plane"}`,
      lines: [
        `Tavg ${formatPlaneMetric(element.average_temperature ?? element.average_temperature_delta ?? 0)} ${isHeat ? "K" : "K"}`,
        `grad x ${formatPlaneMetric(element.temperature_gradient_x ?? 0)} · y ${formatPlaneMetric(element.temperature_gradient_y ?? 0)}`,
        `flux x ${formatPlaneMetric(element.heat_flux_x ?? 0)} · y ${formatPlaneMetric(element.heat_flux_y ?? 0)}`,
        `|q| ${formatPlaneMetric(element.heat_flux_magnitude ?? 0)}`,
      ],
    };
  }

  return {
    kind: "element",
    title: `${element.id} · plane stress`,
    lines: [
      `von Mises ${formatPlaneMetric(element.von_mises ?? 0)} Pa`,
      `p1 ${formatPlaneMetric(element.principal_stress_1 ?? 0)} Pa`,
      `shear ${formatPlaneMetric(element.max_in_plane_shear ?? 0)} Pa`,
      `strain x ${formatPlaneMetric(element.mechanical_strain_x ?? 0)} · y ${formatPlaneMetric(element.mechanical_strain_y ?? 0)}`,
    ],
  };
}
