import type { Truss3dJobInput } from "@/lib/api";
import { applyModelBatch } from "./model-batch-commands.ts";
import { selectBatchNodes } from "./model-batch-selection.ts";

function selectedOrFocusedIndices(selected: number[], focused: number | null) {
  return selected.length > 0 ? selected : focused !== null ? [focused] : [];
}

function updateSelectedNodes(
  model: Truss3dJobInput,
  indices: number[],
  update: (node: Truss3dJobInput["nodes"][number]) => Truss3dJobInput["nodes"][number],
) {
  let nodes = model.nodes;
  for (const index of new Set(indices)) {
    if (!Number.isInteger(index) || index < 0 || index >= model.nodes.length) continue;
    const node = model.nodes[index];
    const next = update(node);
    if (next === node) continue;
    // Copy the node array once, only when an actual edit occurs. History keeps the old objects.
    if (nodes === model.nodes) nodes = model.nodes.slice();
    nodes[index] = next;
  }
  return nodes === model.nodes ? model : { ...model, nodes };
}

export function updateTruss3dSelectedNodes(
  model: Truss3dJobInput,
  selectedNodes: number[],
  selectedNode: number | null,
  key: keyof Truss3dJobInput["nodes"][number],
  value: number | boolean,
) {
  return updateSelectedNodes(model, selectedOrFocusedIndices(selectedNodes, selectedNode),
    (node) => Object.is(node[key], value) ? node : { ...node, [key]: value });
}

export function updateTruss3dNodePositionCommand(
  model: Truss3dJobInput,
  index: number,
  position: { x: number; y: number; z: number },
  round: (value: number) => number,
) {
  return updateSelectedNodes(model, [index], (node) => {
    const x = round(position.x), y = round(position.y), z = round(position.z);
    return Object.is(node.x, x) && Object.is(node.y, y) && Object.is(node.z, z)
      ? node : { ...node, x, y, z };
  });
}

export function nudgeTruss3dSelectedNodes(
  model: Truss3dJobInput,
  selectedNodes: number[],
  selectedNode: number | null,
  axis: "x" | "y" | "z",
  delta: number,
  round: (value: number) => number,
) {
  return updateSelectedNodes(model, selectedOrFocusedIndices(selectedNodes, selectedNode), (node) => {
    const value = round(node[axis] + delta);
    return Object.is(node[axis], value) ? node : { ...node, [axis]: value };
  });
}

export function applyTruss3dSelectedLoads(
  model: Truss3dJobInput,
  selectedNodes: number[],
  selectedNode: number | null,
  mode: "apply" | "clear",
  loads: { x: number; y: number; z: number },
) {
  const load_x = mode === "clear" ? 0 : loads.x;
  const load_y = mode === "clear" ? 0 : loads.y;
  const load_z = mode === "clear" ? 0 : loads.z;
  return updateSelectedNodes(model, selectedOrFocusedIndices(selectedNodes, selectedNode), (node) =>
    Object.is(node.load_x, load_x) && Object.is(node.load_y, load_y) && Object.is(node.load_z, load_z)
      ? node : { ...node, load_x, load_y, load_z });
}

export function cloneTruss3dSelectedNodes(
  model: Truss3dJobInput,
  selectedNodes: number[],
  selectedNode: number | null,
  mirrorAxis: "x" | "y" | "z" | null = null,
) {
  const indices = selectBatchNodes(model, { kind: "current" }, selectedOrFocusedIndices(selectedNodes, selectedNode));
  if (indices.length === 0) return { model, nextSelection: [] as number[] };
  // Keep quick-duplicate's boundary policy, but share IDs, topology and precision guards with PWDT.
  return applyModelBatch(model, { query: { kind: "indices", indices }, operation: mirrorAxis
    ? { kind: "mirror", axis: mirrorAxis, pivot: { kind: "selection" }, copy: true, copyBoundaryConditions: true }
    : { kind: "array", offset: { x: 0.4, y: 0.2, z: 0.4 }, copies: 1, copyBoundaryConditions: true } });
}

export function updateTruss3dElement(
  model: Truss3dJobInput,
  selectedElement: number | null,
  key: keyof Truss3dJobInput["elements"][number],
  value: number,
) {
  if (selectedElement === null) return model;
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      index === selectedElement ? { ...element, [key]: value } : element,
    ),
  };
}

export function assignTruss3dElementMaterial(
  model: Truss3dJobInput,
  selectedElement: number | null,
  materialId: string,
) {
  if (selectedElement === null) return model;
  const material = model.materials?.find((entry) => entry.id === materialId);
  return {
    ...model,
    elements: model.elements.map((element, index) =>
      index === selectedElement
        ? {
            ...element,
            material_id: materialId,
            youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
          }
        : element,
    ),
  };
}

export function addTruss3dNodeCommand(
  model: Truss3dJobInput,
  connectToSelected: boolean,
  selectedNode: number | null,
  round: (value: number) => number,
) {
  const anchorIndex = connectToSelected ? selectedNode : null;
  const anchor =
    anchorIndex !== null ? model.nodes[anchorIndex] : model.nodes[model.nodes.length - 1];
  const id = `n${model.nodes.length}`;
  const nextNode = {
    id,
    x: round((anchor?.x ?? 0) + 0.8),
    y: round((anchor?.y ?? 0) + (connectToSelected ? 0.35 : 0.6)),
    z: round((anchor?.z ?? 0) + 0.45),
    fix_x: false,
    fix_y: false,
    fix_z: false,
    load_x: 0,
    load_y: 0,
    load_z: 0,
  };
  const nodes = [...model.nodes, nextNode];
  const elements =
    anchorIndex !== null
      ? (() => {
          const material = model.materials?.[0];
          return [
            ...model.elements,
            {
              id: `e${model.elements.length}`,
              node_i: anchorIndex,
              node_j: nodes.length - 1,
              area: model.elements[0]?.area ?? 0.01,
              youngs_modulus: material?.youngs_modulus ?? model.elements[0]?.youngs_modulus ?? 70e9,
              material_id: material?.id,
            },
          ];
        })()
      : model.elements;

  return {
    model: { ...model, nodes, elements },
    nextSelectedNode: model.nodes.length,
    nextSelectedElement: connectToSelected && selectedNode !== null ? model.elements.length : null,
    createdBranch: connectToSelected && selectedNode !== null,
  };
}

export function deleteTruss3dNodeCommand(model: Truss3dJobInput, selectedNode: number | null) {
  if (selectedNode === null) return model;
  const nodes = model.nodes.filter((_, index) => index !== selectedNode);
  const elements = model.elements
    .filter((element) => element.node_i !== selectedNode && element.node_j !== selectedNode)
    .map((element, index) => ({
      ...element,
      id: `e${index}`,
      node_i: element.node_i > selectedNode ? element.node_i - 1 : element.node_i,
      node_j: element.node_j > selectedNode ? element.node_j - 1 : element.node_j,
    }));
  return { ...model, nodes, elements };
}

export function completeTruss3dLinkCommand(
  model: Truss3dJobInput,
  firstNode: number,
  secondNode: number,
) {
  if (firstNode === secondNode) {
    return {
      model,
      removedExisting: false,
      nextSelectedElement: null,
      repeatedNode: true,
    };
  }

  const existingIndex = model.elements.findIndex(
    (element) =>
      (element.node_i === firstNode && element.node_j === secondNode) ||
      (element.node_i === secondNode && element.node_j === firstNode),
  );

  if (existingIndex >= 0) {
    return {
      model: {
        ...model,
        elements: model.elements
          .filter((_, index) => index !== existingIndex)
          .map((element, index) => ({ ...element, id: `e${index}` })),
      },
      removedExisting: true,
      nextSelectedElement: null,
      repeatedNode: false,
    };
  }

  const next = {
    id: `e${model.elements.length}`,
    node_i: firstNode,
    node_j: secondNode,
    area: model.elements[0]?.area ?? 0.01,
    youngs_modulus: model.materials?.[0]?.youngs_modulus ?? model.elements[0]?.youngs_modulus ?? 70e9,
    material_id: model.materials?.[0]?.id,
  };

  return {
    model: { ...model, elements: [...model.elements, next] },
    removedExisting: false,
    nextSelectedElement: model.elements.length,
    repeatedNode: false,
  };
}

export function deleteTruss3dElementCommand(model: Truss3dJobInput, selectedElement: number | null) {
  if (selectedElement === null) return model;
  return {
    ...model,
    elements: model.elements
      .filter((_, index) => index !== selectedElement)
      .map((element, index) => ({ ...element, id: `e${index}` })),
  };
}

export function merge3dBoxSelection(currentSelection: number[], indices: number[], append: boolean) {
  return append ? Array.from(new Set([...currentSelection, ...indices])) : indices;
}
