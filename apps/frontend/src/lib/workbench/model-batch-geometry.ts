import {
  batchAssert, batchAxis, finiteBatchNumber, type BatchAxis, type BatchModel, type BatchNode,
} from "./model-batch-selection.ts";

export type BatchVector = { x: number; y: number; z?: number };
export type BatchPivot = { kind: "origin" | "selection" } | { kind: "point"; point: BatchVector };
export type ModelTransformOperation = (
  | { kind: "rotate"; axis: BatchAxis; angleDegrees: number; pivot: BatchPivot }
  | { kind: "scale"; factors: BatchVector; pivot: BatchPivot }
  | { kind: "mirror"; axis: BatchAxis; pivot: BatchPivot }
) & { copy?: boolean; copyBoundaryConditions?: boolean };
export type ModelGeometryOperation = ModelTransformOperation
  | { kind: "snap"; axes: BatchAxis[]; spacing: number };

export function validateBatchVector(vector: BatchVector, spatial: boolean, planarZ = 0) {
  batchAssert(vector && typeof vector === "object", "invalid_number");
  finiteBatchNumber(vector.x); finiteBatchNumber(vector.y);
  if (vector.z !== undefined) finiteBatchNumber(vector.z);
  batchAssert(spatial || (vector.z ?? planarZ) === planarZ, "unsupported_axis");
}

export function patchBatchNode(node: BatchNode, fields: Partial<BatchNode>): BatchNode {
  return Object.entries(fields).every(([key, value]) => node[key as keyof BatchNode] === value)
    ? node : { ...node, ...fields };
}

export function shiftedBatchNode(node: BatchNode, offset: BatchVector, multiplier = 1) {
  const x = node.x + offset.x * multiplier, y = node.y + offset.y * multiplier;
  const z = node.z === undefined ? undefined : node.z + (offset.z ?? 0) * multiplier;
  finiteBatchNumber(x); finiteBatchNumber(y);
  if (z !== undefined) finiteBatchNumber(z);
  return patchBatchNode(node, { x, y, ...(z === undefined ? {} : { z }) });
}

export function assertValidBatchGeometry(model: BatchModel, memberIndices: number[]) {
  for (const index of memberIndices) {
    const member = model.elements[index];
    const a = model.nodes[member.node_i], b = model.nodes[member.node_j];
    batchAssert(a && b, "invalid_topology");
    const length = Math.hypot(a.x - b.x, a.y - b.y, (a.z ?? 0) - (b.z ?? 0));
    batchAssert(Number.isFinite(length) && length > 1e-12, "degenerate_member");
  }
}

function resolvePivot(model: BatchModel, indices: number[], pivot: BatchPivot, spatial: boolean) {
  batchAssert(pivot && typeof pivot === "object", "invalid_pivot");
  if (pivot.kind === "origin") return { x: 0, y: 0, z: 0 };
  if (pivot.kind === "point") {
    validateBatchVector(pivot.point, spatial);
    return { ...pivot.point, z: pivot.point.z ?? 0 };
  }
  batchAssert(pivot.kind === "selection", "invalid_pivot");
  const min = { x: Infinity, y: Infinity, z: Infinity }, max = { x: -Infinity, y: -Infinity, z: -Infinity };
  for (const index of indices) {
    const node = model.nodes[index];
    for (const axis of ["x", "y", "z"] as const) {
      const value = node[axis] ?? 0;
      min[axis] = Math.min(min[axis], value); max[axis] = Math.max(max[axis], value);
    }
  }
  // Halve first so a finite bounds center does not overflow when endpoints have the same sign.
  return { x: min.x / 2 + max.x / 2, y: min.y / 2 + max.y / 2, z: min.z / 2 + max.z / 2 };
}

export function createBatchGeometryTransform(model: BatchModel, indices: number[], operation: ModelGeometryOperation, spatial: boolean) {
  for (const index of indices) {
    const node = model.nodes[index];
    validateBatchVector(node, spatial);
    batchAssert((node.z !== undefined) === spatial, "invalid_dimension");
  }
  if (operation.kind === "snap") {
    finiteBatchNumber(operation.spacing);
    batchAssert(operation.spacing > 0, "invalid_spacing");
    batchAssert(Array.isArray(operation.axes) && operation.axes.length > 0 && operation.axes.length <= 3, "invalid_axes");
    const axes = [...new Set(operation.axes)];
    for (const axis of axes) { batchAxis(axis); batchAssert(spatial || axis !== "z", "unsupported_axis"); }
    return (node: BatchNode) => {
      const fields: Partial<BatchNode> = {};
      for (const axis of axes) {
        const coordinate = node[axis]! / operation.spacing;
        // Half steps round away from zero, symmetrically across the world origin.
        const steps = Math.sign(coordinate) * Math.round(Math.abs(coordinate));
        batchAssert(Number.isSafeInteger(steps), "grid_precision");
        const value = steps * operation.spacing;
        finiteBatchNumber(value);
        fields[axis] = value === 0 ? 0 : value;
      }
      return patchBatchNode(node, fields);
    };
  }
  batchAssert(operation.copy === undefined || typeof operation.copy === "boolean", "invalid_operation");
  batchAssert(operation.copyBoundaryConditions === undefined || typeof operation.copyBoundaryConditions === "boolean", "invalid_operation");
  batchAssert(!operation.copyBoundaryConditions || operation.copy === true, "copy_required");
  const pivot = resolvePivot(model, indices, operation.pivot, spatial);
  let transform: (node: BatchNode) => BatchVector;
  if (operation.kind === "rotate") {
    batchAxis(operation.axis);
    batchAssert(spatial || operation.axis === "z", "unsupported_axis");
    finiteBatchNumber(operation.angleDegrees);
    const degrees = operation.angleDegrees % 360;
    // Exact quarter turns avoid introducing tiny coordinate drift into orthogonal constructions.
    const quarter = degrees / 90;
    const quadrant = ((quarter % 4) + 4) % 4;
    const cosine = Number.isInteger(quarter) ? [1, 0, -1, 0][quadrant] : Math.cos(degrees * Math.PI / 180);
    const sine = Number.isInteger(quarter) ? [0, 1, 0, -1][quadrant] : Math.sin(degrees * Math.PI / 180);
    const [a, b] = operation.axis === "x" ? ["y", "z"] as const
      : operation.axis === "y" ? ["z", "x"] as const : ["x", "y"] as const;
    transform = degrees === 0 ? (node) => node : (node) => ({ ...node,
      [a]: pivot[a] + ((node[a] ?? 0) - pivot[a]) * cosine - ((node[b] ?? 0) - pivot[b]) * sine,
      [b]: pivot[b] + ((node[a] ?? 0) - pivot[a]) * sine + ((node[b] ?? 0) - pivot[b]) * cosine,
    });
  } else if (operation.kind === "scale") {
    validateBatchVector(operation.factors, spatial, 1);
    const factors = { ...operation.factors, z: operation.factors.z ?? 1 };
    batchAssert(factors.x > 0 && factors.y > 0 && factors.z > 0, "invalid_scale");
    const scale = (value: number, axis: BatchAxis) => factors[axis] === 1 ? value : pivot[axis] + (value - pivot[axis]) * factors[axis];
    transform = (node) => ({ x: scale(node.x, "x"), y: scale(node.y, "y"), ...(spatial ? { z: scale(node.z!, "z") } : {}) });
  } else {
    batchAxis(operation.axis);
    batchAssert(spatial || operation.axis !== "z", "unsupported_axis");
    transform = (node) => ({ ...node, [operation.axis]: pivot[operation.axis] + (pivot[operation.axis] - node[operation.axis]!) });
  }
  return (node: BatchNode) => {
    const point = transform(node);
    validateBatchVector(point, spatial);
    return patchBatchNode(node, { x: point.x, y: point.y, ...(spatial ? { z: point.z! } : {}) });
  };
}
