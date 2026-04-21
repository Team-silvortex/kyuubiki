import type { Truss3dJobInput } from "@/lib/api";

function selectedOrFocusedIndices(selected: number[], focused: number | null) {
  return selected.length > 0 ? selected : focused !== null ? [focused] : [];
}

export function updateTruss3dSelectedNodes(
  model: Truss3dJobInput,
  selectedNodes: number[],
  selectedNode: number | null,
  key: keyof Truss3dJobInput["nodes"][number],
  value: number | boolean,
) {
  const targetIndices = selectedOrFocusedIndices(selectedNodes, selectedNode);
  if (targetIndices.length === 0) return model;
  return {
    ...model,
    nodes: model.nodes.map((node, index) =>
      targetIndices.includes(index) ? { ...node, [key]: value } : node,
    ),
  };
}

export function updateTruss3dNodePositionCommand(
  model: Truss3dJobInput,
  index: number,
  position: { x: number; y: number; z: number },
  round: (value: number) => number,
) {
  return {
    ...model,
    nodes: model.nodes.map((node, nodeIndex) =>
      nodeIndex === index
        ? { ...node, x: round(position.x), y: round(position.y), z: round(position.z) }
        : node,
    ),
  };
}

export function nudgeTruss3dSelectedNodes(
  model: Truss3dJobInput,
  selectedNodes: number[],
  selectedNode: number | null,
  axis: "x" | "y" | "z",
  delta: number,
  round: (value: number) => number,
) {
  const targetIndices = selectedOrFocusedIndices(selectedNodes, selectedNode);
  if (targetIndices.length === 0) return model;
  return {
    ...model,
    nodes: model.nodes.map((node, index) =>
      targetIndices.includes(index) ? { ...node, [axis]: round(node[axis] + delta) } : node,
    ),
  };
}

export function applyTruss3dSelectedLoads(
  model: Truss3dJobInput,
  selectedNodes: number[],
  selectedNode: number | null,
  mode: "apply" | "clear",
  loads: { x: number; y: number; z: number },
) {
  const targetIndices = selectedOrFocusedIndices(selectedNodes, selectedNode);
  if (targetIndices.length === 0) return model;
  return {
    ...model,
    nodes: model.nodes.map((node, index) =>
      targetIndices.includes(index)
        ? {
            ...node,
            load_x: mode === "clear" ? 0 : loads.x,
            load_y: mode === "clear" ? 0 : loads.y,
            load_z: mode === "clear" ? 0 : loads.z,
          }
        : node,
    ),
  };
}

export function cloneTruss3dSelectedNodes(
  model: Truss3dJobInput,
  selectedNodes: number[],
  selectedNode: number | null,
  round: (value: number) => number,
  mirrorAxis: "x" | "y" | "z" | null = null,
) {
  const targetIndices = selectedOrFocusedIndices(selectedNodes, selectedNode);
  if (targetIndices.length === 0) {
    return { model, nextSelection: [] as number[] };
  }

  const sourceNodes = targetIndices
    .map((index) => ({ index, node: model.nodes[index] }))
    .filter((entry) => Boolean(entry.node));
  if (sourceNodes.length === 0) {
    return { model, nextSelection: [] as number[] };
  }

  const center = sourceNodes.reduce(
    (acc, entry) => ({
      x: acc.x + entry.node.x,
      y: acc.y + entry.node.y,
      z: acc.z + entry.node.z,
    }),
    { x: 0, y: 0, z: 0 },
  );
  const pivot = {
    x: center.x / sourceNodes.length,
    y: center.y / sourceNodes.length,
    z: center.z / sourceNodes.length,
  };

  const indexMap = new Map<number, number>();
  const duplicatedNodes = sourceNodes.map((entry, offset) => {
    const nextIndex = model.nodes.length + offset;
    indexMap.set(entry.index, nextIndex);
    const baseNode = { ...entry.node, id: `n${nextIndex}` };

    if (mirrorAxis) {
      return {
        ...baseNode,
        [mirrorAxis]: round(pivot[mirrorAxis] - (entry.node[mirrorAxis] - pivot[mirrorAxis])),
      };
    }

    return {
      ...baseNode,
      x: round(entry.node.x + 0.4),
      y: round(entry.node.y + 0.2),
      z: round(entry.node.z + 0.4),
    };
  });

  const duplicatedElements = model.elements.flatMap((element, offset) => {
    const mappedI = indexMap.get(element.node_i);
    const mappedJ = indexMap.get(element.node_j);
    if (mappedI === undefined || mappedJ === undefined) return [];
    return [
      {
        ...element,
        id: `e${model.elements.length + offset}`,
        node_i: mappedI,
        node_j: mappedJ,
      },
    ];
  });

  return {
    model: {
      ...model,
      nodes: [...model.nodes, ...duplicatedNodes],
      elements: [...model.elements, ...duplicatedElements].map((element, index) => ({ ...element, id: `e${index}` })),
    },
    nextSelection: duplicatedNodes.map((_, offset) => model.nodes.length + offset),
  };
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
