"use client";

import { resolveWorkflowFemInputProfile } from "@/components/workbench/workflow/workbench-workflow-fem-input-profile";

export type WorkflowFemMaterialPreset = {
  key: string;
  label: string;
  summary: string;
  values: Record<string, number>;
};

const MATERIAL_PRESETS: Record<string, WorkflowFemMaterialPreset[]> = {
  "study_model/electrostatic_plane_quad_2d": [
    {
      key: "air",
      label: "Air",
      summary: "Baseline dielectric region for open electrostatic space.",
      values: { permittivity: 1, thickness: 1 },
    },
    {
      key: "fr4",
      label: "FR4",
      summary: "Common PCB dielectric substrate.",
      values: { permittivity: 4.2, thickness: 1.6e-3 },
    },
  ],
  "study_model/electrostatic_plane_triangle_2d": [
    {
      key: "air",
      label: "Air",
      summary: "Baseline dielectric region for open electrostatic space.",
      values: { permittivity: 1, thickness: 1 },
    },
    {
      key: "fr4",
      label: "FR4",
      summary: "Common PCB dielectric substrate.",
      values: { permittivity: 4.2, thickness: 1.6e-3 },
    },
  ],
  "study_model/heat_plane_quad_2d": [
    {
      key: "aluminum",
      label: "Aluminum",
      summary: "High-conductivity lightweight thermal plate.",
      values: { conductivity: 205, thickness: 1e-3 },
    },
    {
      key: "concrete",
      label: "Concrete",
      summary: "Low-conductivity structural thermal mass.",
      values: { conductivity: 1.7, thickness: 0.1 },
    },
  ],
  "study_model/heat_plane_triangle_2d": [
    {
      key: "aluminum",
      label: "Aluminum",
      summary: "High-conductivity lightweight thermal plate.",
      values: { conductivity: 205, thickness: 1e-3 },
    },
    {
      key: "concrete",
      label: "Concrete",
      summary: "Low-conductivity structural thermal mass.",
      values: { conductivity: 1.7, thickness: 0.1 },
    },
  ],
  "study_model/plane_quad_2d": [
    {
      key: "steel",
      label: "Steel",
      summary: "General-purpose isotropic structural solid.",
      values: { youngs_modulus: 2.1e11, poisson_ratio: 0.3, thickness: 1e-2 },
    },
    {
      key: "aluminum",
      label: "Aluminum",
      summary: "Lower stiffness and lighter structural plate.",
      values: { youngs_modulus: 7e10, poisson_ratio: 0.33, thickness: 8e-3 },
    },
  ],
  "study_model/plane_triangle_2d": [
    {
      key: "steel",
      label: "Steel",
      summary: "General-purpose isotropic structural solid.",
      values: { youngs_modulus: 2.1e11, poisson_ratio: 0.3, thickness: 1e-2 },
    },
    {
      key: "aluminum",
      label: "Aluminum",
      summary: "Lower stiffness and lighter structural plate.",
      values: { youngs_modulus: 7e10, poisson_ratio: 0.33, thickness: 8e-3 },
    },
  ],
  "study_model/thermal_plane_quad_2d": [
    {
      key: "steel",
      label: "Steel",
      summary: "Structural plate with moderate thermal expansion.",
      values: { youngs_modulus: 2.1e11, poisson_ratio: 0.3, thermal_expansion: 1.2e-5, thickness: 1e-2 },
    },
    {
      key: "aluminum",
      label: "Aluminum",
      summary: "Lighter plate with stronger thermal response.",
      values: { youngs_modulus: 7e10, poisson_ratio: 0.33, thermal_expansion: 2.3e-5, thickness: 8e-3 },
    },
  ],
  "study_model/thermal_plane_triangle_2d": [
    {
      key: "steel",
      label: "Steel",
      summary: "Structural plate with moderate thermal expansion.",
      values: { youngs_modulus: 2.1e11, poisson_ratio: 0.3, thermal_expansion: 1.2e-5, thickness: 1e-2 },
    },
    {
      key: "aluminum",
      label: "Aluminum",
      summary: "Lighter plate with stronger thermal response.",
      values: { youngs_modulus: 7e10, poisson_ratio: 0.33, thermal_expansion: 2.3e-5, thickness: 8e-3 },
    },
  ],
  "study_model/bar_1d": [
    {
      key: "steel",
      label: "Steel",
      summary: "Default bar stiffness baseline.",
      values: { youngs_modulus: 2.1e11, area: 1e-4 },
    },
    {
      key: "aluminum",
      label: "Aluminum",
      summary: "Lower-stiffness lightweight bar.",
      values: { youngs_modulus: 7e10, area: 1e-4 },
    },
  ],
  "study_model/heat_bar_1d": [
    {
      key: "aluminum",
      label: "Aluminum",
      summary: "Fast-conducting thermal bar.",
      values: { conductivity: 205, area: 1e-4 },
    },
    {
      key: "ceramic",
      label: "Ceramic",
      summary: "Lower-conductivity heat path.",
      values: { conductivity: 25, area: 1e-4 },
    },
  ],
  "study_model/thermal_bar_1d": [
    {
      key: "steel",
      label: "Steel",
      summary: "Thermal-structural bar baseline.",
      values: { youngs_modulus: 2.1e11, area: 1e-4, thermal_expansion: 1.2e-5 },
    },
    {
      key: "aluminum",
      label: "Aluminum",
      summary: "Higher thermal expansion lightweight bar.",
      values: { youngs_modulus: 7e10, area: 1e-4, thermal_expansion: 2.3e-5 },
    },
  ],
};

export function resolveWorkflowFemMaterialPresets(artifactType: string) {
  return MATERIAL_PRESETS[artifactType] ?? [];
}

export function applyWorkflowFemMaterialPreset(
  artifactType: string,
  payload: unknown,
  presetKey: string,
) {
  const profile = resolveWorkflowFemInputProfile(artifactType);
  const preset = resolveWorkflowFemMaterialPresets(artifactType).find((entry) => entry.key === presetKey);
  const materialSection = profile?.sections.find((section) => section.key === "material");
  if (!profile || !preset || !materialSection || typeof payload !== "object" || payload === null) return payload;

  const next = JSON.parse(JSON.stringify(payload)) as Record<string, unknown>;
  if (materialSection.target === "root") {
    for (const [field, value] of Object.entries(preset.values)) next[field] = value;
    return next;
  }

  const collection = next[materialSection.target];
  if (!Array.isArray(collection) || collection.length === 0) return next;
  next[materialSection.target] = collection.map((entry) => {
    if (typeof entry !== "object" || entry === null) return entry;
    return { ...(entry as Record<string, unknown>), ...preset.values };
  });
  return next;
}
