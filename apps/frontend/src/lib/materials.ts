import type { ModelMaterial } from "@/lib/api";

export type MaterialPreset = {
  label: string;
  value: string;
  modulusGpa: number | null;
  poissonRatio?: number | null;
};

export const MATERIAL_PRESETS: MaterialPreset[] = [
  { label: "Steel", value: "210", modulusGpa: 210, poissonRatio: 0.3 },
  { label: "Aluminum", value: "70", modulusGpa: 70, poissonRatio: 0.33 },
  { label: "Titanium", value: "116", modulusGpa: 116, poissonRatio: 0.34 },
  { label: "Concrete", value: "30", modulusGpa: 30, poissonRatio: 0.2 },
  { label: "Carbon fiber", value: "135", modulusGpa: 135, poissonRatio: 0.28 },
  { label: "Custom", value: "custom", modulusGpa: null },
];

export function materialLabel(value: string): string {
  return MATERIAL_PRESETS.find((preset) => preset.value === value)?.label ?? "Custom";
}

export function findMaterialPreset(value: string): MaterialPreset | undefined {
  return MATERIAL_PRESETS.find((preset) => preset.value === value);
}

export function createMaterialDefinition(
  value: string,
  nextIndex = 1,
  overrides?: Partial<ModelMaterial>,
): ModelMaterial {
  const preset = findMaterialPreset(value);
  const fallbackName = preset?.label ?? "Custom";
  const fallbackModulus = preset?.modulusGpa ?? 70;

  return {
    id: overrides?.id ?? `mat-${nextIndex}`,
    name: overrides?.name ?? fallbackName,
    youngs_modulus: overrides?.youngs_modulus ?? fallbackModulus * 1.0e9,
    poisson_ratio:
      overrides?.poisson_ratio === undefined
        ? preset?.poissonRatio ?? null
        : overrides.poisson_ratio,
  };
}
