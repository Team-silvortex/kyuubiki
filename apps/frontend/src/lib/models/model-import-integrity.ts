import type { ImportedModel } from "@/lib/models/model-import-types";

type EntityRecord = Record<string, unknown>;

const NODE_REFERENCE_FIELDS = ["node_i", "node_j", "node_k", "node_l"] as const;

function asEntityRecords(value: unknown): EntityRecord[] {
  return Array.isArray(value) ? value.filter((entry): entry is EntityRecord => typeof entry === "object" && entry !== null) : [];
}

function assertUniqueEntityIds(entries: EntityRecord[], label: string) {
  const ids = new Set<string>();
  for (const entry of entries) {
    if (typeof entry.id !== "string") continue;
    if (ids.has(entry.id)) throw new Error(`duplicate ${label} id: ${entry.id}`);
    ids.add(entry.id);
  }
}

function assertElementReferences(
  elements: EntityRecord[],
  nodeCount: number,
  materialIds: Set<string>,
) {
  elements.forEach((element, elementIndex) => {
    for (const field of NODE_REFERENCE_FIELDS) {
      if (!(field in element)) continue;
      const nodeIndex = element[field];
      if (typeof nodeIndex !== "number" || !Number.isInteger(nodeIndex) || nodeIndex < 0 || nodeIndex >= nodeCount) {
        throw new Error(`elements[${elementIndex}].${field} references missing node ${String(nodeIndex)}`);
      }
    }

    if (typeof element.material_id === "string" && !materialIds.has(element.material_id)) {
      throw new Error(
        `elements[${elementIndex}].material_id references missing material: ${element.material_id}`,
      );
    }
  });
}

export function assertImportedModelIntegrity(imported: ImportedModel): ImportedModel {
  if (!("model" in imported)) return imported;

  const model = imported.model as unknown as EntityRecord;
  const nodes = asEntityRecords(model.nodes);
  const elements = asEntityRecords(model.elements);
  const materials = asEntityRecords(model.materials);

  assertUniqueEntityIds(nodes, "nodes");
  assertUniqueEntityIds(elements, "elements");
  assertUniqueEntityIds(materials, "materials");
  assertElementReferences(
    elements,
    nodes.length,
    new Set(materials.flatMap((material) => typeof material.id === "string" ? [material.id] : [])),
  );
  return imported;
}
