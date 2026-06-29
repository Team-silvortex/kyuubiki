import type { ModelMaterial } from "@/lib/api/fem-shared";
import { createMaterialDefinition } from "@/lib/materials/materials";

const MODEL_SCHEMA_VERSION = "kyuubiki.model/v1";

const MATERIAL_MAP = new Map([
  ["steel", "210"],
  ["210", "210"],
  ["aluminum", "70"],
  ["70", "70"],
  ["titanium", "116"],
  ["116", "116"],
  ["concrete", "30"],
  ["30", "30"],
  ["carbon fiber", "135"],
  ["carbon_fiber", "135"],
  ["135", "135"],
]);

export function requiredNumber(value: unknown, name: string): number {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) throw new Error(`${name} must be a positive number`);
  return number;
}

export function numberOrZero(value: unknown): number {
  const number = Number(value ?? 0);
  return Number.isFinite(number) ? number : 0;
}

export function requiredNonNegativeInteger(value: unknown, name: string): number {
  const number = Number(value);
  if (!Number.isInteger(number) || number < 0) throw new Error(`${name} must be a non-negative integer`);
  return number;
}

export function requiredString(value: unknown, name: string): string {
  if (typeof value !== "string" || value.trim().length === 0) throw new Error(`${name} must be a non-empty string`);
  return value;
}

export function normalizeMaterial(value: unknown): string {
  const materialKey = typeof value === "string" ? value.trim().toLowerCase() : "";
  return MATERIAL_MAP.get(materialKey) ?? "custom";
}

export function optionalString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim().length > 0 ? value : undefined;
}

export function parseMaterials(
  raw: Record<string, unknown>,
  fallbackMaterial: string,
  fallbackYoungsModulusGpa: number,
  fallbackPoissonRatio?: number,
): ModelMaterial[] {
  if (!Array.isArray(raw.materials) || raw.materials.length === 0) {
    return [
      createMaterialDefinition(fallbackMaterial, 1, {
        id: "mat-1",
        youngs_modulus: fallbackYoungsModulusGpa * 1.0e9,
        poisson_ratio: fallbackPoissonRatio ?? null,
      }),
    ];
  }

  return raw.materials.map((entry, index) => {
    const material = (entry ?? {}) as Record<string, unknown>;
    return createMaterialDefinition(fallbackMaterial, index + 1, {
      id: requiredString(material.id, `materials[${index}].id`),
      name: optionalString(material.name) ?? createMaterialDefinition(fallbackMaterial, index + 1).name,
      youngs_modulus: requiredNumber(material.youngs_modulus, `materials[${index}].youngs_modulus`),
      poisson_ratio:
        material.poisson_ratio === undefined || material.poisson_ratio === null
          ? fallbackPoissonRatio ?? null
          : Number(material.poisson_ratio),
    });
  });
}

export function assertSupportedVersion(raw: Record<string, unknown>) {
  const version = raw.model_schema_version;
  if (version === undefined) return;
  if (version !== MODEL_SCHEMA_VERSION) throw new Error(`unsupported model_schema_version: ${String(version)}`);
}
