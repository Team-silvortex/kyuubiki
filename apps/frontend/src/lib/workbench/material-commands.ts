import { createCustomMaterial } from "@/lib/materials/material-library";
import { createMaterialDefinition } from "@/lib/materials/materials";
import type {
  ElectrostaticPlaneQuad2dJobInput,
  ElectrostaticPlaneTriangle2dJobInput,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
} from "@/lib/api/fem-2d-surface";
import type { Frame2dJobInput, Truss2dJobInput } from "@/lib/api/fem-2d-line";
import type { Truss3dJobInput } from "@/lib/api/fem-3d";
import type { ModelMaterial } from "@/lib/api/fem-shared";

type PlaneStudyJobInput =
  | ElectrostaticPlaneTriangle2dJobInput
  | ElectrostaticPlaneQuad2dJobInput
  | PlaneTriangle2dJobInput
  | PlaneQuad2dJobInput
  | ThermalPlaneTriangle2dJobInput
  | ThermalPlaneQuad2dJobInput;

type MaterialField = "name" | "youngs_modulus" | "poisson_ratio";

function nextMaterialIndex(materials: ModelMaterial[] | undefined) {
  const ids = new Set((materials ?? []).map((material) => material.id));
  let index = 1;
  while (ids.has(`mat-${index}`)) index += 1;
  return index;
}

function normalizeMaterialUpdateValue(field: MaterialField, value: string | number) {
  if (field === "name") return typeof value === "string" ? value : String(value);
  const numericValue = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numericValue)) return undefined;
  if (field === "youngs_modulus" && numericValue <= 0) return undefined;
  if (field === "poisson_ratio" && (numericValue <= -1 || numericValue >= 0.5)) return undefined;
  return numericValue;
}

export function nextMaterialId(materials: ModelMaterial[] | undefined) {
  return `mat-${nextMaterialIndex(materials)}`;
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

export function ensurePlaneModelMaterials<T extends PlaneStudyJobInput>(model: T, fallbackValue = "70"): T {
  const firstElement = model.elements[0];
  const fallbackPoisson =
    firstElement && "poisson_ratio" in firstElement && typeof firstElement.poisson_ratio === "number"
      ? firstElement.poisson_ratio
      : 0.33;
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
  } as T;
}

export function ensureFrameModelMaterials(model: Frame2dJobInput, fallbackValue = "70"): Frame2dJobInput {
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

export function addPresetMaterialToTrussModel(model: Truss2dJobInput, activeMaterial: string) {
  const nextIndex = nextMaterialIndex(model.materials);
  return {
    ...model,
    materials: [
      ...(model.materials ?? []),
      createMaterialDefinition(activeMaterial, nextIndex, {
        id: `mat-${nextIndex}`,
      }),
    ],
  };
}

export function addPresetMaterialToTruss3dModel(model: Truss3dJobInput, activeMaterial: string) {
  const nextIndex = nextMaterialIndex(model.materials);
  return {
    ...model,
    materials: [
      ...(model.materials ?? []),
      createMaterialDefinition(activeMaterial, nextIndex, {
        id: `mat-${nextIndex}`,
      }),
    ],
  };
}

export function addPresetMaterialToPlaneModel<T extends PlaneStudyJobInput>(model: T, activeMaterial: string): T {
  const firstElement = model.elements[0];
  const nextIndex = nextMaterialIndex(model.materials);
  const fallbackPoisson =
    firstElement && "poisson_ratio" in firstElement && typeof firstElement.poisson_ratio === "number"
      ? firstElement.poisson_ratio
      : 0.33;
  return {
    ...model,
    materials: [
      ...(model.materials ?? []),
      createMaterialDefinition(activeMaterial, nextIndex, {
        id: `mat-${nextIndex}`,
        poisson_ratio: fallbackPoisson,
      }),
    ],
  } as T;
}

export function addPresetMaterialToFrameModel(model: Frame2dJobInput, activeMaterial: string) {
  const nextIndex = nextMaterialIndex(model.materials);
  return {
    ...model,
    materials: [
      ...(model.materials ?? []),
      createMaterialDefinition(activeMaterial, nextIndex, {
        id: `mat-${nextIndex}`,
      }),
    ],
  };
}

export function addCustomMaterialToTrussModel(model: Truss2dJobInput) {
  return {
    ...model,
    materials: [...(model.materials ?? []), createCustomMaterial(nextMaterialIndex(model.materials))],
  };
}

export function addCustomMaterialToTruss3dModel(model: Truss3dJobInput) {
  return {
    ...model,
    materials: [...(model.materials ?? []), createCustomMaterial(nextMaterialIndex(model.materials))],
  };
}

export function addCustomMaterialToPlaneModel<T extends PlaneStudyJobInput>(model: T): T {
  return {
    ...model,
    materials: [...(model.materials ?? []), createCustomMaterial(nextMaterialIndex(model.materials))],
  } as T;
}

export function addCustomMaterialToFrameModel(model: Frame2dJobInput) {
  return {
    ...model,
    materials: [...(model.materials ?? []), createCustomMaterial(nextMaterialIndex(model.materials))],
  };
}

export function applyMaterialToTrussModel(
  model: Truss2dJobInput,
  materialId: string,
  mode: "selected" | "all",
  selectedElement: number | null,
) {
  const material = model.materials?.find((entry) => entry.id === materialId);
  if (!material) return model;
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
  if (!material) return model;
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

export function applyMaterialToPlaneModel<T extends PlaneStudyJobInput>(
  model: T,
  materialId: string,
  mode: "selected" | "all",
  selectedElement: number | null,
): T {
  const material = model.materials?.find((entry) => entry.id === materialId);
  if (!material) return model;
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      mode === "all" || index === selectedElement
        ? {
            ...element,
            material_id: materialId,
            ...("youngs_modulus" in element ? { youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus } : {}),
            ...(
              "poisson_ratio" in element
                ? {
                    poisson_ratio:
                      material?.poisson_ratio === null || material?.poisson_ratio === undefined
                        ? element.poisson_ratio
                        : material.poisson_ratio,
                  }
                : {}
            ),
          }
        : element,
    ),
  } as T;
}

export function applyMaterialToFrameModel(
  model: Frame2dJobInput,
  materialId: string,
  mode: "selected" | "all",
  selectedElement: number | null,
) {
  const material = model.materials?.find((entry) => entry.id === materialId);
  if (!material) return model;
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
  field: MaterialField,
  value: string | number,
) {
  const normalizedValue = normalizeMaterialUpdateValue(field, value);
  if (normalizedValue === undefined || !model.materials?.some((material) => material.id === materialId)) {
    return model;
  }
  const materials = (model.materials ?? []).map((material) =>
    material.id === materialId ? { ...material, [field]: normalizedValue } : material,
  );

  return {
    ...model,
    materials,
    elements: model.elements.map((element) =>
      element.material_id === materialId && field === "youngs_modulus"
        ? { ...element, youngs_modulus: Number(normalizedValue) }
        : element,
    ),
  };
}

export function updateMaterialInTruss3dModel(
  model: Truss3dJobInput,
  materialId: string,
  field: MaterialField,
  value: string | number,
) {
  const normalizedValue = normalizeMaterialUpdateValue(field, value);
  if (normalizedValue === undefined || !model.materials?.some((material) => material.id === materialId)) {
    return model;
  }
  const materials = (model.materials ?? []).map((material) =>
    material.id === materialId ? { ...material, [field]: normalizedValue } : material,
  );

  return {
    ...model,
    materials,
    elements: model.elements.map((element) =>
      element.material_id === materialId && field === "youngs_modulus"
        ? { ...element, youngs_modulus: Number(normalizedValue) }
        : element,
    ),
  };
}

export function updateMaterialInPlaneModel<T extends PlaneStudyJobInput>(
  model: T,
  materialId: string,
  field: MaterialField,
  value: string | number,
): T {
  const normalizedValue = normalizeMaterialUpdateValue(field, value);
  if (normalizedValue === undefined || !model.materials?.some((material) => material.id === materialId)) {
    return model;
  }
  const materials = (model.materials ?? []).map((material) =>
    material.id === materialId ? { ...material, [field]: normalizedValue } : material,
  );

  return {
    ...model,
    materials,
    elements: model.elements.map((element) => {
      if (element.material_id !== materialId) return element;
      if (field === "youngs_modulus" && "youngs_modulus" in element) {
        return { ...element, youngs_modulus: Number(normalizedValue) };
      }
      if (field === "poisson_ratio" && "poisson_ratio" in element) {
        return { ...element, poisson_ratio: Number(normalizedValue) };
      }
      return element;
    }),
  } as T;
}

export function updateMaterialInFrameModel(
  model: Frame2dJobInput,
  materialId: string,
  field: MaterialField,
  value: string | number,
) {
  const normalizedValue = normalizeMaterialUpdateValue(field, value);
  if (normalizedValue === undefined || !model.materials?.some((material) => material.id === materialId)) {
    return model;
  }
  const materials = (model.materials ?? []).map((material) =>
    material.id === materialId ? { ...material, [field]: normalizedValue } : material,
  );

  return {
    ...model,
    materials,
    elements: model.elements.map((element) =>
      element.material_id === materialId && field === "youngs_modulus"
        ? { ...element, youngs_modulus: Number(normalizedValue) }
        : element,
    ),
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

export function deleteMaterialFromPlaneModel<T extends PlaneStudyJobInput>(model: T, materialId: string): T {
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
            ...("youngs_modulus" in element ? { youngs_modulus: fallback?.youngs_modulus ?? element.youngs_modulus } : {}),
            ...(
              "poisson_ratio" in element
                ? {
                    poisson_ratio:
                      fallback?.poisson_ratio === null || fallback?.poisson_ratio === undefined
                        ? element.poisson_ratio
                        : fallback.poisson_ratio,
                  }
                : {}
            ),
          }
        : element,
    ),
  } as T;
}

export function deleteMaterialFromFrameModel(model: Frame2dJobInput, materialId: string) {
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
