const MATERIAL_PRESETS = new Map([
  ["steel", "210"],
  ["aluminum", "70"],
  ["titanium", "116"],
  ["concrete", "30"],
  ["carbon fiber", "135"],
  ["carbon_fiber", "135"],
]);

function requiredNumber(value, name) {
  const number = Number(value);

  if (!Number.isFinite(number) || number <= 0) {
    throw new Error(`${name} must be a positive number`);
  }

  return number;
}

export function parsePlaygroundModel(text) {
  const raw = JSON.parse(text);
  const modulus = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materialKey = typeof raw.material === "string" ? raw.material.trim().toLowerCase() : "";
  const preset = MATERIAL_PRESETS.get(materialKey);

  return {
    name: typeof raw.name === "string" ? raw.name : "imported-model",
    length: requiredNumber(raw.length, "length"),
    area: requiredNumber(raw.area, "area"),
    elements: Math.trunc(requiredNumber(raw.elements, "elements")),
    tipForce: Number(raw.tip_force ?? raw.tipForce ?? 0),
    material: preset ?? "custom",
    youngsModulusGpa: modulus,
  };
}
