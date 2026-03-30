const MODEL_SCHEMA_VERSION = "kyuubiki.model/v1";

const MATERIAL_PRESETS = new Map([
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

function requiredNumber(value, name) {
  const number = Number(value);

  if (!Number.isFinite(number) || number <= 0) {
    throw new Error(`${name} must be a positive number`);
  }

  return number;
}

function assertSupportedVersion(raw) {
  if (raw.model_schema_version === undefined) return;

  if (raw.model_schema_version !== MODEL_SCHEMA_VERSION) {
    throw new Error(`unsupported model_schema_version: ${String(raw.model_schema_version)}`);
  }
}

function normalizeMaterial(value) {
  const materialKey = typeof value === "string" ? value.trim().toLowerCase() : "";
  return MATERIAL_PRESETS.get(materialKey) ?? "custom";
}

export function parsePlaygroundModel(text) {
  const raw = JSON.parse(text);
  assertSupportedVersion(raw);

  if ((raw.kind ?? "axial_bar_1d") !== "axial_bar_1d") {
    throw new Error("legacy playground only supports axial_bar_1d imports");
  }

  return {
    kind: "axial_bar_1d",
    name: typeof raw.name === "string" ? raw.name : "imported-model",
    length: requiredNumber(raw.length, "length"),
    area: requiredNumber(raw.area, "area"),
    elements: Math.trunc(requiredNumber(raw.elements, "elements")),
    tipForce: Number(raw.tip_force ?? raw.tipForce ?? 0),
    material: normalizeMaterial(raw.material),
    youngsModulusGpa: requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa"),
  };
}
