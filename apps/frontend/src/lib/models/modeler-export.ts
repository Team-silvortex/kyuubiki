import type {
  AxialBarJobInput,
  Beam1dJobInput,
  Frame2dJobInput,
  HeatBar1dJobInput,
  HeatPlaneQuad2dJobInput,
  HeatPlaneTriangle2dJobInput,
  JobResultRecord,
  JobState,
  ModelMaterial,
  ModelRecord,
  ModelVersionRecord,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ResultRecord,
  Spring1dJobInput,
  Spring2dJobInput,
  Spring3dJobInput,
  ThermalBar1dJobInput,
  ThermalBeam1dJobInput,
  ThermalFrame2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
  ThermalTruss2dJobInput,
  ThermalTruss3dJobInput,
  Torsion1dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
  ProjectRecord,
} from "@/lib/api";
import type { WorkbenchMacroPresetRecord } from "@/lib/scripting/workbench-script-runtime";
import {
  PROJECT_SCHEMA_VERSION,
  defaultProjectFileManifest,
  type ProjectAssetMetaRecord,
  type ProjectAssetReferenceRecord,
} from "@/lib/projects";
import { buildAnalysisMetadata } from "@/lib/models/modeler-analysis";
import { MODEL_SCHEMA_VERSION, type StudyKind } from "@/lib/models/modeler-types";

type ExportStudyPayload = {
  name: string;
  material: string;
  youngsModulusGpa: number;
  materials?: ModelMaterial[];
  axial?: AxialBarJobInput;
  truss?: Truss2dJobInput;
  truss3d?: Truss3dJobInput;
  thermalTruss?: ThermalTruss2dJobInput;
  thermalTruss3d?: ThermalTruss3dJobInput;
  plane?:
    | PlaneTriangle2dJobInput
    | PlaneQuad2dJobInput
    | ThermalPlaneTriangle2dJobInput
    | ThermalPlaneQuad2dJobInput
    | HeatPlaneTriangle2dJobInput
    | HeatPlaneQuad2dJobInput;
  frame?: Frame2dJobInput;
  thermalFrame?: ThermalFrame2dJobInput;
  beam?: Beam1dJobInput;
  thermalBeam?: ThermalBeam1dJobInput;
  torsion?: Torsion1dJobInput;
  heatBar?: HeatBar1dJobInput;
  thermalBar?: ThermalBar1dJobInput;
  spring?: Spring1dJobInput;
  spring2d?: Spring2dJobInput;
  spring3d?: Spring3dJobInput;
};

export function exportStudyModel(kind: StudyKind, payload: ExportStudyPayload): string {
  if (kind === "spring_1d" && payload.spring) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), nodes: payload.spring.nodes, elements: payload.spring.elements }, null, 2);
  }
  if (kind === "spring_2d" && payload.spring2d) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), nodes: payload.spring2d.nodes, elements: payload.spring2d.elements }, null, 2);
  }
  if (kind === "spring_3d" && payload.spring3d) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), nodes: payload.spring3d.nodes, elements: payload.spring3d.elements }, null, 2);
  }
  if (kind === "heat_bar_1d" && payload.heatBar) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), nodes: payload.heatBar.nodes, elements: payload.heatBar.elements }, null, 2);
  }
  if ((kind === "heat_plane_triangle_2d" || kind === "heat_plane_quad_2d" || kind === "plane_triangle_2d" || kind === "plane_quad_2d" || kind === "thermal_plane_triangle_2d" || kind === "thermal_plane_quad_2d") && payload.plane) {
    return JSON.stringify({
      kind,
      model_schema_version: MODEL_SCHEMA_VERSION,
      name: payload.name,
      analysis_metadata: buildAnalysisMetadata(kind, payload),
      ...((kind === "heat_plane_triangle_2d" || kind === "heat_plane_quad_2d")
        ? {}
        : { material: payload.material, youngs_modulus_gpa: payload.youngsModulusGpa, materials: payload.plane.materials ?? payload.materials }),
      nodes: payload.plane.nodes,
      elements: payload.plane.elements,
    }, null, 2);
  }
  if (kind === "truss_2d" && payload.truss) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), material: payload.material, youngs_modulus_gpa: payload.youngsModulusGpa, materials: payload.truss.materials ?? payload.materials, nodes: payload.truss.nodes, elements: payload.truss.elements }, null, 2);
  }
  if (kind === "thermal_truss_2d" && payload.thermalTruss) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), material: payload.material, youngs_modulus_gpa: payload.youngsModulusGpa, materials: payload.thermalTruss.materials ?? payload.materials, nodes: payload.thermalTruss.nodes, elements: payload.thermalTruss.elements }, null, 2);
  }
  if (kind === "truss_3d" && payload.truss3d) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), material: payload.material, youngs_modulus_gpa: payload.youngsModulusGpa, materials: payload.truss3d.materials ?? payload.materials, nodes: payload.truss3d.nodes, elements: payload.truss3d.elements }, null, 2);
  }
  if (kind === "thermal_truss_3d" && payload.thermalTruss3d) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), material: payload.material, youngs_modulus_gpa: payload.youngsModulusGpa, materials: payload.thermalTruss3d.materials ?? payload.materials, nodes: payload.thermalTruss3d.nodes, elements: payload.thermalTruss3d.elements }, null, 2);
  }
  if (kind === "frame_2d" && payload.frame) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), material: payload.material, youngs_modulus_gpa: payload.youngsModulusGpa, materials: payload.frame.materials ?? payload.materials, nodes: payload.frame.nodes, elements: payload.frame.elements }, null, 2);
  }
  if (kind === "thermal_frame_2d" && payload.thermalFrame) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), material: payload.material, youngs_modulus_gpa: payload.youngsModulusGpa, materials: payload.thermalFrame.materials ?? payload.materials, nodes: payload.thermalFrame.nodes, elements: payload.thermalFrame.elements }, null, 2);
  }
  if (kind === "beam_1d" && payload.beam) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, material: payload.material, youngs_modulus_gpa: payload.youngsModulusGpa, materials: payload.beam.materials ?? payload.materials, nodes: payload.beam.nodes, elements: payload.beam.elements }, null, 2);
  }
  if (kind === "thermal_beam_1d" && payload.thermalBeam) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, material: payload.material, youngs_modulus_gpa: payload.youngsModulusGpa, materials: payload.thermalBeam.materials ?? payload.materials, nodes: payload.thermalBeam.nodes, elements: payload.thermalBeam.elements }, null, 2);
  }
  if (kind === "torsion_1d" && payload.torsion) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), nodes: payload.torsion.nodes, elements: payload.torsion.elements }, null, 2);
  }
  if (kind === "thermal_bar_1d" && payload.thermalBar) {
    return JSON.stringify({ kind, model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata(kind, payload), nodes: payload.thermalBar.nodes, elements: payload.thermalBar.elements }, null, 2);
  }
  if (payload.axial) {
    return JSON.stringify({ kind: "axial_bar_1d", model_schema_version: MODEL_SCHEMA_VERSION, name: payload.name, analysis_metadata: buildAnalysisMetadata("axial_bar_1d", payload), material: payload.material, length: payload.axial.length, area: payload.axial.area, elements: payload.axial.elements, tip_force: payload.axial.tip_force, youngs_modulus_gpa: payload.youngsModulusGpa }, null, 2);
  }
  return JSON.stringify({}, null, 2);
}

export function buildStudyModelPayload(kind: StudyKind, payload: ExportStudyPayload): Record<string, unknown> {
  return JSON.parse(exportStudyModel(kind, payload)) as Record<string, unknown>;
}

export function exportProjectBundle(payload: {
  project: ProjectRecord;
  models: ModelRecord[];
  modelVersions: ModelVersionRecord[];
  activeModelId?: string | null;
  activeVersionId?: string | null;
  workspaceSnapshot?: Record<string, unknown> | null;
  automationPresets?: WorkbenchMacroPresetRecord[];
  assetCatalog?: ProjectAssetMetaRecord[];
  assetReferences?: ProjectAssetReferenceRecord[];
  jobs?: JobState[];
  results?: JobResultRecord[];
}): string {
  return JSON.stringify(
    {
      project_schema_version: PROJECT_SCHEMA_VERSION,
      exported_at: new Date().toISOString(),
      project_file_manifest: defaultProjectFileManifest(),
      project: payload.project,
      models: payload.models,
      model_versions: payload.modelVersions,
      active_model_id: payload.activeModelId ?? null,
      active_version_id: payload.activeVersionId ?? null,
      workspace_snapshot: payload.workspaceSnapshot ?? null,
      automation_presets: payload.automationPresets ?? [],
      asset_catalog: payload.assetCatalog ?? [],
      asset_references: payload.assetReferences ?? [],
      jobs: payload.jobs ?? [],
      results: payload.results ?? [],
    },
    null,
    2,
  );
}
