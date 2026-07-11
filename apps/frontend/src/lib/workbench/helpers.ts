import { buildStudyModelPayload } from "@/lib/models/modeler";
import {
  buildWorkbenchGovernanceConfig,
  normalizeWorkbenchGovernanceRuntime,
  type WorkbenchGovernanceConfig,
} from "@/lib/workbench/governance";
import { mergeLanguagePack } from "@/lib/workbench/language-pack-merge";
import {
  readInMemoryWorkbenchSecrets,
  scrubPersistedWorkbenchSecrets,
  WORKBENCH_SECRETS_KEY,
  writeInMemoryWorkbenchSecrets,
  type StoredWorkbenchSecrets,
} from "@/lib/workbench/workbench-secrets";
import type {
  AxialBarJobInput,
  Beam1dJobInput,
  ElectrostaticPlaneQuad2dJobInput,
  ElectrostaticPlaneTriangle2dJobInput,
  Frame2dJobInput,
  FrontendRuntimeMode,
  HeatBar1dJobInput,
  HeatPlaneQuad2dJobInput,
  HeatPlaneTriangle2dJobInput,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
  ThermalBeam1dJobInput,
  ThermalBar1dJobInput,
  ThermalFrame2dJobInput,
  ThermalTruss2dJobInput,
  ThermalTruss3dJobInput,
  Spring1dJobInput,
  Spring2dJobInput,
  Spring3dJobInput,
  Torsion1dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
} from "@/lib/api";

export const WORKBENCH_SETTINGS_KEY = "kyuubiki-workbench-settings";
export const WORKBENCH_LANGUAGE_PACKS_KEY = "kyuubiki-workbench-language-packs";
export const WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION = "kyuubiki.language-pack/v1";
export const WORKBENCH_LANGUAGE_PACK_VERSION_LINE = "tamamono 1.x";
export const WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION = "1.18.0";

export type WorkbenchLanguagePackCompatibility = "exact" | "line" | "unscoped" | "mismatch";
export { mergeLanguagePack };
export {
  readInMemoryWorkbenchSecrets,
  WORKBENCH_SECRETS_KEY,
  writeInMemoryWorkbenchSecrets,
};

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
  governanceConfig?: WorkbenchGovernanceConfig;
};

export type WorkbenchLanguagePack = {
  schema_version: string;
  id: string;
  language: string;
  targetSurface?: "workbench";
  name: string;
  version: string;
  versionLine?: string;
  targetAppVersion?: string;
  source: "imported" | "downloaded";
  updatedAt: string;
  description?: string;
  overrides: Record<string, unknown>;
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
  governanceConfig?: WorkbenchGovernanceConfig;
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

function sanitizeStoredSettings(input: StoredWorkbenchSettings): PersistedWorkbenchSettings {
  return {
    theme: input.theme,
    language: input.language,
    showShortcutHints: input.showShortcutHints,
    immersiveGuardrails: input.immersiveGuardrails,
    frontendRuntimeMode: input.frontendRuntimeMode,
    directMeshEndpointsText: input.directMeshEndpointsText,
    directMeshSelectionMode: input.directMeshSelectionMode,
    assistantMode: input.assistantMode,
    assistantApiBaseUrl: input.assistantApiBaseUrl,
    assistantModel: input.assistantModel,
    governanceConfig: input.governanceConfig,
  };
}

type StudyKind = "axial_bar_1d" | "heat_bar_1d" | "electrostatic_plane_triangle_2d" | "electrostatic_plane_quad_2d" | "heat_plane_triangle_2d" | "heat_plane_quad_2d" | "thermal_bar_1d" | "thermal_beam_1d" | "thermal_frame_2d" | "thermal_truss_2d" | "thermal_truss_3d" | "thermal_plane_triangle_2d" | "thermal_plane_quad_2d" | "spring_1d" | "spring_2d" | "spring_3d" | "beam_1d" | "torsion_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d" | "plane_quad_2d" | "frame_2d";

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
    const persistedSessionSecrets = rawSecrets ? (JSON.parse(rawSecrets) as StoredWorkbenchSecrets) : {};

    const legacySecrets: StoredWorkbenchSecrets = {
      ...(parsedSettings.controlPlaneApiToken ? { controlPlaneApiToken: parsedSettings.controlPlaneApiToken } : {}),
      ...(parsedSettings.clusterApiToken ? { clusterApiToken: parsedSettings.clusterApiToken } : {}),
      ...(parsedSettings.directMeshApiToken ? { directMeshApiToken: parsedSettings.directMeshApiToken } : {}),
      ...(parsedSettings.assistantApiKey ? { assistantApiKey: parsedSettings.assistantApiKey } : {}),
    };

    if (Object.keys(persistedSessionSecrets).length > 0) {
      writeInMemoryWorkbenchSecrets(persistedSessionSecrets);
    }

    if (Object.keys(legacySecrets).length > 0) {
      writeInMemoryWorkbenchSecrets(legacySecrets);
      window.localStorage.setItem(WORKBENCH_SETTINGS_KEY, JSON.stringify(sanitizeStoredSettings(parsedSettings)));
    }

    scrubPersistedWorkbenchSecrets();

    const normalized = normalizeWorkbenchGovernanceRuntime({
      frontendRuntimeMode: parsedSettings.frontendRuntimeMode ?? "orchestrated_gui",
      directMeshEndpointsText: parsedSettings.directMeshEndpointsText ?? "",
    });

    return {
      ...sanitizeStoredSettings(parsedSettings),
      frontendRuntimeMode: normalized.frontendRuntimeMode,
      directMeshEndpointsText: normalized.directMeshEndpointsText,
      ...readInMemoryWorkbenchSecrets(),
    };
  } catch {
    return {};
  }
}

export function sanitizeWorkbenchSettings(input: WorkbenchSettingsInput): PersistedWorkbenchSettings {
  const normalized = normalizeWorkbenchGovernanceRuntime({
    frontendRuntimeMode: input.frontendRuntimeMode,
    directMeshEndpointsText: input.directMeshEndpointsText,
  });

  return {
    theme: input.theme,
    language: input.language,
    showShortcutHints: input.showShortcutHints,
    immersiveGuardrails: input.immersiveGuardrails,
    frontendRuntimeMode: normalized.frontendRuntimeMode,
    directMeshEndpointsText: normalized.directMeshEndpointsText,
    directMeshSelectionMode: input.directMeshSelectionMode,
    assistantMode: input.assistantMode,
    assistantApiBaseUrl: input.assistantApiBaseUrl.trim(),
    assistantModel: input.assistantModel.trim(),
    governanceConfig: buildWorkbenchGovernanceConfig({
      frontendRuntimeMode: normalized.frontendRuntimeMode,
      directMeshEndpointsText: normalized.directMeshEndpointsText,
      controlPlaneApiToken: input.controlPlaneApiToken,
      clusterApiToken: input.clusterApiToken,
      directMeshApiToken: input.directMeshApiToken,
    }),
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
  writeInMemoryWorkbenchSecrets(sanitizeWorkbenchSecrets(input));
  scrubPersistedWorkbenchSecrets();
}

export function readWorkbenchLanguagePacks(): WorkbenchLanguagePack[] {
  if (typeof window === "undefined") return [];

  try {
    const raw = window.localStorage.getItem(WORKBENCH_LANGUAGE_PACKS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((entry): entry is WorkbenchLanguagePack => {
      return (
        entry &&
        typeof entry === "object" &&
        typeof entry.id === "string" &&
        typeof entry.schema_version === "string" &&
        typeof entry.language === "string" &&
        (entry.targetSurface === undefined || entry.targetSurface === "workbench") &&
        typeof entry.name === "string" &&
        typeof entry.version === "string" &&
        (entry.versionLine === undefined || typeof entry.versionLine === "string") &&
        (entry.targetAppVersion === undefined || typeof entry.targetAppVersion === "string") &&
        (entry.source === "imported" || entry.source === "downloaded") &&
        typeof entry.updatedAt === "string" &&
        entry.overrides &&
        typeof entry.overrides === "object" &&
        !Array.isArray(entry.overrides)
      );
    });
  } catch {
    return [];
  }
}

export function persistWorkbenchLanguagePacks(packs: WorkbenchLanguagePack[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(WORKBENCH_LANGUAGE_PACKS_KEY, JSON.stringify(packs));
}

export function getWorkbenchLanguagePackCompatibility(
  pack: Pick<WorkbenchLanguagePack, "versionLine" | "targetAppVersion">,
): WorkbenchLanguagePackCompatibility {
  const versionLine = pack.versionLine?.trim();
  const targetAppVersion = pack.targetAppVersion?.trim();

  if (targetAppVersion && targetAppVersion !== WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION) {
    return "mismatch";
  }

  if (versionLine && versionLine !== WORKBENCH_LANGUAGE_PACK_VERSION_LINE) {
    return "mismatch";
  }

  if (targetAppVersion === WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION) {
    return "exact";
  }

  if (versionLine === WORKBENCH_LANGUAGE_PACK_VERSION_LINE) {
    return "line";
  }

  return "unscoped";
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
  heatBarModel: HeatBar1dJobInput,
  heatPlaneModel: HeatPlaneTriangle2dJobInput | HeatPlaneQuad2dJobInput,
  thermalBarModel: ThermalBar1dJobInput,
  thermalBeamModel: ThermalBeam1dJobInput,
  thermalFrameModel: ThermalFrame2dJobInput,
  thermalTrussModel: ThermalTruss2dJobInput,
  trussModel: Truss2dJobInput,
  thermalTruss3dModel: ThermalTruss3dJobInput,
  truss3dModel: Truss3dJobInput,
  planeModel:
    | ElectrostaticPlaneTriangle2dJobInput
    | ElectrostaticPlaneQuad2dJobInput
    | PlaneTriangle2dJobInput
    | PlaneQuad2dJobInput
    | ThermalPlaneTriangle2dJobInput
    | ThermalPlaneQuad2dJobInput,
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
        : studyKind === "heat_bar_1d"
          ? 0
        : studyKind === "electrostatic_plane_triangle_2d" || studyKind === "electrostatic_plane_quad_2d"
          ? 0
        : studyKind === "heat_plane_triangle_2d" || studyKind === "heat_plane_quad_2d"
          ? 0
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
            : round((((planeModel.elements[0] as { youngs_modulus?: number } | undefined)?.youngs_modulus ?? 0)) / 1.0e9),
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
          : studyKind === "heat_plane_triangle_2d" || studyKind === "heat_plane_quad_2d"
            ? heatPlaneModel.materials
          : studyKind === "electrostatic_plane_triangle_2d" || studyKind === "electrostatic_plane_quad_2d"
            ? planeModel.materials
          : studyKind === "plane_triangle_2d" ||
            studyKind === "plane_quad_2d" ||
            studyKind === "thermal_plane_triangle_2d" ||
            studyKind === "thermal_plane_quad_2d"
            ? planeModel.materials
            : undefined,
    axial: studyKind === "axial_bar_1d" ? toAxialInput(axialForm) : undefined,
    truss: studyKind === "truss_2d" ? trussModel : undefined,
    thermalTruss: studyKind === "thermal_truss_2d" ? thermalTrussModel : undefined,
    truss3d: studyKind === "truss_3d" ? truss3dModel : undefined,
    thermalTruss3d: studyKind === "thermal_truss_3d" ? thermalTruss3dModel : undefined,
    plane:
      studyKind === "heat_plane_triangle_2d" ||
      studyKind === "heat_plane_quad_2d"
        ? heatPlaneModel
      : studyKind === "electrostatic_plane_triangle_2d" ||
        studyKind === "electrostatic_plane_quad_2d"
        ? planeModel
      : studyKind === "plane_triangle_2d" ||
      studyKind === "plane_quad_2d" ||
      studyKind === "thermal_plane_triangle_2d" ||
      studyKind === "thermal_plane_quad_2d"
        ? planeModel
        : undefined,
    frame: studyKind === "frame_2d" ? frameModel : undefined,
    thermalFrame: studyKind === "thermal_frame_2d" ? thermalFrameModel : undefined,
    beam: studyKind === "beam_1d" ? beamModel : undefined,
    thermalBeam: studyKind === "thermal_beam_1d" ? thermalBeamModel : undefined,
    torsion: studyKind === "torsion_1d" ? torsionModel : undefined,
    heatBar: studyKind === "heat_bar_1d" ? heatBarModel : undefined,
    thermalBar: studyKind === "thermal_bar_1d" ? thermalBarModel : undefined,
    spring: studyKind === "spring_1d" ? springModel : undefined,
    spring2d: studyKind === "spring_2d" ? spring2dModel : undefined,
    spring3d: studyKind === "spring_3d" ? spring3dModel : undefined,
  });
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
  const locale = language === "zh" ? "zh-CN" : language === "ja" ? "ja-JP" : language === "es" ? "es-ES" : "en-US";
  return new Intl.DateTimeFormat(locale, {
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
