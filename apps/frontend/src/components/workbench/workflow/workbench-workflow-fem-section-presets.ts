"use client";

import type { WorkflowFemInputSection } from "@/components/workbench/workflow/workbench-workflow-fem-input-profile";
import { resolveWorkflowFemInputProfile } from "@/components/workbench/workflow/workbench-workflow-fem-input-profile";

export type WorkflowFemSectionPreset = {
  key: string;
  label: string;
  summary: string;
  values: Record<string, boolean | number>;
};

const SECTION_PRESETS: Record<
  string,
  Partial<Record<WorkflowFemInputSection["key"], WorkflowFemSectionPreset[]>>
> = {
  "study_model/electrostatic_plane_quad_2d": {
    boundary: [
      { key: "ground", label: "Ground", summary: "Pin selected nodes to zero electric potential.", values: { fix_potential: true, potential: 0 } },
      { key: "bias_1v", label: "Bias 1V", summary: "Apply a unit-potential electrode boundary.", values: { fix_potential: true, potential: 1 } },
    ],
    load: [
      { key: "neutral", label: "Neutral", summary: "Zero volumetric charge density.", values: { charge_density: 0 } },
      { key: "charged", label: "Charged", summary: "Light positive charge density seed.", values: { charge_density: 1e-6 } },
    ],
  },
  "study_model/electrostatic_plane_triangle_2d": {
    boundary: [
      { key: "ground", label: "Ground", summary: "Pin selected nodes to zero electric potential.", values: { fix_potential: true, potential: 0 } },
      { key: "bias_1v", label: "Bias 1V", summary: "Apply a unit-potential electrode boundary.", values: { fix_potential: true, potential: 1 } },
    ],
    load: [
      { key: "neutral", label: "Neutral", summary: "Zero volumetric charge density.", values: { charge_density: 0 } },
      { key: "charged", label: "Charged", summary: "Light positive charge density seed.", values: { charge_density: 1e-6 } },
    ],
  },
  "study_model/heat_plane_quad_2d": {
    boundary: [
      { key: "ambient", label: "Ambient", summary: "Clamp nodes to ambient temperature.", values: { fix_temperature: true, temperature: 293.15 } },
      { key: "hot_plate", label: "Hot Plate", summary: "Clamp nodes to a hot boundary.", values: { fix_temperature: true, temperature: 373.15 } },
    ],
    load: [
      { key: "idle", label: "Idle", summary: "No internal heat generation.", values: { heat_load: 0 } },
      { key: "heater", label: "Heater", summary: "Seed a moderate internal heat source.", values: { heat_load: 1000 } },
    ],
  },
  "study_model/heat_plane_triangle_2d": {
    boundary: [
      { key: "ambient", label: "Ambient", summary: "Clamp nodes to ambient temperature.", values: { fix_temperature: true, temperature: 293.15 } },
      { key: "hot_plate", label: "Hot Plate", summary: "Clamp nodes to a hot boundary.", values: { fix_temperature: true, temperature: 373.15 } },
    ],
    load: [
      { key: "idle", label: "Idle", summary: "No internal heat generation.", values: { heat_load: 0 } },
      { key: "heater", label: "Heater", summary: "Seed a moderate internal heat source.", values: { heat_load: 1000 } },
    ],
  },
  "study_model/plane_quad_2d": {
    boundary: [
      { key: "clamped", label: "Clamped", summary: "Fully restrain planar displacement.", values: { fix_x: true, fix_y: true } },
      { key: "roller_x", label: "Roller X", summary: "Restrict x only, allow tangential sliding.", values: { fix_x: true, fix_y: false } },
    ],
    load: [
      { key: "idle", label: "Idle", summary: "No external nodal load.", values: { load_x: 0, load_y: 0 } },
      { key: "downward", label: "Downward", summary: "Apply a unit downward nodal force.", values: { load_x: 0, load_y: -1000 } },
    ],
  },
  "study_model/plane_triangle_2d": {
    boundary: [
      { key: "clamped", label: "Clamped", summary: "Fully restrain planar displacement.", values: { fix_x: true, fix_y: true } },
      { key: "roller_x", label: "Roller X", summary: "Restrict x only, allow tangential sliding.", values: { fix_x: true, fix_y: false } },
    ],
    load: [
      { key: "idle", label: "Idle", summary: "No external nodal load.", values: { load_x: 0, load_y: 0 } },
      { key: "downward", label: "Downward", summary: "Apply a unit downward nodal force.", values: { load_x: 0, load_y: -1000 } },
    ],
  },
  "study_model/thermal_plane_quad_2d": {
    boundary: [
      { key: "clamped", label: "Clamped", summary: "Fully restrain planar displacement.", values: { fix_x: true, fix_y: true } },
      { key: "roller_x", label: "Roller X", summary: "Restrict x only, allow tangential sliding.", values: { fix_x: true, fix_y: false } },
    ],
    load: [
      { key: "thermal_ramp", label: "Thermal Ramp", summary: "Seed uniform temperature growth.", values: { load_x: 0, load_y: 0, temperature_delta: 60 } },
      { key: "downward_hot", label: "Downward Hot", summary: "Combine mechanical and thermal load.", values: { load_x: 0, load_y: -1000, temperature_delta: 40 } },
    ],
  },
  "study_model/thermal_plane_triangle_2d": {
    boundary: [
      { key: "clamped", label: "Clamped", summary: "Fully restrain planar displacement.", values: { fix_x: true, fix_y: true } },
      { key: "roller_x", label: "Roller X", summary: "Restrict x only, allow tangential sliding.", values: { fix_x: true, fix_y: false } },
    ],
    load: [
      { key: "thermal_ramp", label: "Thermal Ramp", summary: "Seed uniform temperature growth.", values: { load_x: 0, load_y: 0, temperature_delta: 60 } },
      { key: "downward_hot", label: "Downward Hot", summary: "Combine mechanical and thermal load.", values: { load_x: 0, load_y: -1000, temperature_delta: 40 } },
    ],
  },
  "study_model/bar_1d": {
    boundary: [
      { key: "short_bar", label: "Short Bar", summary: "Set a compact axial span.", values: { length: 1 } },
      { key: "long_bar", label: "Long Bar", summary: "Set a longer axial span.", values: { length: 10 } },
    ],
    load: [
      { key: "idle", label: "Idle", summary: "No axial force at the tip.", values: { tip_force: 0 } },
      { key: "tension", label: "Tension", summary: "Apply tensile tip force.", values: { tip_force: 1000 } },
    ],
  },
  "study_model/heat_bar_1d": {
    boundary: [
      { key: "ambient", label: "Ambient", summary: "Clamp nodal temperature to ambient.", values: { fix_temperature: true, temperature: 293.15 } },
      { key: "hot_end", label: "Hot End", summary: "Clamp nodal temperature to a hot state.", values: { fix_temperature: true, temperature: 373.15 } },
    ],
    load: [
      { key: "idle", label: "Idle", summary: "No distributed heating.", values: { heat_load: 0 } },
      { key: "heater", label: "Heater", summary: "Apply moderate distributed heating.", values: { heat_load: 500 } },
    ],
  },
  "study_model/thermal_bar_1d": {
    boundary: [
      { key: "fixed", label: "Fixed", summary: "Axially restrain the selected nodes.", values: { fix_x: true } },
      { key: "free", label: "Free", summary: "Release axial restraint at the selected nodes.", values: { fix_x: false } },
    ],
    load: [
      { key: "thermal_ramp", label: "Thermal Ramp", summary: "Apply pure thermal expansion loading.", values: { load_x: 0, temperature_delta: 60 } },
      { key: "tension_hot", label: "Tension Hot", summary: "Combine axial tension with heating.", values: { load_x: 1000, temperature_delta: 40 } },
    ],
  },
};

export function resolveWorkflowFemSectionPresets(
  artifactType: string,
  sectionKey: WorkflowFemInputSection["key"],
) {
  return SECTION_PRESETS[artifactType]?.[sectionKey] ?? [];
}

export function applyWorkflowFemSectionPreset(
  artifactType: string,
  payload: unknown,
  sectionKey: WorkflowFemInputSection["key"],
  presetKey: string,
) {
  const profile = resolveWorkflowFemInputProfile(artifactType);
  const section = profile?.sections.find((entry) => entry.key === sectionKey);
  const preset = resolveWorkflowFemSectionPresets(artifactType, sectionKey).find((entry) => entry.key === presetKey);
  if (!profile || !section || !preset || typeof payload !== "object" || payload === null) return payload;

  const next = JSON.parse(JSON.stringify(payload)) as Record<string, unknown>;
  if (section.target === "root") {
    for (const [field, value] of Object.entries(preset.values)) next[field] = value;
    return next;
  }

  const collection = next[section.target];
  if (!Array.isArray(collection) || collection.length === 0) return next;
  next[section.target] = collection.map((entry) => {
    if (typeof entry !== "object" || entry === null) return entry;
    return { ...(entry as Record<string, unknown>), ...preset.values };
  });
  return next;
}
