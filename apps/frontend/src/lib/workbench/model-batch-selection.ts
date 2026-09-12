import type { ModelMaterial, TrussElementInput, TrussNodeInput } from "@/lib/api";

export type BatchNode = TrussNodeInput & {
  z?: number; fix_z?: boolean; load_z?: number; fix_rz?: boolean; moment_z?: number;
};
export type BatchModel = {
  nodes: BatchNode[];
  elements: Array<TrussElementInput & { distributed_load_y?: number }>;
  materials?: ModelMaterial[];
};
export type BatchAxis = "x" | "y" | "z";
export type BatchQuery = (
  | { kind: "all" | "current" }
  | { kind: "indices"; indices: number[] }
  | { kind: "range"; axis: BatchAxis; min: number; max: number }
) & { invert?: boolean };

export class ModelBatchError extends Error {
  constructor(readonly code: string) { super(`model_batch:${code}`); }
}
export function batchAssert(condition: unknown, code: string): asserts condition {
  if (!condition) throw new ModelBatchError(code);
}
export function finiteBatchNumber(value: unknown): asserts value is number {
  batchAssert(typeof value === "number" && Number.isFinite(value), "invalid_number");
}
export function batchAxis(value: unknown): asserts value is BatchAxis {
  batchAssert(value === "x" || value === "y" || value === "z", "invalid_axis");
}

export function parseBatchIndices(text: string, nodeCount: number): number[] {
  batchAssert(text.trim().length > 0 && text.length <= 50_000, "invalid_indices");
  const indices = new Set<number>();
  let expanded = 0;
  for (const token of text.trim().split(/[,\s]+/)) {
    const match = /^(\d+)(?:-(\d+))?$/.exec(token);
    batchAssert(match, "invalid_indices");
    const start = Number(match[1]), end = Number(match[2] ?? match[1]);
    batchAssert(Number.isSafeInteger(start) && Number.isSafeInteger(end) && start <= end && end < nodeCount,
      "invalid_indices");
    expanded += end - start + 1;
    batchAssert(expanded <= Math.max(nodeCount * 2, 1000), "selection_limit");
    for (let index = start; index <= end; index += 1) indices.add(index);
  }
  return [...indices].sort((a, b) => a - b);
}

export function selectBatchNodes(model: BatchModel, query: BatchQuery, current: number[] = []): number[] {
  batchAssert(query && typeof query === "object", "invalid_query");
  batchAssert(query.invert === undefined || typeof query.invert === "boolean", "invalid_query");
  let selected: Set<number> | undefined;
  switch (query.kind) {
    case "all": break;
    case "current":
      selected = new Set(current.filter((index) => Number.isInteger(index) && index >= 0 && index < model.nodes.length));
      break;
    case "indices":
      batchAssert(Array.isArray(query.indices) && query.indices.length <= Math.max(model.nodes.length * 2, 1000), "invalid_indices");
      batchAssert(query.indices.every((index) => Number.isInteger(index) && index >= 0 && index < model.nodes.length), "invalid_indices");
      selected = new Set(query.indices);
      break;
    case "range":
      batchAxis(query.axis);
      finiteBatchNumber(query.min);
      finiteBatchNumber(query.max);
      batchAssert(query.min <= query.max, "invalid_range");
      batchAssert(query.axis !== "z" || model.nodes.every((node) => node.z !== undefined), "unsupported_axis");
      break;
    default: throw new ModelBatchError("invalid_query");
  }
  const indices: number[] = [];
  for (let index = 0; index < model.nodes.length; index += 1) {
    const node = model.nodes[index];
    const matched = query.kind === "range"
      ? Number.isFinite(node[query.axis]) && node[query.axis]! >= query.min && node[query.axis]! <= query.max
      : selected ? selected.has(index) : true;
    if (query.invert ? !matched : matched) indices.push(index);
  }
  return indices;
}

export function batchMemberIndices(model: BatchModel, selected: Set<number>, scope: "internal" | "touching") {
  batchAssert(scope === "internal" || scope === "touching", "invalid_scope");
  const indices: number[] = [];
  for (let index = 0; index < model.elements.length; index += 1) {
    const { node_i, node_j } = model.elements[index];
    if (scope === "internal" ? selected.has(node_i) && selected.has(node_j) : selected.has(node_i) || selected.has(node_j)) {
      indices.push(index);
    }
  }
  return indices;
}

export function inspectBatchSelection(model: BatchModel, query: BatchQuery, current: number[] = []) {
  const indices = selectBatchNodes(model, query, current);
  const selected = new Set(indices);
  return {
    nodes: indices.length,
    internalMembers: batchMemberIndices(model, selected, "internal").length,
    touchingMembers: batchMemberIndices(model, selected, "touching").length,
  };
}
