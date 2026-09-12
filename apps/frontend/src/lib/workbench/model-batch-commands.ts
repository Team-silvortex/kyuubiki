import {
  batchAssert, batchAxis, batchMemberIndices, finiteBatchNumber, ModelBatchError, selectBatchNodes,
  type BatchAxis, type BatchModel, type BatchNode, type BatchQuery,
} from "./model-batch-selection.ts";
import {
  assertValidBatchGeometry, createBatchGeometryTransform, patchBatchNode, shiftedBatchNode, validateBatchVector,
  type BatchVector, type ModelGeometryOperation,
} from "./model-batch-geometry.ts";
import { copyBatchGeometry, MODEL_BATCH_LIMITS } from "./model-batch-copy.ts";

export { MODEL_BATCH_LIMITS } from "./model-batch-copy.ts";
type Vector = BatchVector;
export type ModelBatchOperation =
  | ModelGeometryOperation
  | { kind: "translate"; offset: Vector }
  | { kind: "align"; axis: BatchAxis; value: number }
  | { kind: "array"; offset: Vector; copies: number; copyBoundaryConditions?: boolean }
  | { kind: "loads"; loads: Vector; distribution: "per_node" | "total" }
  | { kind: "supports"; axes: Array<BatchAxis | "rz">; fixed: boolean }
  | { kind: "members"; scope: "internal" | "touching"; area?: number; materialId?: string }
  | { kind: "delete"; confirmDelete: boolean };
export type ModelBatchRequest = { query: BatchQuery; operation: ModelBatchOperation };

export function applyModelBatch<T extends BatchModel>(model: T, request: ModelBatchRequest, current: number[] = []) {
  batchAssert(request && typeof request === "object", "invalid_request");
  const indices = selectBatchNodes(model, request.query, current);
  batchAssert(indices.length > 0, "empty_selection");
  const selected = new Set(indices);
  const spatial = model.nodes[indices[0]].z !== undefined;
  const operation = request.operation;
  batchAssert(operation && typeof operation === "object", "invalid_operation");
  let next: T = model;
  let nextSelection = indices;
  let affectedMembers = 0;
  const editNodes = (update: (node: BatchNode) => BatchNode) => {
    let nodes = model.nodes;
    for (const index of indices) {
      const node = update(model.nodes[index]);
      if (node === model.nodes[index]) continue;
      if (nodes === model.nodes) nodes = model.nodes.slice();
      nodes[index] = node;
    }
    if (nodes !== model.nodes) next = { ...model, nodes };
  };

  switch (operation.kind) {
    case "translate":
      validateBatchVector(operation.offset, spatial);
      editNodes((node) => shiftedBatchNode(node, operation.offset));
      break;
    case "align":
      batchAxis(operation.axis);
      finiteBatchNumber(operation.value);
      batchAssert(spatial || operation.axis !== "z", "unsupported_axis");
      editNodes((node) => patchBatchNode(node, { [operation.axis]: operation.value }));
      break;
    case "rotate": case "scale": case "mirror": case "snap": {
      const transform = createBatchGeometryTransform(model, indices, operation, spatial);
      if (operation.kind !== "snap" && operation.copy) {
        const memberIndices = batchMemberIndices(model, selected, "internal");
        const copied = copyBatchGeometry(model, indices, memberIndices, 1, transform, operation.copyBoundaryConditions);
        next = copied.model; nextSelection = copied.nextSelection;
        affectedMembers = memberIndices.length;
      } else editNodes(transform);
      break;
    }
    case "loads": {
      validateBatchVector(operation.loads, spatial);
      batchAssert(operation.distribution === "per_node" || operation.distribution === "total", "invalid_distribution");
      const divisor = operation.distribution === "total" ? indices.length : 1;
      editNodes((node) => patchBatchNode(node, {
        load_x: operation.loads.x / divisor, load_y: operation.loads.y / divisor,
        ...(spatial ? { load_z: (operation.loads.z ?? 0) / divisor } : {}),
      }));
      break;
    }
    case "supports": {
      batchAssert(typeof operation.fixed === "boolean" && Array.isArray(operation.axes) && operation.axes.length > 0 && operation.axes.length <= 4,
        "invalid_supports");
      const fields: Partial<BatchNode> = {};
      for (const axis of operation.axes) {
        batchAssert(["x", "y", "z", "rz"].includes(axis), "invalid_axis");
        batchAssert(axis !== "z" || spatial, "unsupported_axis");
        batchAssert(axis !== "rz" || indices.every((index) => typeof model.nodes[index].fix_rz === "boolean"), "unsupported_axis");
        fields[`fix_${axis}`] = operation.fixed;
      }
      editNodes((node) => patchBatchNode(node, fields));
      break;
    }
    case "members": {
      batchAssert(operation.area !== undefined || operation.materialId !== undefined, "invalid_properties");
      if (operation.area !== undefined) {
        finiteBatchNumber(operation.area);
        batchAssert(operation.area > 0, "invalid_properties");
      }
      const material = model.materials?.find((item) => item.id === operation.materialId);
      if (operation.materialId !== undefined) {
        batchAssert(typeof operation.materialId === "string" && material, "missing_material");
        finiteBatchNumber(material.youngs_modulus);
        batchAssert(material.youngs_modulus > 0, "invalid_properties");
      }
      const memberIndices = batchMemberIndices(model, selected, operation.scope);
      affectedMembers = memberIndices.length;
      batchAssert(affectedMembers > 0, "empty_members");
      let elements = model.elements;
      for (const index of memberIndices) {
        const member = model.elements[index];
        const fields = { ...(operation.area === undefined ? {} : { area: operation.area }),
          ...(material ? { material_id: material.id, youngs_modulus: material.youngs_modulus } : {}) };
        if (Object.entries(fields).every(([key, value]) => Object.is(member[key as keyof typeof member], value))) continue;
        if (elements === model.elements) elements = model.elements.slice();
        elements[index] = { ...member, ...fields };
      }
      if (elements !== model.elements) next = { ...model, elements };
      break;
    }
    case "array": {
      validateBatchVector(operation.offset, spatial);
      batchAssert(Number.isInteger(operation.copies) && operation.copies > 0 && operation.copies <= MODEL_BATCH_LIMITS.copies,
        "invalid_copies");
      batchAssert(operation.copyBoundaryConditions === undefined || typeof operation.copyBoundaryConditions === "boolean", "invalid_operation");
      batchAssert(Math.hypot(operation.offset.x, operation.offset.y, operation.offset.z ?? 0) > 1e-12, "zero_offset");
      const memberIndices = batchMemberIndices(model, selected, "internal");
      batchAssert(indices.length * operation.copies <= MODEL_BATCH_LIMITS.addedNodes &&
        memberIndices.length * operation.copies <= MODEL_BATCH_LIMITS.addedMembers, "array_limit");
      // Validate the farthest copy before allocating the array, including overflow in offset * copies.
      for (const index of indices) shiftedBatchNode(model.nodes[index], operation.offset, operation.copies);
      const copied = copyBatchGeometry(model, indices, memberIndices, operation.copies,
        (node, copy) => shiftedBatchNode(node, operation.offset, copy), operation.copyBoundaryConditions);
      affectedMembers = memberIndices.length;
      next = copied.model; nextSelection = copied.nextSelection;
      break;
    }
    case "delete": {
      batchAssert(operation.confirmDelete === true, "confirmation_required");
      const nodes: BatchNode[] = [], remap = new Map<number, number>();
      for (let index = 0; index < model.nodes.length; index += 1) {
        if (!selected.has(index)) { remap.set(index, nodes.length); nodes.push(model.nodes[index]); }
      }
      const elements: BatchModel["elements"] = [];
      for (const member of model.elements) {
        if (selected.has(member.node_i) || selected.has(member.node_j)) { affectedMembers += 1; continue; }
        const node_i = remap.get(member.node_i), node_j = remap.get(member.node_j);
        batchAssert(node_i !== undefined && node_j !== undefined, "invalid_topology");
        elements.push(node_i === member.node_i && node_j === member.node_j ? member : { ...member, node_i, node_j });
      }
      next = { ...model, nodes, elements };
      nextSelection = [];
      break;
    }
    default: throw new ModelBatchError("invalid_operation");
  }
  const inPlaceGeometry = operation.kind === "translate" || operation.kind === "align" || operation.kind === "snap" ||
    ((operation.kind === "rotate" || operation.kind === "scale" || operation.kind === "mirror") && !operation.copy);
  if (next !== model && inPlaceGeometry) {
    const memberIndices = batchMemberIndices(model, selected, "touching");
    assertValidBatchGeometry(next, memberIndices);
    affectedMembers = memberIndices.length;
  }
  return { model: next, nextSelection, summary: {
    changed: next !== model, selectedNodes: indices.length, affectedMembers,
    addedNodes: Math.max(0, next.nodes.length - model.nodes.length),
    addedMembers: Math.max(0, next.elements.length - model.elements.length),
    removedNodes: Math.max(0, model.nodes.length - next.nodes.length),
    removedMembers: Math.max(0, model.elements.length - next.elements.length),
  } };
}
