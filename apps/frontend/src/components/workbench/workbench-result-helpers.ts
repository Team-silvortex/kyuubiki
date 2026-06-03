import type {
  AxialBarResult,
  Beam1dResult,
  Frame2dResult,
  HeatBar1dResult,
  HeatPlaneQuad2dResult,
  HeatPlaneTriangle2dResult,
  JobEnvelope,
  PlaneQuad2dResult,
  PlaneTriangle2dResult,
  Spring1dResult,
  Spring2dResult,
  Spring3dResult,
  ThermalBar1dResult,
  ThermalBeam1dResult,
  ThermalFrame2dResult,
  ThermalPlaneQuad2dResult,
  ThermalPlaneTriangle2dResult,
  ThermalTruss2dResult,
  ThermalTruss3dResult,
  Torsion1dResult,
  Truss2dResult,
  Truss3dResult,
} from "@/lib/api";
import {
  humanizeSolverFailure,
  type WorkbenchCopy,
  type WorkbenchLanguage,
} from "@/components/workbench/workbench-copy";

type PlaneResultField =
  | "von_mises"
  | "principal_stress_1"
  | "max_in_plane_shear"
  | "average_temperature"
  | "average_temperature_delta"
  | "temperature_gradient_x"
  | "temperature_gradient_y"
  | "heat_flux_x"
  | "heat_flux_y"
  | "heat_flux_magnitude"
  | "thermal_strain"
  | "mechanical_strain";

type LineResultField =
  | "axial_stress"
  | "max_bending_stress"
  | "max_combined_stress"
  | "moment"
  | "shear_force"
  | "average_temperature_delta"
  | "temperature_gradient_y"
  | "thermal_curvature";

const MATERIAL_COLOR_STOPS = [
  "#1677a3",
  "#ff8a3d",
  "#4a9c61",
  "#915fe2",
  "#c7547a",
  "#8a6f3b",
];

export function formatJobMessage(
  job: JobEnvelope["job"] | null,
  fallback: string,
  languageCopy: WorkbenchCopy,
) {
  if (!job) return fallback;
  if (job.status === "failed" && job.message) {
    return `${job.job_id} failed: ${
      humanizeSolverFailure(job.message, languageCopy) ?? job.message
    }`;
  }
  return `${job.job_id} ${job.status}`;
}

export function localMaterialLabel(
  value: string,
  language: WorkbenchLanguage,
): string {
  const labels = {
    en: {
      "210": "Steel",
      "70": "Aluminum",
      "116": "Titanium",
      "30": "Concrete",
      "135": "Carbon fiber",
      custom: "Custom",
    },
    zh: {
      "210": "钢",
      "70": "铝",
      "116": "钛",
      "30": "混凝土",
      "135": "碳纤维",
      custom: "自定义",
    },
  } as const;

  const localized = language === "zh" ? labels.zh : labels.en;
  return localized[value as keyof typeof localized] ?? localized.custom;
}

export function materialColorByIndex(index: number) {
  return MATERIAL_COLOR_STOPS[index % MATERIAL_COLOR_STOPS.length];
}

export function isTruss3dResult(value: unknown): value is Truss3dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Truss3dResult).nodes) &&
    (value as Truss3dResult).nodes.some((node) => "z" in node)
  );
}

export function isAxialResult(value: unknown): value is AxialBarResult {
  return typeof value === "object" && value !== null && "tip_displacement" in value;
}

export function isThermalBar1dResult(
  value: unknown,
): value is ThermalBar1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_delta" in value &&
    "max_axial_force" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

export function isHeatBar1dResult(value: unknown): value is HeatBar1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature" in value &&
    "max_heat_flux" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

export function isHeatPlaneTriangle2dResult(
  value: unknown,
): value is HeatPlaneTriangle2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature" in value &&
    "max_heat_flux" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as HeatPlaneTriangle2dResult).elements) &&
    (value as HeatPlaneTriangle2dResult).elements.every(
      (element) => !("node_l" in element),
    )
  );
}

export function isHeatPlaneQuad2dResult(
  value: unknown,
): value is HeatPlaneQuad2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature" in value &&
    "max_heat_flux" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as HeatPlaneQuad2dResult).elements) &&
    (value as HeatPlaneQuad2dResult).elements.some((element) => "node_l" in element)
  );
}

export function isThermalTruss2dResult(
  value: unknown,
): value is ThermalTruss2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_delta" in value &&
    "max_axial_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as ThermalTruss2dResult).nodes) &&
    !(value as ThermalTruss2dResult).nodes.some((node) => "z" in node)
  );
}

export function isThermalTruss3dResult(
  value: unknown,
): value is ThermalTruss3dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_delta" in value &&
    "max_axial_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as ThermalTruss3dResult).nodes) &&
    (value as ThermalTruss3dResult).nodes.some((node) => "z" in node)
  );
}

export function isTrussResult(value: unknown): value is Truss2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "nodes" in value &&
    "elements" in value &&
    !("tip_displacement" in value) &&
    Array.isArray((value as Truss2dResult).nodes) &&
    !(value as Truss2dResult).nodes.some((node) => "z" in node)
  );
}

export function isBeam1dResult(value: unknown): value is Beam1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_rotation" in value &&
    "max_moment" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Beam1dResult).nodes) &&
    !(value as Beam1dResult).nodes.some((node) => "y" in node)
  );
}

export function isThermalBeam1dResult(
  value: unknown,
): value is ThermalBeam1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_gradient" in value &&
    "max_rotation" in value &&
    "max_moment" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as ThermalBeam1dResult).nodes) &&
    !(value as ThermalBeam1dResult).nodes.some((node) => "y" in node)
  );
}

export function isTorsion1dResult(value: unknown): value is Torsion1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_torque" in value &&
    "max_rotation" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Torsion1dResult).nodes)
  );
}

export function isSpring1dResult(value: unknown): value is Spring1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Spring1dResult).nodes)
  );
}

export function isSpring2dResult(value: unknown): value is Spring2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Spring2dResult).nodes) &&
    (value as Spring2dResult).nodes.some((node) => "y" in node)
  );
}

export function isSpring3dResult(value: unknown): value is Spring3dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Spring3dResult).nodes) &&
    (value as Spring3dResult).nodes.some((node) => "z" in node)
  );
}

export function isFrame2dResult(value: unknown): value is Frame2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_rotation" in value &&
    "max_moment" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

export function isThermalFrame2dResult(
  value: unknown,
): value is ThermalFrame2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_delta" in value &&
    "max_temperature_gradient" in value &&
    "max_rotation" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

export function isPlaneResult(
  value: unknown,
): value is
  | PlaneTriangle2dResult
  | PlaneQuad2dResult
  | ThermalPlaneTriangle2dResult
  | ThermalPlaneQuad2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "elements" in value &&
    "nodes" in value &&
    "input" in value &&
    Array.isArray(
      (
        value as
          | PlaneTriangle2dResult
          | PlaneQuad2dResult
          | ThermalPlaneTriangle2dResult
          | ThermalPlaneQuad2dResult
      ).elements,
    ) &&
    (
      value as
        | PlaneTriangle2dResult
        | PlaneQuad2dResult
        | ThermalPlaneTriangle2dResult
        | ThermalPlaneQuad2dResult
    ).elements.some((element) => "node_k" in element)
  );
}

export function planeStressFill(value: number, maxValue: number): string {
  const normalized = maxValue > 0 ? Math.max(0, Math.min(1, value / maxValue)) : 0;
  const hue = 205 - normalized * 180;
  const lightness = 72 - normalized * 22;
  return `hsla(${hue}, 72%, ${lightness}%, 0.72)`;
}

export function planeResultFieldValue(
  element: {
    von_mises?: number;
    principal_stress_1?: number;
    max_in_plane_shear?: number;
    average_temperature?: number;
    average_temperature_delta?: number;
    temperature_gradient_x?: number;
    temperature_gradient_y?: number;
    heat_flux_x?: number;
    heat_flux_y?: number;
    heat_flux_magnitude?: number;
    thermal_strain?: number;
    mechanical_strain_x?: number;
    mechanical_strain_y?: number;
  },
  field: PlaneResultField,
) {
  if (field === "average_temperature") return Math.abs(element.average_temperature ?? 0);
  if (field === "average_temperature_delta") {
    return Math.abs(element.average_temperature_delta ?? 0);
  }
  if (field === "temperature_gradient_x") {
    return Math.abs(element.temperature_gradient_x ?? 0);
  }
  if (field === "temperature_gradient_y") {
    return Math.abs(element.temperature_gradient_y ?? 0);
  }
  if (field === "heat_flux_x") return Math.abs(element.heat_flux_x ?? 0);
  if (field === "heat_flux_y") return Math.abs(element.heat_flux_y ?? 0);
  if (field === "heat_flux_magnitude") {
    return Math.abs(element.heat_flux_magnitude ?? 0);
  }
  if (field === "thermal_strain") return Math.abs(element.thermal_strain ?? 0);
  if (field === "mechanical_strain") {
    return Math.max(
      Math.abs(element.mechanical_strain_x ?? 0),
      Math.abs(element.mechanical_strain_y ?? 0),
    );
  }
  if (field === "principal_stress_1") {
    return Math.abs(element.principal_stress_1 ?? 0);
  }
  if (field === "max_in_plane_shear") {
    return Math.abs(element.max_in_plane_shear ?? 0);
  }
  return Math.abs(element.von_mises ?? 0);
}

export function lineResultFieldValue(
  element: {
    axial_stress?: number;
    max_bending_stress?: number;
    max_combined_stress?: number;
    shear_force_i?: number;
    shear_force_j?: number;
    moment_i?: number;
    moment_j?: number;
    average_temperature_delta?: number;
    temperature_gradient_y?: number;
    thermal_curvature?: number;
  },
  field: LineResultField,
) {
  if (field === "axial_stress") return Math.abs(element.axial_stress ?? 0);
  if (field === "average_temperature_delta") {
    return Math.abs(element.average_temperature_delta ?? 0);
  }
  if (field === "temperature_gradient_y") {
    return Math.abs(element.temperature_gradient_y ?? 0);
  }
  if (field === "thermal_curvature") {
    return Math.abs(element.thermal_curvature ?? 0);
  }
  if (field === "max_bending_stress") {
    return Math.abs(element.max_bending_stress ?? 0);
  }
  if (field === "shear_force") {
    return Math.max(
      Math.abs(element.shear_force_i ?? 0),
      Math.abs(element.shear_force_j ?? 0),
    );
  }
  if (field === "moment") {
    return Math.max(Math.abs(element.moment_i ?? 0), Math.abs(element.moment_j ?? 0));
  }
  return Math.abs(element.max_combined_stress ?? 0);
}

export function heartbeatStatus(
  job: JobEnvelope["job"] | null,
  languageCopy: WorkbenchCopy,
) {
  if (!job?.updated_at) return "--";

  const updatedAt = new Date(job.updated_at);
  if (Number.isNaN(updatedAt.getTime())) return "--";

  const active =
    job.status === "queued" ||
    job.status === "preprocessing" ||
    job.status === "partitioning" ||
    job.status === "solving" ||
    job.status === "postprocessing";

  if (!active) return languageCopy.heartbeatHealthy;

  const ageMs = Math.max(0, Date.now() - updatedAt.getTime());
  if (ageMs < 6_000) return languageCopy.heartbeatHealthy;
  if (ageMs < 20_000) {
    return `${languageCopy.heartbeatQuiet} ${Math.round(ageMs / 1000)}s`;
  }
  return `${languageCopy.heartbeatStale} ${Math.round(ageMs / 1000)}s`;
}

export function heartbeatTone(
  job: JobEnvelope["job"] | null,
): "healthy" | "quiet" | "stale" {
  if (!job?.updated_at) return "quiet";

  const updatedAt = new Date(job.updated_at);
  if (Number.isNaN(updatedAt.getTime())) return "quiet";

  const active =
    job.status === "queued" ||
    job.status === "preprocessing" ||
    job.status === "partitioning" ||
    job.status === "solving" ||
    job.status === "postprocessing";

  if (!active) return "healthy";

  const ageMs = Math.max(0, Date.now() - updatedAt.getTime());
  if (ageMs < 6_000) return "healthy";
  if (ageMs < 20_000) return "quiet";
  return "stale";
}

export function formatProtocolMethodLabel(method: string) {
  return method.replaceAll("_", " ");
}

export function clusterHealthTone(score: number | null | undefined) {
  if (score == null) return "quiet";
  if (score >= 85) return "healthy";
  if (score >= 55) return "watch";
  return "stale";
}

export function formatPeerStatus(
  status: string | undefined,
  languageCopy: WorkbenchCopy,
) {
  if (!status) return "--";
  switch (status) {
    case "healthy":
      return languageCopy.heartbeatHealthy;
    case "degraded":
      return languageCopy.heartbeatQuiet;
    case "unreachable":
      return languageCopy.heartbeatStale;
    case "seed":
      return languageCopy.ready;
    default:
      return status.replaceAll("_", " ");
  }
}
