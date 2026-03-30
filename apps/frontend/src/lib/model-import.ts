export type ImportedModel = {
  name: string;
  length: number;
  area: number;
  elements: number;
  tipForce: number;
  material: string;
  youngsModulusGpa: number;
};

const MATERIAL_MAP = new Map([
  ["steel", "210"],
  ["aluminum", "70"],
  ["titanium", "116"],
  ["concrete", "30"],
  ["carbon fiber", "135"],
  ["carbon_fiber", "135"],
]);

function requiredNumber(value: unknown, name: string): number {
  const number = Number(value);

  if (!Number.isFinite(number) || number <= 0) {
    throw new Error(`${name} must be a positive number`);
  }

  return number;
}

export function parsePlaygroundModel(text: string): ImportedModel {
  const raw = JSON.parse(text) as Record<string, unknown>;
  const modulus = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materialKey = typeof raw.material === "string" ? raw.material.trim().toLowerCase() : "";

  return {
    name: typeof raw.name === "string" ? raw.name : "imported-model",
    length: requiredNumber(raw.length, "length"),
    area: requiredNumber(raw.area, "area"),
    elements: Math.trunc(requiredNumber(raw.elements, "elements")),
    tipForce: Number(raw.tip_force ?? raw.tipForce ?? 0),
    material: MATERIAL_MAP.get(materialKey) ?? "custom",
    youngsModulusGpa: modulus,
  };
}
