import { batchAssert, type BatchModel, type BatchNode } from "./model-batch-selection.ts";
import { assertValidBatchGeometry } from "./model-batch-geometry.ts";

export const MODEL_BATCH_LIMITS = { copies: 1000, addedNodes: 100_000, addedMembers: 200_000 } as const;

function uniqueIdFactory(items: Array<{ id: string }>, prefix: string) {
  const used = new Set(items.map((item) => item.id));
  let cursor = items.length;
  return () => {
    while (used.has(`${prefix}${cursor}`)) cursor += 1;
    const id = `${prefix}${cursor++}`;
    used.add(id);
    return id;
  };
}

export function copyBatchGeometry<T extends BatchModel>(
  model: T, indices: number[], memberIndices: number[], copies: number,
  transform: (node: BatchNode, copy: number) => BatchNode, copyBoundaryConditions?: boolean,
) {
  batchAssert(Number.isInteger(copies) && copies > 0 && copies <= MODEL_BATCH_LIMITS.copies, "invalid_copies");
  batchAssert(copyBoundaryConditions === undefined || typeof copyBoundaryConditions === "boolean", "invalid_operation");
  batchAssert(indices.length * copies <= MODEL_BATCH_LIMITS.addedNodes &&
    memberIndices.length * copies <= MODEL_BATCH_LIMITS.addedMembers, "array_limit");
  assertValidBatchGeometry(model, memberIndices);
  const nodeId = uniqueIdFactory(model.nodes, "n"), memberId = uniqueIdFactory(model.elements, "e");
  const nodes = model.nodes.slice(), elements = model.elements.slice(), nextSelection: number[] = [];
  for (let copy = 1; copy <= copies; copy += 1) {
    const remap = new Map<number, number>();
    for (const index of indices) {
      remap.set(index, nodes.length);
      nextSelection.push(nodes.length);
      const node = { ...transform(model.nodes[index], copy), id: nodeId() };
      if (!copyBoundaryConditions) {
        node.fix_x = node.fix_y = false;
        node.load_x = node.load_y = 0;
        if (node.z !== undefined) { node.fix_z = false; node.load_z = 0; }
        if (node.fix_rz !== undefined) { node.fix_rz = false; node.moment_z = 0; }
      }
      nodes.push(node);
    }
    const firstMember = elements.length;
    for (const index of memberIndices) {
      const member = model.elements[index];
      elements.push({ ...member, id: memberId(), node_i: remap.get(member.node_i)!, node_j: remap.get(member.node_j)!,
        ...(!copyBoundaryConditions && member.distributed_load_y !== undefined ? { distributed_load_y: 0 } : {}) });
    }
    assertValidBatchGeometry({ nodes, elements }, Array.from({ length: elements.length - firstMember }, (_, index) => firstMember + index));
  }
  return { model: { ...model, nodes, elements } as T, nextSelection };
}
