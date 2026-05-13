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

type StudyKind = "axial_bar_1d" | "beam_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d" | "plane_quad_2d" | "frame_2d";

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
  trussModel: Truss2dJobInput,
  truss3dModel: Truss3dJobInput,
  planeModel: PlaneTriangle2dJobInput | PlaneQuad2dJobInput,
  frameModel: Frame2dJobInput,
  beamModel: Beam1dJobInput,
  parametric: ParametricLike,
  round: (value: number) => number,
): Record<string, unknown> {
  return buildStudyModelPayload(studyKind, {
    name: loadedModelName,
    material: activeMaterial,
    youngsModulusGpa:
      studyKind === "axial_bar_1d"
        ? axialForm.youngsModulusGpa
        : studyKind === "truss_2d"
          ? parametric.youngsModulusGpa
          : studyKind === "truss_3d"
            ? round((truss3dModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
            : studyKind === "beam_1d"
              ? round((beamModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
            : studyKind === "frame_2d"
              ? round((frameModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
            : round((planeModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9),
    materials:
      studyKind === "truss_2d"
        ? trussModel.materials
        : studyKind === "truss_3d"
          ? truss3dModel.materials
          : studyKind === "beam_1d"
            ? beamModel.materials
          : studyKind === "frame_2d"
            ? frameModel.materials
          : studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d"
            ? planeModel.materials
            : undefined,
    axial: studyKind === "axial_bar_1d" ? toAxialInput(axialForm) : undefined,
    truss: studyKind === "truss_2d" ? trussModel : undefined,
    truss3d: studyKind === "truss_3d" ? truss3dModel : undefined,
    plane: studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d" ? planeModel : undefined,
    frame: studyKind === "frame_2d" ? frameModel : undefined,
    beam: studyKind === "beam_1d" ? beamModel : undefined,
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

export function serializeResultCsv(
  studyKind: StudyKind,
  job: JobEnvelope["job"] | null,
  result: AxialBarResult | Beam1dResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult | PlaneQuad2dResult | Frame2dResult | null,
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

  lines.push("nodes");
  lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy", "displacement_magnitude"]));
  result.nodes.forEach((node) =>
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
  result.elements.forEach((element) =>
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
