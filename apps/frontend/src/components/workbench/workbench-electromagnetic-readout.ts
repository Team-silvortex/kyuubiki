"use client";

type ElectrostaticNodeLike = {
  id: string;
  x: number;
  y?: number;
  z?: number;
  potential: number;
  charge_density?: number;
};

type ElectrostaticElementLike = {
  id: string;
  average_potential?: number;
  potential_gradient?: number;
  potential_gradient_x?: number;
  potential_gradient_y?: number;
  electric_field?: number;
  electric_field_x?: number;
  electric_field_y?: number;
  electric_field_magnitude?: number;
  electric_flux_density?: number;
  electric_flux_density_x?: number;
  electric_flux_density_y?: number;
  electric_flux_density_magnitude?: number;
};

export type ElectromagneticReadout = {
  title: string;
  lines: string[];
};

function formatElectromagneticMetric(value: number, digits = 3) {
  if (!Number.isFinite(value)) return "--";
  const abs = Math.abs(value);
  return abs >= 1.0e3 || (abs > 0 && abs < 1.0e-3) ? value.toExponential(2) : value.toFixed(digits);
}

function formatCoordinates(node: ElectrostaticNodeLike) {
  if (typeof node.z === "number") {
    return `x ${formatElectromagneticMetric(node.x)} · y ${formatElectromagneticMetric(node.y ?? 0)} · z ${formatElectromagneticMetric(node.z)} m`;
  }
  if (typeof node.y === "number") {
    return `x ${formatElectromagneticMetric(node.x)} · y ${formatElectromagneticMetric(node.y)} m`;
  }
  return `x ${formatElectromagneticMetric(node.x)} m`;
}

export function buildElectrostaticNodeReadout(
  node: ElectrostaticNodeLike,
  options: { title?: string } = {},
): ElectromagneticReadout {
  return {
    title: options.title ?? `${node.id} · electrostatic node`,
    lines: [
      `potential ${formatElectromagneticMetric(node.potential)} V`,
      `charge ${formatElectromagneticMetric(node.charge_density ?? 0)} C/m^3`,
      formatCoordinates(node),
    ],
  };
}

export function buildElectrostaticElementReadout(
  element: ElectrostaticElementLike,
  options: { title?: string; dimensionality?: "1d" | "2d" | "3d" } = {},
): ElectromagneticReadout {
  const dimensionality = options.dimensionality ?? "2d";
  const gradientLine =
    dimensionality === "1d"
      ? `grad ${formatElectromagneticMetric(element.potential_gradient ?? 0)} V/m`
      : `grad x ${formatElectromagneticMetric(element.potential_gradient_x ?? 0)} · y ${formatElectromagneticMetric(element.potential_gradient_y ?? 0)} V/m`;
  const fieldLine =
    dimensionality === "1d"
      ? `field ${formatElectromagneticMetric(element.electric_field ?? 0)} V/m`
      : `field |E| ${formatElectromagneticMetric(element.electric_field_magnitude ?? 0)} V/m`;
  const fluxLine =
    dimensionality === "1d"
      ? `flux ${formatElectromagneticMetric(element.electric_flux_density ?? 0)} C/m^2`
      : `flux |D| ${formatElectromagneticMetric(element.electric_flux_density_magnitude ?? 0)} C/m^2`;

  return {
    title: options.title ?? `${element.id} · electrostatic field`,
    lines: [
      `avg potential ${formatElectromagneticMetric(element.average_potential ?? 0)} V`,
      gradientLine,
      fieldLine,
      fluxLine,
    ],
  };
}
