"use client";

import type { PlaneResultField } from "./workbench-defaults";
import type { ViewportRenderStrategy } from "./workbench-render-diagnostics";
import type { BeamResultField, FrameResultField, StudyKind } from "./workbench-types";

type PlaneFamily = "structural_plane" | "heat_plane" | "thermal_plane";
type LineFamily = "frame" | "thermal_frame" | "beam" | "thermal_beam" | "torsion";

function planeFamilyForStudyKind(studyKind: StudyKind): PlaneFamily | null {
  if (studyKind === "heat_plane_triangle_2d" || studyKind === "heat_plane_quad_2d") return "heat_plane";
  if (studyKind === "thermal_plane_triangle_2d" || studyKind === "thermal_plane_quad_2d") return "thermal_plane";
  if (studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d") return "structural_plane";
  return null;
}

function lineFamilyForStudyKind(studyKind: StudyKind): LineFamily | null {
  if (studyKind === "frame_2d") return "frame";
  if (studyKind === "thermal_frame_2d") return "thermal_frame";
  if (studyKind === "beam_1d") return "beam";
  if (studyKind === "thermal_beam_1d") return "thermal_beam";
  if (studyKind === "torsion_1d") return "torsion";
  return null;
}

export function prioritizedPlaneFields(studyKind: StudyKind, strategy: ViewportRenderStrategy): PlaneResultField[] {
  const family = planeFamilyForStudyKind(studyKind);
  if (!family) return [];

  if (family === "heat_plane") {
    if (strategy === "focus") return ["average_temperature", "heat_flux_magnitude"];
    if (strategy === "progressive") {
      return ["average_temperature", "heat_flux_magnitude", "temperature_gradient_y", "temperature_gradient_x"];
    }
    if (strategy === "full") {
      return [
        "average_temperature",
        "heat_flux_magnitude",
        "temperature_gradient_x",
        "temperature_gradient_y",
        "heat_flux_x",
        "heat_flux_y",
      ];
    }
    return [
      "average_temperature",
      "heat_flux_magnitude",
      "temperature_gradient_x",
      "temperature_gradient_y",
      "heat_flux_x",
      "heat_flux_y",
    ];
  }

  if (family === "thermal_plane") {
    if (strategy === "focus") return ["average_temperature_delta", "von_mises"];
    if (strategy === "progressive") {
      return ["average_temperature_delta", "von_mises", "thermal_strain", "principal_stress_1"];
    }
    return [
      "average_temperature_delta",
      "von_mises",
      "principal_stress_1",
      "max_in_plane_shear",
      "thermal_strain",
      "mechanical_strain",
    ];
  }

  if (strategy === "focus") return ["von_mises"];
  if (strategy === "progressive") return ["von_mises", "principal_stress_1"];
  return ["von_mises", "principal_stress_1", "max_in_plane_shear"];
}

export function prioritizedFrameFields(studyKind: StudyKind, strategy: ViewportRenderStrategy): FrameResultField[] {
  const family = lineFamilyForStudyKind(studyKind);
  if (!family || family === "beam") return [];

  if (family === "torsion") {
    return strategy === "focus" ? ["max_bending_stress"] : ["max_bending_stress", "moment"];
  }

  if (family === "thermal_frame") {
    if (strategy === "focus") return ["max_combined_stress", "average_temperature_delta"];
    if (strategy === "progressive") {
      return ["max_combined_stress", "average_temperature_delta", "axial_stress", "moment"];
    }
    return [
      "max_combined_stress",
      "average_temperature_delta",
      "axial_stress",
      "max_bending_stress",
      "moment",
      "temperature_gradient_y",
      "thermal_curvature",
    ];
  }

  if (strategy === "focus") return ["max_combined_stress", "axial_stress"];
  if (strategy === "progressive") return ["max_combined_stress", "axial_stress", "moment"];
  return ["max_combined_stress", "axial_stress", "max_bending_stress", "moment"];
}

export function prioritizedBeamFields(studyKind: StudyKind, strategy: ViewportRenderStrategy): BeamResultField[] {
  const family = lineFamilyForStudyKind(studyKind);
  if (!family || (family !== "beam" && family !== "thermal_beam")) return [];

  if (family === "thermal_beam") {
    if (strategy === "focus") return ["max_bending_stress"];
    if (strategy === "progressive") return ["max_bending_stress", "moment", "temperature_gradient_y"];
    return ["max_bending_stress", "shear_force", "moment", "temperature_gradient_y", "thermal_curvature"];
  }

  if (strategy === "focus") return ["max_bending_stress"];
  if (strategy === "progressive") return ["max_bending_stress", "moment"];
  return ["max_bending_stress", "shear_force", "moment"];
}
