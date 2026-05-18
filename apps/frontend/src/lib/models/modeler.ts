import type {
  AxialBarJobInput,
  Beam1dJobInput,
  Frame2dJobInput,
  HeatBar1dJobInput,
  JobResultRecord,
  JobState,
  ModelMaterial,
  ModelRecord,
  ModelVersionRecord,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
  ProjectRecord,
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
  TrussElementInput,
  TrussNodeInput,
} from "@/lib/api";
import type { WorkbenchMacroPresetRecord } from "@/lib/scripting/workbench-script-runtime";
import {
  PROJECT_SCHEMA_VERSION,
  defaultProjectFileManifest,
  type ProjectAssetMetaRecord,
  type ProjectAssetReferenceRecord,
} from "@/lib/projects";

export type ParametricTrussConfig = {
  bays: number;
  span: number;
  height: number;
  area: number;
  youngsModulusGpa: number;
  loadY: number;
};

export type ParametricPanelConfig = {
  width: number;
  height: number;
  divisionsX: number;
  divisionsY: number;
  thickness: number;
  youngsModulusGpa: number;
  poissonRatio: number;
  loadY: number;
};

export type ParametricPanelElementKind = "triangle" | "quad";

export const MODEL_SCHEMA_VERSION = "kyuubiki.model/v1";
export function generateRectangularPanelMesh(config: ParametricPanelConfig): PlaneTriangle2dJobInput {
  const width = Math.max(0.2, config.width);
  const height = Math.max(0.2, config.height);
  const divisionsX = Math.max(1, Math.round(config.divisionsX));
  const divisionsY = Math.max(1, Math.round(config.divisionsY));
  const thickness = Math.max(0.001, config.thickness);
  const modulus = Math.max(0.1, config.youngsModulusGpa) * 1.0e9;
  const poissonRatio = Math.min(0.49, Math.max(0.01, config.poissonRatio));
  const loadY = config.loadY;

  const nodes: PlaneTriangle2dJobInput["nodes"] = [];
  const elements: PlaneTriangle2dJobInput["elements"] = [];

  const dx = width / divisionsX;
  const dy = height / divisionsY;

  for (let row = 0; row <= divisionsY; row += 1) {
    for (let col = 0; col <= divisionsX; col += 1) {
      const index = row * (divisionsX + 1) + col;
      const onLeftEdge = col === 0;
      const onRightEdge = col === divisionsX;

      nodes.push({
        id: `p${index}`,
        x: round(col * dx),
        y: round(row * dy),
        fix_x: onLeftEdge,
        fix_y: onLeftEdge,
        load_x: 0,
        load_y: onRightEdge ? loadY / (divisionsY + 1) : 0,
      });
    }
  }

  for (let row = 0; row < divisionsY; row += 1) {
    for (let col = 0; col < divisionsX; col += 1) {
      const n0 = row * (divisionsX + 1) + col;
      const n1 = n0 + 1;
      const n2 = n0 + divisionsX + 1;
      const n3 = n2 + 1;
      const base = elements.length;

      elements.push({
        id: `pt${base}`,
        node_i: n0,
        node_j: n1,
        node_k: n3,
        thickness,
        youngs_modulus: modulus,
        poisson_ratio: poissonRatio,
      });

      elements.push({
        id: `pt${base + 1}`,
        node_i: n0,
        node_j: n3,
        node_k: n2,
        thickness,
        youngs_modulus: modulus,
        poisson_ratio: poissonRatio,
      });
    }
  }

  return { nodes, elements };
}

export function generateRectangularQuadPanelMesh(config: ParametricPanelConfig): PlaneQuad2dJobInput {
  const width = Math.max(0.2, config.width);
  const height = Math.max(0.2, config.height);
  const divisionsX = Math.max(1, Math.round(config.divisionsX));
  const divisionsY = Math.max(1, Math.round(config.divisionsY));
  const thickness = Math.max(0.001, config.thickness);
  const modulus = Math.max(0.1, config.youngsModulusGpa) * 1.0e9;
  const poissonRatio = Math.min(0.49, Math.max(0.01, config.poissonRatio));
  const loadY = config.loadY;

  const nodes: PlaneQuad2dJobInput["nodes"] = [];
  const elements: PlaneQuad2dJobInput["elements"] = [];

  const dx = width / divisionsX;
  const dy = height / divisionsY;

  for (let row = 0; row <= divisionsY; row += 1) {
    for (let col = 0; col <= divisionsX; col += 1) {
      const index = row * (divisionsX + 1) + col;
      const onLeftEdge = col === 0;
      const onRightEdge = col === divisionsX;

      nodes.push({
        id: `q${index}`,
        x: round(col * dx),
        y: round(row * dy),
        fix_x: onLeftEdge,
        fix_y: onLeftEdge,
        load_x: 0,
        load_y: onRightEdge ? loadY / (divisionsY + 1) : 0,
      });
    }
  }

  for (let row = 0; row < divisionsY; row += 1) {
    for (let col = 0; col < divisionsX; col += 1) {
      const n0 = row * (divisionsX + 1) + col;
      const n1 = n0 + 1;
      const n2 = n0 + divisionsX + 2;
      const n3 = n0 + divisionsX + 1;
      const base = elements.length;

      elements.push({
        id: `pq${base}`,
        node_i: n0,
        node_j: n1,
        node_k: n2,
        node_l: n3,
        thickness,
        youngs_modulus: modulus,
        poisson_ratio: poissonRatio,
      });
    }
  }

  return { nodes, elements };
}

export function generatePrattTruss(config: ParametricTrussConfig): Truss2dJobInput {
  const bays = Math.max(2, Math.round(config.bays));
  const span = Math.max(1, config.span);
  const height = Math.max(0.2, config.height);
  const bayWidth = span / bays;
  const modulus = Math.max(0.1, config.youngsModulusGpa) * 1.0e9;
  const area = Math.max(1.0e-5, config.area);
  const loadY = config.loadY;

  const nodes: TrussNodeInput[] = [];
  const elements: TrussElementInput[] = [];

  for (let index = 0; index <= bays; index += 1) {
    nodes.push({
      id: `b${index}`,
      x: round(index * bayWidth),
      y: 0,
      fix_x: index === 0,
      fix_y: index === 0 || index === bays,
      load_x: 0,
      load_y: 0,
    });
  }

  for (let index = 0; index < bays; index += 1) {
    nodes.push({
      id: `t${index}`,
      x: round(index * bayWidth + bayWidth / 2),
      y: round(height),
      fix_x: false,
      fix_y: false,
      load_x: 0,
      load_y: index === Math.floor((bays - 1) / 2) ? loadY : 0,
    });
  }

  for (let index = 0; index < bays; index += 1) {
    elements.push(member(`bb${index}`, index, index + 1, area, modulus));
  }

  for (let index = 0; index < bays - 1; index += 1) {
    elements.push(member(`tt${index}`, bays + 1 + index, bays + 2 + index, area, modulus));
  }

  for (let index = 0; index < bays; index += 1) {
    const top = bays + 1 + index;
    elements.push(member(`v${index}`, index + 1, top, area, modulus));

    if (index % 2 === 0) {
      elements.push(member(`d${index}`, index, top, area, modulus));
    } else {
      elements.push(member(`d${index}`, index + 1, top, area, modulus));
    }
  }

  return { nodes, elements };
}

export function exportStudyModel(
  kind: "axial_bar_1d" | "heat_bar_1d" | "thermal_bar_1d" | "thermal_beam_1d" | "thermal_frame_2d" | "thermal_truss_2d" | "thermal_truss_3d" | "thermal_plane_triangle_2d" | "thermal_plane_quad_2d" | "spring_1d" | "spring_2d" | "spring_3d" | "beam_1d" | "torsion_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d" | "plane_quad_2d" | "frame_2d",
  payload: {
    name: string;
    material: string;
    youngsModulusGpa: number;
    materials?: ModelMaterial[];
    axial?: AxialBarJobInput;
    truss?: Truss2dJobInput;
    truss3d?: Truss3dJobInput;
    thermalTruss?: ThermalTruss2dJobInput;
    thermalTruss3d?: ThermalTruss3dJobInput;
    plane?: PlaneTriangle2dJobInput | PlaneQuad2dJobInput | ThermalPlaneTriangle2dJobInput | ThermalPlaneQuad2dJobInput;
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
  },
): string {
  if (kind === "spring_1d" && payload.spring) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        nodes: payload.spring.nodes,
        elements: payload.spring.elements,
      },
      null,
      2,
    );
  }

  if (kind === "spring_2d" && payload.spring2d) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        nodes: payload.spring2d.nodes,
        elements: payload.spring2d.elements,
      },
      null,
      2,
    );
  }

  if (kind === "spring_3d" && payload.spring3d) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        nodes: payload.spring3d.nodes,
        elements: payload.spring3d.elements,
      },
      null,
      2,
    );
  }

  if (kind === "heat_bar_1d" && payload.heatBar) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        nodes: payload.heatBar.nodes,
        elements: payload.heatBar.elements,
      },
      null,
      2,
    );
  }

  if (
    (kind === "plane_triangle_2d" ||
      kind === "plane_quad_2d" ||
      kind === "thermal_plane_triangle_2d" ||
      kind === "thermal_plane_quad_2d") &&
    payload.plane
  ) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        materials: payload.plane.materials ?? payload.materials,
        nodes: payload.plane.nodes,
        elements: payload.plane.elements,
      },
      null,
      2,
    );
  }

  if (kind === "truss_2d" && payload.truss) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        materials: payload.truss.materials ?? payload.materials,
        nodes: payload.truss.nodes,
        elements: payload.truss.elements,
      },
      null,
      2,
    );
  }

  if (kind === "thermal_truss_2d" && payload.thermalTruss) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        materials: payload.thermalTruss.materials ?? payload.materials,
        nodes: payload.thermalTruss.nodes,
        elements: payload.thermalTruss.elements,
      },
      null,
      2,
    );
  }

  if (kind === "truss_3d" && payload.truss3d) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        materials: payload.truss3d.materials ?? payload.materials,
        nodes: payload.truss3d.nodes,
        elements: payload.truss3d.elements,
      },
      null,
      2,
    );
  }

  if (kind === "thermal_truss_3d" && payload.thermalTruss3d) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        materials: payload.thermalTruss3d.materials ?? payload.materials,
        nodes: payload.thermalTruss3d.nodes,
        elements: payload.thermalTruss3d.elements,
      },
      null,
      2,
    );
  }

  if (kind === "frame_2d" && payload.frame) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        materials: payload.frame.materials ?? payload.materials,
        nodes: payload.frame.nodes,
        elements: payload.frame.elements,
      },
      null,
      2,
    );
  }

  if (kind === "thermal_frame_2d" && payload.thermalFrame) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        materials: payload.thermalFrame.materials ?? payload.materials,
        nodes: payload.thermalFrame.nodes,
        elements: payload.thermalFrame.elements,
      },
      null,
      2,
    );
  }

  if (kind === "beam_1d" && payload.beam) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        materials: payload.beam.materials ?? payload.materials,
        nodes: payload.beam.nodes,
        elements: payload.beam.elements,
      },
      null,
      2,
    );
  }

  if (kind === "thermal_beam_1d" && payload.thermalBeam) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        materials: payload.thermalBeam.materials ?? payload.materials,
        nodes: payload.thermalBeam.nodes,
        elements: payload.thermalBeam.elements,
      },
      null,
      2,
    );
  }

  if (kind === "torsion_1d" && payload.torsion) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        nodes: payload.torsion.nodes,
        elements: payload.torsion.elements,
      },
      null,
      2,
    );
  }

  if (kind === "thermal_bar_1d" && payload.thermalBar) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        nodes: payload.thermalBar.nodes,
        elements: payload.thermalBar.elements,
      },
      null,
      2,
    );
  }

  if (payload.axial) {
    return JSON.stringify(
      {
        kind: "axial_bar_1d",
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        length: payload.axial.length,
        area: payload.axial.area,
        elements: payload.axial.elements,
        tip_force: payload.axial.tip_force,
        youngs_modulus_gpa: payload.youngsModulusGpa,
      },
      null,
      2,
    );
  }

  return JSON.stringify({}, null, 2);
}

export function buildStudyModelPayload(
  kind: "axial_bar_1d" | "heat_bar_1d" | "thermal_bar_1d" | "thermal_beam_1d" | "thermal_frame_2d" | "thermal_truss_2d" | "thermal_truss_3d" | "thermal_plane_triangle_2d" | "thermal_plane_quad_2d" | "spring_1d" | "spring_2d" | "spring_3d" | "beam_1d" | "torsion_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d" | "plane_quad_2d" | "frame_2d",
  payload: {
    name: string;
    material: string;
    youngsModulusGpa: number;
    materials?: ModelMaterial[];
    axial?: AxialBarJobInput;
    truss?: Truss2dJobInput;
    truss3d?: Truss3dJobInput;
    thermalTruss?: ThermalTruss2dJobInput;
    thermalTruss3d?: ThermalTruss3dJobInput;
    plane?: PlaneTriangle2dJobInput | PlaneQuad2dJobInput;
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
  },
): Record<string, unknown> {
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

function member(
  id: string,
  nodeI: number,
  nodeJ: number,
  area: number,
  youngsModulus: number,
): TrussElementInput {
  return {
    id,
    node_i: nodeI,
    node_j: nodeJ,
    area,
    youngs_modulus: youngsModulus,
  };
}

function round(value: number): number {
  return Math.round(value * 1000) / 1000;
}
