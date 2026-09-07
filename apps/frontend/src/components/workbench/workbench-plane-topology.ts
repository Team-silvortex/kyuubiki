type IndexedPlaneNode = { index: number; x: number; y: number };
type PlaneConnectivity = { node_i: number; node_j: number; node_k: number; node_l?: number };

export function findPlaneItemByIndex<T extends { index: number }>(items: readonly T[], index: number | null): T | null {
  if (index === null || !Number.isInteger(index) || index < 0) return null;
  // Keep the dense, ordered path constant-time without assuming all results are dense.
  return items[index]?.index === index ? items[index] : items.find((item) => item.index === index) ?? null;
}

export function buildPlaneNodeIndex<T extends IndexedPlaneNode>(nodes: readonly T[]): ReadonlyMap<number, T> {
  const lookup = new Map<number, T>();
  const ambiguous = new Set<number>();
  for (const node of nodes) {
    if (!Number.isInteger(node.index) || node.index < 0) continue;
    // Duplicate identities are unusable, even when only one copy has valid coordinates.
    if (lookup.has(node.index) || ambiguous.has(node.index)) {
      lookup.delete(node.index);
      ambiguous.add(node.index);
    } else if (!Number.isFinite(node.x) || !Number.isFinite(node.y)) {
      ambiguous.add(node.index);
    } else {
      lookup.set(node.index, node);
    }
  }
  return lookup;
}

export function resolvePlaneElementNodes<T>(element: PlaneConnectivity, lookup: ReadonlyMap<number, T>): T[] | null {
  const indices = [element.node_i, element.node_j, element.node_k];
  if (element.node_l !== undefined) indices.push(element.node_l);
  if (new Set(indices).size !== indices.length) return null;
  const nodes: T[] = [];
  for (const index of indices) {
    if (!Number.isInteger(index) || index < 0) return null;
    const node = lookup.get(index);
    if (node === undefined) return null;
    nodes.push(node);
  }
  return nodes;
}
