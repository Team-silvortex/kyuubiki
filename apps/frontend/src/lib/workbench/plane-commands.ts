import type {
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
} from "@/lib/api";

type PlaneStudyJobInput =
  | PlaneTriangle2dJobInput
  | PlaneQuad2dJobInput
  | ThermalPlaneTriangle2dJobInput
  | ThermalPlaneQuad2dJobInput;

export function updatePlaneNode(
  model: PlaneStudyJobInput,
  selectedNode: number | null,
  key: "x" | "y" | "load_x" | "load_y" | "fix_x" | "fix_y" | "temperature_delta",
  value: number | boolean,
) {
  if (selectedNode === null) return model;
  return {
    ...model,
    nodes: model.nodes.map((node, index) =>
      index === selectedNode ? { ...node, [key]: value } : node,
    ),
  };
}

export function updatePlaneElement(
  model: PlaneStudyJobInput,
  selectedElement: number | null,
  key: "thickness" | "youngs_modulus" | "poisson_ratio" | "thermal_expansion",
  value: number,
) {
  if (selectedElement === null) return model;
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      index === selectedElement ? { ...element, [key]: value } : element,
    ),
  };
}

export function assignPlaneElementMaterial(
  model: PlaneStudyJobInput,
  selectedElement: number | null,
  materialId: string,
) {
  if (selectedElement === null) return model;
  const material = model.materials?.find((entry) => entry.id === materialId);
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      index === selectedElement
        ? {
            ...element,
            material_id: materialId,
            youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
            poisson_ratio:
              material?.poisson_ratio === null || material?.poisson_ratio === undefined
                ? element.poisson_ratio
                : material.poisson_ratio,
          }
        : element,
    ),
  };
}
