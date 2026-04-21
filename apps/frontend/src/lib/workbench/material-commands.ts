import { createCustomMaterial, createMaterialDefinition } from "@/lib/materials";
import type {
  ModelMaterial,
  PlaneTriangle2dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
} from "@/lib/api";

export function nextMaterialId(materials: ModelMaterial[] | undefined) {
  return `mat-${(materials?.length ?? 0) + 1}`;
}

export function ensureTrussModelMaterials(model: Truss2dJobInput, fallbackValue = "70"): Truss2dJobInput {
  const materials =
    model.materials && model.materials.length > 0
      ? model.materials
      : [createMaterialDefinition(fallbackValue, 1, { id: "mat-1" })];
  const defaultMaterialId = materials[0]?.id;

  return {
    ...model,
    materials,
    elements: model.elements.map((element) => ({
      ...element,
      material_id: element.material_id ?? defaultMaterialId,
    })),
  };
}

export function ensureTruss3dModelMaterials(model: Truss3dJobInput, fallbackValue = "70"): Truss3dJobInput {
  const materials =
    model.materials && model.materials.length > 0
      ? model.materials
      : [createMaterialDefinition(fallbackValue, 1, { id: "mat-1" })];
  const defaultMaterialId = materials[0]?.id;

  return {
    ...model,
    materials,
    elements: model.elements.map((element) => ({
      ...element,
      material_id: element.material_id ?? defaultMaterialId,
    })),
  };
}

export function ensurePlaneModelMaterials(model: PlaneTriangle2dJobInput, fallbackValue = "70"): PlaneTriangle2dJobInput {
  const fallbackPoisson = model.elements[0]?.poisson_ratio ?? 0.33;
  const materials =
    model.materials && model.materials.length > 0
      ? model.materials
      : [createMaterialDefinition(fallbackValue, 1, { id: "mat-1", poisson_ratio: fallbackPoisson })];
  const defaultMaterialId = materials[0]?.id;

  return {
    ...model,
    materials,
    elements: model.elements.map((element) => ({
      ...element,
      material_id: element.material_id ?? defaultMaterialId,
    })),
  };
}

export function addPresetMaterialToTrussModel(model: Truss2dJobInput, activeMaterial: string) {
  return {
    ...model,
    materials: [
      ...(model.materials ?? []),
      createMaterialDefinition(activeMaterial, (model.materials?.length ?? 0) + 1, {
        id: nextMaterialId(model.materials),
      }),
    ],
  };
}

export function addPresetMaterialToTruss3dModel(model: Truss3dJobInput, activeMaterial: string) {
  return {
    ...model,
    materials: [
      ...(model.materials ?? []),
      createMaterialDefinition(activeMaterial, (model.materials?.length ?? 0) + 1, {
        id: nextMaterialId(model.materials),
      }),
    ],
  };
}

export function addPresetMaterialToPlaneModel(model: PlaneTriangle2dJobInput, activeMaterial: string) {
  return {
    ...model,
    materials: [
      ...(model.materials ?? []),
      createMaterialDefinition(activeMaterial, (model.materials?.length ?? 0) + 1, {
        id: nextMaterialId(model.materials),
        poisson_ratio: model.elements[0]?.poisson_ratio ?? 0.33,
      }),
    ],
  };
}

export function addCustomMaterialToTrussModel(model: Truss2dJobInput) {
  return {
    ...model,
    materials: [...(model.materials ?? []), createCustomMaterial((model.materials?.length ?? 0) + 1)],
  };
}

export function addCustomMaterialToTruss3dModel(model: Truss3dJobInput) {
  return {
    ...model,
    materials: [...(model.materials ?? []), createCustomMaterial((model.materials?.length ?? 0) + 1)],
  };
}

export function addCustomMaterialToPlaneModel(model: PlaneTriangle2dJobInput) {
  return {
    ...model,
    materials: [...(model.materials ?? []), createCustomMaterial((model.materials?.length ?? 0) + 1)],
  };
}

export function applyMaterialToTrussModel(
  model: Truss2dJobInput,
  materialId: string,
  mode: "selected" | "all",
  selectedElement: number | null,
) {
  const material = model.materials?.find((entry) => entry.id === materialId);
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      mode === "all" || index === selectedElement
        ? {
            ...element,
            material_id: materialId,
            youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
          }
        : element,
    ),
  };
}

export function applyMaterialToTruss3dModel(
  model: Truss3dJobInput,
  materialId: string,
  mode: "selected" | "all",
  selectedElement: number | null,
) {
  const material = model.materials?.find((entry) => entry.id === materialId);
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      mode === "all" || index === selectedElement
        ? {
            ...element,
            material_id: materialId,
            youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
          }
        : element,
    ),
  };
}

export function applyMaterialToPlaneModel(
  model: PlaneTriangle2dJobInput,
  materialId: string,
  mode: "selected" | "all",
  selectedElement: number | null,
) {
  const material = model.materials?.find((entry) => entry.id === materialId);
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      mode === "all" || index === selectedElement
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

export function mergeImportedMaterials(current: ModelMaterial[] | undefined, imported: ModelMaterial[]) {
  const existing = current ?? [];
  const existingIds = new Set(existing.map((material) => material.id));
  const next = [...existing];

  imported.forEach((material, index) => {
    const baseId = material.id || `mat-import-${index + 1}`;
    let nextId = baseId;
    let suffix = 2;
    while (existingIds.has(nextId)) {
      nextId = `${baseId}-${suffix}`;
      suffix += 1;
    }
    existingIds.add(nextId);
    next.push({ ...material, id: nextId });
  });

  return next;
}

export function updateMaterialInTrussModel(
  model: Truss2dJobInput,
  materialId: string,
  field: "name" | "youngs_modulus" | "poisson_ratio",
  value: string | number,
) {
  const materials = (model.materials ?? []).map((material) =>
    material.id === materialId ? { ...material, [field]: value } : material,
  );

  return {
    ...model,
    materials,
    elements: model.elements.map((element) =>
      element.material_id === materialId && field === "youngs_modulus"
        ? { ...element, youngs_modulus: Number(value) }
        : element,
    ),
  };
}

export function updateMaterialInTruss3dModel(
  model: Truss3dJobInput,
  materialId: string,
  field: "name" | "youngs_modulus" | "poisson_ratio",
  value: string | number,
) {
  const materials = (model.materials ?? []).map((material) =>
    material.id === materialId ? { ...material, [field]: value } : material,
  );

  return {
    ...model,
    materials,
    elements: model.elements.map((element) =>
      element.material_id === materialId && field === "youngs_modulus"
        ? { ...element, youngs_modulus: Number(value) }
        : element,
    ),
  };
}

export function updateMaterialInPlaneModel(
  model: PlaneTriangle2dJobInput,
  materialId: string,
  field: "name" | "youngs_modulus" | "poisson_ratio",
  value: string | number,
) {
  const materials = (model.materials ?? []).map((material) =>
    material.id === materialId ? { ...material, [field]: value } : material,
  );

  return {
    ...model,
    materials,
    elements: model.elements.map((element) => {
      if (element.material_id !== materialId) return element;
      if (field === "youngs_modulus") return { ...element, youngs_modulus: Number(value) };
      if (field === "poisson_ratio") return { ...element, poisson_ratio: Number(value) };
      return element;
    }),
  };
}

export function deleteMaterialFromTrussModel(model: Truss2dJobInput, materialId: string) {
  const materials = model.materials ?? [];
  if (materials.length <= 1) return model;
  const nextMaterials = materials.filter((material) => material.id !== materialId);
  const fallback = nextMaterials[0];
  return {
    ...model,
    materials: nextMaterials,
    elements: model.elements.map((element) =>
      element.material_id === materialId
        ? {
            ...element,
            material_id: fallback?.id,
            youngs_modulus: fallback?.youngs_modulus ?? element.youngs_modulus,
          }
        : element,
    ),
  };
}

export function deleteMaterialFromTruss3dModel(model: Truss3dJobInput, materialId: string) {
  const materials = model.materials ?? [];
  if (materials.length <= 1) return model;
  const nextMaterials = materials.filter((material) => material.id !== materialId);
  const fallback = nextMaterials[0];
  return {
    ...model,
    materials: nextMaterials,
    elements: model.elements.map((element) =>
      element.material_id === materialId
        ? {
            ...element,
            material_id: fallback?.id,
            youngs_modulus: fallback?.youngs_modulus ?? element.youngs_modulus,
          }
        : element,
    ),
  };
}

export function deleteMaterialFromPlaneModel(model: PlaneTriangle2dJobInput, materialId: string) {
  const materials = model.materials ?? [];
  if (materials.length <= 1) return model;
  const nextMaterials = materials.filter((material) => material.id !== materialId);
  const fallback = nextMaterials[0];
  return {
    ...model,
    materials: nextMaterials,
    elements: model.elements.map((element) =>
      element.material_id === materialId
        ? {
            ...element,
            material_id: fallback?.id,
            youngs_modulus: fallback?.youngs_modulus ?? element.youngs_modulus,
            poisson_ratio:
              fallback?.poisson_ratio === null || fallback?.poisson_ratio === undefined
                ? element.poisson_ratio
                : fallback.poisson_ratio,
          }
        : element,
    ),
  };
}
