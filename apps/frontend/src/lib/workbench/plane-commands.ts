import type { PlaneTriangle2dJobInput } from "@/lib/api";

export function updatePlaneNode(
  model: PlaneTriangle2dJobInput,
  selectedNode: number | null,
  key: keyof PlaneTriangle2dJobInput["nodes"][number],
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
  model: PlaneTriangle2dJobInput,
  selectedElement: number | null,
  key: keyof PlaneTriangle2dJobInput["elements"][number],
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
  model: PlaneTriangle2dJobInput,
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
