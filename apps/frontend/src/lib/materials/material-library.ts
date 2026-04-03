import type { ModelMaterial } from "@/lib/api";
import { createMaterialDefinition } from "@/lib/materials";

function positiveNumber(value: unknown, field: string) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) {
    throw new Error(`${field} must be a positive number`);
  }
  return number;
}

function optionalNumber(value: unknown): number | null {
  if (value === undefined || value === null || value === "") return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function parseMaterialRecord(entry: Record<string, unknown>, index: number): ModelMaterial {
  const id = typeof entry.id === "string" && entry.id.trim().length > 0 ? entry.id : `mat-import-${index + 1}`;
  const name = typeof entry.name === "string" && entry.name.trim().length > 0 ? entry.name : `Imported ${index + 1}`;
  const youngsModulus =
    entry.youngs_modulus_gpa !== undefined
      ? positiveNumber(entry.youngs_modulus_gpa, `materials[${index}].youngs_modulus_gpa`) * 1.0e9
      : positiveNumber(entry.youngs_modulus, `materials[${index}].youngs_modulus`);

  return {
    id,
    name,
    youngs_modulus: youngsModulus,
    poisson_ratio: optionalNumber(entry.poisson_ratio),
  };
}

function parseJsonMaterials(text: string): ModelMaterial[] {
  const raw = JSON.parse(text) as unknown;
  const records =
    Array.isArray(raw)
      ? raw
      : typeof raw === "object" && raw !== null && Array.isArray((raw as { materials?: unknown[] }).materials)
        ? (raw as { materials: unknown[] }).materials
        : null;

  if (!records) {
    throw new Error("JSON material library must be an array or an object with a materials array");
  }

  return records.map((entry, index) => parseMaterialRecord((entry ?? {}) as Record<string, unknown>, index));
}

function parseCsvMaterials(text: string): ModelMaterial[] {
  const rows = text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);

  if (rows.length < 2) {
    throw new Error("CSV material library needs a header row and at least one material row");
  }

  const headers = rows[0].split(",").map((value) => value.trim().toLowerCase());

  return rows.slice(1).map((row, index) => {
    const values = row.split(",").map((value) => value.trim());
    const entry = Object.fromEntries(headers.map((header, headerIndex) => [header, values[headerIndex] ?? ""]));
    return parseMaterialRecord(entry, index);
  });
}

export function parseMaterialLibrary(text: string, hint?: string): ModelMaterial[] {
  const trimmed = text.trim();
  if (trimmed.length === 0) {
    throw new Error("material library is empty");
  }

  const looksJson = hint?.toLowerCase().endsWith(".json") || trimmed.startsWith("{") || trimmed.startsWith("[");
  return looksJson ? parseJsonMaterials(trimmed) : parseCsvMaterials(trimmed);
}

export function createCustomMaterial(nextIndex: number): ModelMaterial {
  return createMaterialDefinition("custom", nextIndex, {
    id: `mat-${nextIndex}`,
    name: `Custom ${nextIndex}`,
    youngs_modulus: 70e9,
    poisson_ratio: 0.33,
  });
}
