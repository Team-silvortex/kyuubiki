import { buildStudyModelPayload } from "@/lib/models";
import type {
  AxialBarJobInput,
  AxialBarResult,
  Beam1dJobInput,
  Beam1dResult,
  Frame2dJobInput,
  Frame2dResult,
  FrontendRuntimeMode,
  JobEnvelope,
  PlaneQuad2dJobInput,
  PlaneQuad2dResult,
  PlaneTriangle2dJobInput,
  PlaneTriangle2dResult,
  ThermalBeam1dJobInput,
  ThermalBeam1dResult,
  ThermalBar1dJobInput,
  ThermalBar1dResult,
  ThermalFrame2dJobInput,
  ThermalFrame2dResult,
  ThermalTruss2dJobInput,
  ThermalTruss2dResult,
  ThermalTruss3dJobInput,
  ThermalTruss3dResult,
  Spring1dJobInput,
  Spring1dResult,
  Spring2dJobInput,
  Spring2dResult,
  Spring3dJobInput,
  Spring3dResult,
  Torsion1dJobInput,
  Torsion1dResult,
  Truss2dJobInput,
  Truss2dResult,
  Truss3dJobInput,
  Truss3dResult,
} from "@/lib/api";

export const WORKBENCH_SETTINGS_KEY = "kyuubiki-workbench-settings";
export const WORKBENCH_SECRETS_KEY = "kyuubiki-workbench-secrets";

export type StoredWorkbenchSettings = {
  theme?: string;
  language?: string;
  showShortcutHints?: boolean;
  immersiveGuardrails?: boolean;
  frontendRuntimeMode?: FrontendRuntimeMode;
  directMeshEndpointsText?: string;
  directMeshSelectionMode?: string;
  assistantMode?: string;
  assistantApiBaseUrl?: string;
  assistantModel?: string;
  controlPlaneApiToken?: string;
  clusterApiToken?: string;
  directMeshApiToken?: string;
  assistantApiKey?: string;
};

type PersistedWorkbenchSettings = {
  theme?: string;
  language?: string;
  showShortcutHints?: boolean;
  immersiveGuardrails?: boolean;
  frontendRuntimeMode?: FrontendRuntimeMode;
  directMeshEndpointsText?: string;
  directMeshSelectionMode?: string;
  assistantMode?: string;
  assistantApiBaseUrl?: string;
  assistantModel?: string;
};

type StoredWorkbenchSecrets = {
  controlPlaneApiToken?: string;
  clusterApiToken?: string;
  directMeshApiToken?: string;
  assistantApiKey?: string;
};

export type WorkbenchSettingsInput = {
  theme: string;
  language: string;
  showShortcutHints: boolean;
  immersiveGuardrails: boolean;
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshEndpointsText: string;
  directMeshSelectionMode: string;
  controlPlaneApiToken: string;
  clusterApiToken: string;
  directMeshApiToken: string;
  assistantMode: string;
  assistantApiBaseUrl: string;
  assistantApiKey: string;
  assistantModel: string;
};

type StudyKind = "axial_bar_1d" | "thermal_bar_1d" | "thermal_beam_1d" | "thermal_frame_2d" | "thermal_truss_2d" | "thermal_truss_3d" | "spring_1d" | "spring_2d" | "spring_3d" | "beam_1d" | "torsion_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d" | "plane_quad_2d" | "frame_2d";

type AxialFormLike = {
  length: number;
  area: number;
  elements: number;
  tipForce: number;
  youngsModulusGpa: number;
};

type ParametricLike = {
  youngsModulusGpa: number;
};

export function safeStorageGet(): StoredWorkbenchSettings {
  if (typeof window === "undefined") return {};

  try {
    const rawSettings = window.localStorage.getItem(WORKBENCH_SETTINGS_KEY);
    const parsedSettings = rawSettings ? (JSON.parse(rawSettings) as StoredWorkbenchSettings) : {};

    const rawSecrets = window.sessionStorage.getItem(WORKBENCH_SECRETS_KEY);
    const parsedSecrets = rawSecrets ? (JSON.parse(rawSecrets) as StoredWorkbenchSecrets) : {};

    const legacySecrets: StoredWorkbenchSecrets = {
      ...(parsedSettings.controlPlaneApiToken ? { controlPlaneApiToken: parsedSettings.controlPlaneApiToken } : {}),
      ...(parsedSettings.clusterApiToken ? { clusterApiToken: parsedSettings.clusterApiToken } : {}),
      ...(parsedSettings.directMeshApiToken ? { directMeshApiToken: parsedSettings.directMeshApiToken } : {}),
      ...(parsedSettings.assistantApiKey ? { assistantApiKey: parsedSettings.assistantApiKey } : {}),
    };

    const mergedSecrets =
      Object.keys(parsedSecrets).length > 0
        ? parsedSecrets
        : Object.keys(legacySecrets).length > 0
          ? legacySecrets
          : {};

    if (Object.keys(legacySecrets).length > 0) {
      window.sessionStorage.setItem(WORKBENCH_SECRETS_KEY, JSON.stringify(mergedSecrets));

      const sanitizedSettings: PersistedWorkbenchSettings = {
        theme: parsedSettings.theme,
        language: parsedSettings.language,
        showShortcutHints: parsedSettings.showShortcutHints,
        immersiveGuardrails: parsedSettings.immersiveGuardrails,
        frontendRuntimeMode: parsedSettings.frontendRuntimeMode,
        directMeshEndpointsText: parsedSettings.directMeshEndpointsText,
        directMeshSelectionMode: parsedSettings.directMeshSelectionMode,
        assistantMode: parsedSettings.assistantMode,
        assistantApiBaseUrl: parsedSettings.assistantApiBaseUrl,
        assistantModel: parsedSettings.assistantModel,
      };

      window.localStorage.setItem(WORKBENCH_SETTINGS_KEY, JSON.stringify(sanitizedSettings));
    }

    return {
      ...parsedSettings,
      ...mergedSecrets,
    };
  } catch {
    return {};
  }
}

export function sanitizeWorkbenchSettings(input: WorkbenchSettingsInput): PersistedWorkbenchSettings {
  return {
    theme: input.theme,
    language: input.language,
    showShortcutHints: input.showShortcutHints,
    immersiveGuardrails: input.immersiveGuardrails,
    frontendRuntimeMode: input.frontendRuntimeMode,
    directMeshEndpointsText: input.directMeshEndpointsText,
    directMeshSelectionMode: input.directMeshSelectionMode,
    assistantMode: input.assistantMode,
    assistantApiBaseUrl: input.assistantApiBaseUrl.trim(),
    assistantModel: input.assistantModel.trim(),
  };
}

export function sanitizeWorkbenchSecrets(input: WorkbenchSettingsInput): StoredWorkbenchSecrets {
  return {
    ...(input.controlPlaneApiToken.trim()
      ? { controlPlaneApiToken: input.controlPlaneApiToken.trim() }
      : {}),
    ...(input.clusterApiToken.trim() ? { clusterApiToken: input.clusterApiToken.trim() } : {}),
    ...(input.directMeshApiToken.trim()
      ? { directMeshApiToken: input.directMeshApiToken.trim() }
      : {}),
    ...(input.assistantApiKey.trim() ? { assistantApiKey: input.assistantApiKey.trim() } : {}),
  };
}

export function persistWorkbenchSettings(input: WorkbenchSettingsInput) {
  if (typeof window === "undefined") return;

  window.localStorage.setItem(
    WORKBENCH_SETTINGS_KEY,
    JSON.stringify(sanitizeWorkbenchSettings(input)),
  );
  window.sessionStorage.setItem(
    WORKBENCH_SECRETS_KEY,
    JSON.stringify(sanitizeWorkbenchSecrets(input)),
  );
}

export function parseDirectMeshEndpoints(value: string) {
  return [...new Set(value.split(/[\n,]+/).map((entry) => entry.trim()).filter(Boolean))];
}

export function toAxialInput(form: AxialFormLike): AxialBarJobInput {
  return {
    length: form.length,
    area: form.area,
    elements: form.elements,
    tip_force: form.tipForce,
    youngs_modulus_gpa: form.youngsModulusGpa,
  };
}

export function serializeCurrentModel(
  studyKind: StudyKind,
  loadedModelName: string,
  activeMaterial: string,
  axialForm: AxialFormLike,
  thermalBarModel: ThermalBar1dJobInput,
  thermalBeamModel: ThermalBeam1dJobInput,
  thermalFrameModel: ThermalFrame2dJobInput,
  thermalTrussModel: ThermalTruss2dJobInput,
  trussModel: Truss2dJobInput,
  thermalTruss3dModel: ThermalTruss3dJobInput,
  truss3dModel: Truss3dJobInput,
  planeModel: PlaneTriangle2dJobInput | PlaneQuad2dJobInput,
  frameModel: Frame2dJobInput,
  beamModel: Beam1dJobInput,
  torsionModel: Torsion1dJobInput,
  springModel: Spring1dJobInput,
  spring2dModel: Spring2dJobInput,
  spring3dModel: Spring3dJobInput,
  parametric: ParametricLike,
  round: (value: number) => number,
): Record<string, unknown> {
  return buildStudyModelPayload(studyKind, {
    name: loadedModelName,
    material: activeMaterial,
    youngsModulusGpa:
      studyKind === "axial_bar_1d"
        ? axialForm.youngsModulusGpa
        : studyKind === "thermal_bar_1d"
          ? round((thermalBarModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
        : studyKind === "thermal_beam_1d"
          ? round((thermalBeamModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
        : studyKind === "thermal_frame_2d"
          ? round((thermalFrameModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
        : studyKind === "thermal_truss_2d"
          ? round((thermalTrussModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
        : studyKind === "truss_2d"
          ? parametric.youngsModulusGpa
          : studyKind === "thermal_truss_3d"
            ? round((thermalTruss3dModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
          : studyKind === "truss_3d"
            ? round((truss3dModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
            : studyKind === "beam_1d"
              ? round((beamModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
            : studyKind === "torsion_1d"
              ? 0
            : studyKind === "frame_2d"
              ? round((frameModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
            : round((planeModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9),
    materials:
      studyKind === "truss_2d"
        ? trussModel.materials
        : studyKind === "thermal_truss_2d"
          ? thermalTrussModel.materials
        : studyKind === "thermal_beam_1d"
          ? thermalBeamModel.materials
        : studyKind === "thermal_frame_2d"
          ? thermalFrameModel.materials
        : studyKind === "truss_3d"
          ? truss3dModel.materials
          : studyKind === "thermal_truss_3d"
            ? thermalTruss3dModel.materials
          : studyKind === "spring_1d" || studyKind === "spring_2d" || studyKind === "spring_3d"
            ? undefined
          : studyKind === "beam_1d"
            ? beamModel.materials
          : studyKind === "torsion_1d"
            ? undefined
          : studyKind === "frame_2d"
            ? frameModel.materials
          : studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d"
            ? planeModel.materials
            : undefined,
    axial: studyKind === "axial_bar_1d" ? toAxialInput(axialForm) : undefined,
    truss: studyKind === "truss_2d" ? trussModel : undefined,
    thermalTruss: studyKind === "thermal_truss_2d" ? thermalTrussModel : undefined,
    truss3d: studyKind === "truss_3d" ? truss3dModel : undefined,
    thermalTruss3d: studyKind === "thermal_truss_3d" ? thermalTruss3dModel : undefined,
    plane: studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d" ? planeModel : undefined,
    frame: studyKind === "frame_2d" ? frameModel : undefined,
    thermalFrame: studyKind === "thermal_frame_2d" ? thermalFrameModel : undefined,
    beam: studyKind === "beam_1d" ? beamModel : undefined,
    thermalBeam: studyKind === "thermal_beam_1d" ? thermalBeamModel : undefined,
    torsion: studyKind === "torsion_1d" ? torsionModel : undefined,
    thermalBar: studyKind === "thermal_bar_1d" ? thermalBarModel : undefined,
    spring: studyKind === "spring_1d" ? springModel : undefined,
    spring2d: studyKind === "spring_2d" ? spring2dModel : undefined,
    spring3d: studyKind === "spring_3d" ? spring3dModel : undefined,
  });
}

function toCsvRow(values: Array<string | number | boolean | null | undefined>) {
  return values
    .map((value) => {
      const text = value === null || value === undefined ? "" : String(value);
      if (text.includes(",") || text.includes("\"") || text.includes("\n")) {
        return `"${text.replaceAll("\"", "\"\"")}"`;
      }
      return text;
    })
    .join(",");
}

function isAxialResult(value: unknown): value is AxialBarResult {
  return typeof value === "object" && value !== null && "tip_displacement" in value;
}

function isThermalBar1dResult(value: unknown): value is ThermalBar1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_delta" in value &&
    "max_axial_force" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

function isThermalBeam1dResult(value: unknown): value is ThermalBeam1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_gradient" in value &&
    "max_rotation" in value &&
    "max_moment" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

function isThermalFrame2dResult(value: unknown): value is ThermalFrame2dResult {
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

function isThermalTruss2dResult(value: unknown): value is ThermalTruss2dResult {
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

function isThermalTruss3dResult(value: unknown): value is ThermalTruss3dResult {
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

function isTruss3dResult(value: unknown): value is Truss3dResult {
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && Array.isArray((value as Truss3dResult).nodes) && (value as Truss3dResult).nodes.some((node) => "z" in node);
}

function isTrussResult(value: unknown): value is Truss2dResult {
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && !("tip_displacement" in value) && Array.isArray((value as Truss2dResult).nodes) && !(value as Truss2dResult).nodes.some((node) => "z" in node);
}

function isFrame2dResult(value: unknown): value is Frame2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_rotation" in value &&
    "max_moment" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

function isBeam1dResult(value: unknown): value is Beam1dResult {
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

function isSpring1dResult(value: unknown): value is Spring1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Spring1dResult).nodes)
  );
}

function isSpring2dResult(value: unknown): value is Spring2dResult {
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

function isSpring3dResult(value: unknown): value is Spring3dResult {
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

function isTorsion1dResult(value: unknown): value is Torsion1dResult {
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

export function serializeResultCsv(
  studyKind: StudyKind,
  job: JobEnvelope["job"] | null,
  result: AxialBarResult | ThermalBar1dResult | ThermalBeam1dResult | ThermalFrame2dResult | ThermalTruss2dResult | ThermalTruss3dResult | Spring1dResult | Spring2dResult | Spring3dResult | Beam1dResult | Torsion1dResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult | PlaneQuad2dResult | Frame2dResult | null,
) {
  if (!result) return "";

  const lines: string[] = [];
  lines.push("meta");
  lines.push(toCsvRow(["study_kind", studyKind]));
  lines.push(toCsvRow(["job_id", job?.job_id]));
  lines.push(toCsvRow(["status", job?.status]));
  lines.push(toCsvRow(["worker_id", job?.worker_id]));
  lines.push("");

  if (isAxialResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "x", "displacement"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.x, node.displacement])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "x1", "x2", "strain", "stress", "axial_force"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.x1, element.x2, element.strain, element.stress, element.axial_force])),
    );
    return lines.join("\n");
  }

  if (isThermalBar1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "ux", "temperature_delta"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.ux, node.temperature_delta])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "average_temperature_delta",
        "thermal_strain",
        "mechanical_strain",
        "total_strain",
        "stress",
        "axial_force",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.average_temperature_delta,
          element.thermal_strain,
          element.mechanical_strain,
          element.total_strain,
          element.stress,
          element.axial_force,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isThermalTruss3dResult(result)) {
    const thermalResult = result as ThermalTruss3dResult;
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "z", "ux", "uy", "uz", "temperature_delta"]));
    thermalResult.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.z, node.ux, node.uy, node.uz, node.temperature_delta])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "average_temperature_delta",
        "thermal_strain",
        "mechanical_strain",
        "total_strain",
        "stress",
        "axial_force",
      ]),
    );
    thermalResult.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.average_temperature_delta,
          element.thermal_strain,
          element.mechanical_strain,
          element.total_strain,
          element.stress,
          element.axial_force,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isTruss3dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "z", "ux", "uy", "uz"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.z, node.ux, node.uy, node.uz])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "strain", "stress", "axial_force"]));
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.strain, element.stress, element.axial_force]),
      ),
    );
    return lines.join("\n");
  }

  if (isThermalTruss2dResult(result)) {
    const thermalResult = result as ThermalTruss2dResult;
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy", "temperature_delta"]));
    thermalResult.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy, node.temperature_delta])));
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "average_temperature_delta",
        "thermal_strain",
        "mechanical_strain",
        "total_strain",
        "stress",
        "axial_force",
      ]),
    );
    thermalResult.elements.forEach((element) =>
      lines.push(
        toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.average_temperature_delta, element.thermal_strain, element.mechanical_strain, element.total_strain, element.stress, element.axial_force]),
      ),
    );
    return lines.join("\n");
  }

  if (isTrussResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "strain", "stress", "axial_force"]));
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.strain, element.stress, element.axial_force]),
      ),
    );
    return lines.join("\n");
  }

  if (isFrame2dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy", "rz", "displacement_magnitude"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy, node.rz, node.displacement_magnitude])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "axial_force_i",
        "shear_force_i",
        "moment_i",
        "axial_force_j",
        "shear_force_j",
        "moment_j",
        "axial_stress",
        "max_bending_stress",
        "max_combined_stress",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.axial_force_i,
          element.shear_force_i,
          element.moment_i,
          element.axial_force_j,
          element.shear_force_j,
          element.moment_j,
          element.axial_stress,
          element.max_bending_stress,
          element.max_combined_stress,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isThermalFrame2dResult(result)) {
    const thermalFrameResult = result as ThermalFrame2dResult;
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy", "rz", "displacement_magnitude", "temperature_delta"]));
    thermalFrameResult.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy, node.rz, node.displacement_magnitude, node.temperature_delta])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "average_temperature_delta",
        "thermal_strain",
        "mechanical_strain",
        "total_strain",
        "temperature_gradient_y",
        "thermal_curvature",
        "axial_force_i",
        "shear_force_i",
        "moment_i",
        "axial_force_j",
        "shear_force_j",
        "moment_j",
        "axial_stress",
        "max_bending_stress",
        "max_combined_stress",
      ]),
    );
    thermalFrameResult.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.average_temperature_delta,
          element.thermal_strain,
          element.mechanical_strain,
          element.total_strain,
          element.temperature_gradient_y,
          element.thermal_curvature,
          element.axial_force_i,
          element.shear_force_i,
          element.moment_i,
          element.axial_force_j,
          element.shear_force_j,
          element.moment_j,
          element.axial_stress,
          element.max_bending_stress,
          element.max_combined_stress,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isBeam1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "uy", "rz", "displacement_magnitude"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.uy, node.rz, node.displacement_magnitude])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "shear_force_i",
        "moment_i",
        "shear_force_j",
        "moment_j",
        "max_bending_stress",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.shear_force_i,
          element.moment_i,
          element.shear_force_j,
          element.moment_j,
          element.max_bending_stress,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isThermalBeam1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "uy", "rz", "displacement_magnitude"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.uy, node.rz, node.displacement_magnitude])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "temperature_gradient_y",
        "thermal_curvature",
        "shear_force_i",
        "moment_i",
        "shear_force_j",
        "moment_j",
        "max_bending_stress",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.temperature_gradient_y,
          element.thermal_curvature,
          element.shear_force_i,
          element.moment_i,
          element.shear_force_j,
          element.moment_j,
          element.max_bending_stress,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isTorsion1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "rz"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.rz])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "twist", "torque", "shear_stress"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.twist, element.torque, element.shear_stress])),
    );
    return lines.join("\n");
  }

  if (isSpring1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "ux"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.ux])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "extension", "force"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.extension, element.force])),
    );
    return lines.join("\n");
  }

  if (isSpring2dResult(result)) {
    const spring2d = result as Spring2dResult;
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy"]));
    spring2d.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "extension", "force"]));
    spring2d.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.extension, element.force])),
    );
    return lines.join("\n");
  }

  if (isSpring3dResult(result)) {
    const spring3d = result as Spring3dResult;
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "z", "ux", "uy", "uz"]));
    spring3d.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.z, node.ux, node.uy, node.uz])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "extension", "force"]));
    spring3d.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.extension, element.force])),
    );
    return lines.join("\n");
  }

  const plane = result as PlaneTriangle2dResult | PlaneQuad2dResult;
  lines.push("nodes");
  lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy", "displacement_magnitude"]));
  plane.nodes.forEach((node) =>
    lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy, node.displacement_magnitude])),
  );
  lines.push("");
  lines.push("elements");
  lines.push(
    toCsvRow([
      "index",
      "id",
      "node_i",
      "node_j",
      "node_k",
      "area",
      "strain_x",
      "strain_y",
      "gamma_xy",
      "stress_x",
      "stress_y",
      "tau_xy",
      "principal_stress_1",
      "principal_stress_2",
      "max_in_plane_shear",
      "von_mises",
    ]),
  );
  plane.elements.forEach((element) =>
    lines.push(
      toCsvRow([
        element.index,
        element.id,
        element.node_i,
        element.node_j,
        element.node_k,
        element.area,
        element.strain_x,
        element.strain_y,
        element.gamma_xy,
        element.stress_x,
        element.stress_y,
        element.tau_xy,
        element.principal_stress_1,
        element.principal_stress_2,
        element.max_in_plane_shear,
        element.von_mises,
      ]),
    ),
  );
  return lines.join("\n");
}

export function scientific(value: number | null | undefined, digits = 3): string {
  return typeof value === "number" ? value.toExponential(digits) : "--";
}

export function fixed(value: number | null | undefined, digits = 2): string {
  return typeof value === "number" ? value.toFixed(digits) : "--";
}

export function formatTime(value: string | undefined, language: string): string {
  if (!value) return "--";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : "en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function formatMilliseconds(value: number | null | undefined) {
  if (typeof value !== "number" || Number.isNaN(value)) return "--";
  if (value >= 1000) {
    return `${(value / 1000).toFixed(value % 1000 === 0 ? 0 : 1)}s`;
  }
  return `${Math.round(value)} ms`;
}
