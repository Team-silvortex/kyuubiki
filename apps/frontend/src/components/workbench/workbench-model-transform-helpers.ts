import { createMaterialDefinition } from "@/lib/materials";
import type {
  Frame2dJobInput,
  HeatBar1dJobInput,
  HeatBar1dResult,
  HeatPlaneQuad2dJobInput,
  HeatPlaneQuad2dResult,
  HeatPlaneTriangle2dJobInput,
  HeatPlaneTriangle2dResult,
  ThermalBar1dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
} from "@/lib/api";
import {
  defaultThermalBar1d,
  defaultThermalPlaneQuad,
  defaultThermalPlaneTriangle,
} from "@/components/workbench/workbench-defaults";
import { ensurePlaneModelMaterials } from "@/lib/workbench/material-commands";

export function ensureBeamModelMaterials<
  T extends {
    materials?: Array<{ id: string }>;
    elements: Array<{ material_id?: string | undefined }>;
  },
>(model: T, materialValue: string): T {
  const existingMaterials = model.materials?.length
    ? model.materials
    : [createMaterialDefinition(materialValue, 1, { id: "mat-1" })];
  const defaultMaterialId = existingMaterials[0]?.id ?? "mat-1";
  return {
    ...model,
    materials: existingMaterials,
    elements: model.elements.map((element) => ({
      ...element,
      material_id: element.material_id ?? defaultMaterialId,
    })),
  };
}

export function ensureFrameModelMaterials(
  model: Frame2dJobInput,
  materialValue: string,
): Frame2dJobInput {
  const existingMaterials = model.materials?.length
    ? model.materials
    : [createMaterialDefinition(materialValue, 1, { id: "mat-1" })];
  const defaultMaterialId = existingMaterials[0]?.id ?? "mat-1";
  return {
    ...model,
    materials: existingMaterials,
    elements: model.elements.map((element) => ({
      ...element,
      material_id: element.material_id ?? defaultMaterialId,
    })),
  };
}

export function buildThermalBarFromHeatResult(
  sourceModel: HeatBar1dJobInput,
  result: HeatBar1dResult,
  fallbackModel: ThermalBar1dJobInput,
): ThermalBar1dJobInput {
  const sameTopology =
    fallbackModel.nodes.length === sourceModel.nodes.length &&
    fallbackModel.elements.length === sourceModel.elements.length;
  const defaultElement = defaultThermalBar1d.elements[0];

  return {
    project_id: sourceModel.project_id,
    model_version_id: sourceModel.model_version_id,
    nodes: sourceModel.nodes.map((node, index) => ({
      id: node.id,
      x: node.x,
      fix_x: sameTopology
        ? fallbackModel.nodes[index]?.fix_x ?? node.fix_temperature
        : node.fix_temperature,
      load_x: sameTopology ? fallbackModel.nodes[index]?.load_x ?? 0 : 0,
      temperature_delta: result.nodes[index]?.temperature ?? node.temperature ?? 0,
    })),
    elements: sourceModel.elements.map((element, index) => ({
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      area: element.area,
      youngs_modulus: sameTopology
        ? fallbackModel.elements[index]?.youngs_modulus ??
          defaultElement.youngs_modulus
        : defaultElement.youngs_modulus,
      thermal_expansion: sameTopology
        ? fallbackModel.elements[index]?.thermal_expansion ??
          defaultElement.thermal_expansion
        : defaultElement.thermal_expansion,
    })),
  };
}

export function buildThermalPlaneTriangleFromHeatResult(
  sourceModel: HeatPlaneTriangle2dJobInput,
  result: HeatPlaneTriangle2dResult,
  fallbackModel: ThermalPlaneTriangle2dJobInput,
  fallbackMaterial: string,
): ThermalPlaneTriangle2dJobInput {
  const sameTopology =
    fallbackModel.nodes.length === sourceModel.nodes.length &&
    fallbackModel.elements.length === sourceModel.elements.length;
  const fallbackMaterials =
    sameTopology && fallbackModel.materials?.length
      ? fallbackModel.materials
      : defaultThermalPlaneTriangle.materials;

  return ensurePlaneModelMaterials(
    {
      materials: fallbackMaterials,
      project_id: sourceModel.project_id,
      model_version_id: sourceModel.model_version_id,
      nodes: sourceModel.nodes.map((node, index) => ({
        id: node.id,
        x: node.x,
        y: node.y,
        fix_x: sameTopology
          ? fallbackModel.nodes[index]?.fix_x ?? node.fix_temperature
          : node.fix_temperature,
        fix_y: sameTopology
          ? fallbackModel.nodes[index]?.fix_y ?? node.fix_temperature
          : node.fix_temperature,
        load_x: sameTopology ? fallbackModel.nodes[index]?.load_x ?? 0 : 0,
        load_y: sameTopology ? fallbackModel.nodes[index]?.load_y ?? 0 : 0,
        temperature_delta: result.nodes[index]?.temperature ?? node.temperature ?? 0,
      })),
      elements: sourceModel.elements.map((element, index) => {
        const template = sameTopology
          ? fallbackModel.elements[index]
          : defaultThermalPlaneTriangle.elements[
              Math.min(index, defaultThermalPlaneTriangle.elements.length - 1)
            ];
        return {
          id: element.id,
          node_i: element.node_i,
          node_j: element.node_j,
          node_k: element.node_k,
          thickness: element.thickness,
          youngs_modulus:
            template?.youngs_modulus ??
            defaultThermalPlaneTriangle.elements[0].youngs_modulus,
          poisson_ratio:
            template?.poisson_ratio ??
            defaultThermalPlaneTriangle.elements[0].poisson_ratio,
          thermal_expansion:
            template?.thermal_expansion ??
            defaultThermalPlaneTriangle.elements[0].thermal_expansion,
          material_id: template?.material_id ?? fallbackMaterials?.[0]?.id,
        };
      }),
    },
    fallbackMaterial,
  ) as ThermalPlaneTriangle2dJobInput;
}

export function buildThermalPlaneQuadFromHeatResult(
  sourceModel: HeatPlaneQuad2dJobInput,
  result: HeatPlaneQuad2dResult,
  fallbackModel: ThermalPlaneQuad2dJobInput,
  fallbackMaterial: string,
): ThermalPlaneQuad2dJobInput {
  const sameTopology =
    fallbackModel.nodes.length === sourceModel.nodes.length &&
    fallbackModel.elements.length === sourceModel.elements.length;
  const fallbackMaterials =
    sameTopology && fallbackModel.materials?.length
      ? fallbackModel.materials
      : defaultThermalPlaneQuad.materials;

  return ensurePlaneModelMaterials(
    {
      materials: fallbackMaterials,
      project_id: sourceModel.project_id,
      model_version_id: sourceModel.model_version_id,
      nodes: sourceModel.nodes.map((node, index) => ({
        id: node.id,
        x: node.x,
        y: node.y,
        fix_x: sameTopology
          ? fallbackModel.nodes[index]?.fix_x ?? node.fix_temperature
          : node.fix_temperature,
        fix_y: sameTopology
          ? fallbackModel.nodes[index]?.fix_y ?? node.fix_temperature
          : node.fix_temperature,
        load_x: sameTopology ? fallbackModel.nodes[index]?.load_x ?? 0 : 0,
        load_y: sameTopology ? fallbackModel.nodes[index]?.load_y ?? 0 : 0,
        temperature_delta: result.nodes[index]?.temperature ?? node.temperature ?? 0,
      })),
      elements: sourceModel.elements.map((element, index) => {
        const template = sameTopology
          ? fallbackModel.elements[index]
          : defaultThermalPlaneQuad.elements[
              Math.min(index, defaultThermalPlaneQuad.elements.length - 1)
            ];
        return {
          id: element.id,
          node_i: element.node_i,
          node_j: element.node_j,
          node_k: element.node_k,
          node_l: element.node_l,
          thickness: element.thickness,
          youngs_modulus:
            template?.youngs_modulus ??
            defaultThermalPlaneQuad.elements[0].youngs_modulus,
          poisson_ratio:
            template?.poisson_ratio ??
            defaultThermalPlaneQuad.elements[0].poisson_ratio,
          thermal_expansion:
            template?.thermal_expansion ??
            defaultThermalPlaneQuad.elements[0].thermal_expansion,
          material_id: template?.material_id ?? fallbackMaterials?.[0]?.id,
        };
      }),
    },
    fallbackMaterial,
  ) as ThermalPlaneQuad2dJobInput;
}
