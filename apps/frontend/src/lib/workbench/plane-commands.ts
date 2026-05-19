import type {
  HeatPlaneQuad2dJobInput,
  HeatPlaneTriangle2dJobInput,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
} from "@/lib/api";

type PlaneStudyJobInput =
  | HeatPlaneTriangle2dJobInput
  | HeatPlaneQuad2dJobInput
  | PlaneTriangle2dJobInput
  | PlaneQuad2dJobInput
  | ThermalPlaneTriangle2dJobInput
  | ThermalPlaneQuad2dJobInput;

export function updatePlaneNode<T extends PlaneStudyJobInput>(
  model: T,
  selectedNode: number | null,
  key: "x" | "y" | "load_x" | "load_y" | "fix_x" | "fix_y" | "temperature_delta" | "fix_temperature" | "temperature" | "heat_load",
  value: number | boolean,
) {
  if (selectedNode === null) return model;
  return {
    ...model,
    nodes: model.nodes.map((node, index) =>
      index === selectedNode ? { ...node, [key]: value } : node,
    ),
  } as T;
}

export function updatePlaneElement<T extends PlaneStudyJobInput>(
  model: T,
  selectedElement: number | null,
  key: "thickness" | "youngs_modulus" | "poisson_ratio" | "thermal_expansion" | "conductivity",
  value: number,
) {
  if (selectedElement === null) return model;
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      index === selectedElement ? { ...element, [key]: value } : element,
    ),
  } as T;
}

export function assignPlaneElementMaterial<T extends PlaneStudyJobInput>(
  model: T,
  selectedElement: number | null,
  materialId: string,
) {
  if (selectedElement === null) return model;
  if (!("materials" in model) || !Array.isArray(model.materials) || !("material_id" in model.elements[selectedElement])) {
    return model;
  }
  const material = model.materials?.find((entry) => entry.id === materialId);
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      index === selectedElement
        ? {
            ...element,
            material_id: materialId,
            youngs_modulus: "youngs_modulus" in element ? material?.youngs_modulus ?? element.youngs_modulus : undefined,
            poisson_ratio:
              "poisson_ratio" in element
                ? material?.poisson_ratio === null || material?.poisson_ratio === undefined
                  ? element.poisson_ratio
                  : material.poisson_ratio
                : undefined,
          }
        : element,
    ),
  } as T;
}
