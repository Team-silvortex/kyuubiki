import type { Truss2dJobInput } from "@/lib/api";
import type { ParametricTrussConfig } from "@/lib/models";

export function updateTruss2dNode(
  model: Truss2dJobInput,
  selectedNode: number | null,
  key: keyof Truss2dJobInput["nodes"][number],
  value: number | boolean,
) {
  if (selectedNode === null) return model;
  return {
    ...model,
    nodes: model.nodes.map((node, index) =>
      index === selectedNode ? { ...node, [key]: value } : node,
    ),
  };
}

export function updateTruss2dElement(
  model: Truss2dJobInput,
  selectedElement: number | null,
  key: keyof Truss2dJobInput["elements"][number],
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

export function assignTruss2dElementMaterial(
  model: Truss2dJobInput,
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

export function addTruss2dNode(
  model: Truss2dJobInput,
  connectToSelected: boolean,
  selectedNode: number | null,
  parametric: ParametricTrussConfig,
  round: (value: number) => number,
) {
  const anchorIndex = connectToSelected ? selectedNode : null;
  const anchor =
    anchorIndex !== null ? model.nodes[anchorIndex] : model.nodes[model.nodes.length - 1];
  const id = `n${model.nodes.length}`;
  const nextNode = {
    id,
    x: round((anchor?.x ?? 0) + 1),
    y: round((anchor?.y ?? 0) + (anchorIndex !== null ? 0.4 : 0.5)),
    fix_x: false,
    fix_y: false,
    load_x: 0,
    load_y: 0,
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
              area: parametric.area,
              youngs_modulus: material?.youngs_modulus ?? parametric.youngsModulusGpa * 1.0e9,
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

export function deleteTruss2dNode(model: Truss2dJobInput, selectedNode: number | null) {
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

export function toggleDraftSelection(current: number[], index: number) {
  if (current.includes(index)) {
    return current.filter((value) => value !== index);
  }
  return [...current, index].slice(-2);
}

export function toggleTruss2dMember(
  model: Truss2dJobInput,
  memberDraftNodes: number[],
  parametric: ParametricTrussConfig,
) {
  if (memberDraftNodes.length !== 2) {
    return { model, removedExisting: false, nextSelectedElement: null, valid: false };
  }

  const [nodeI, nodeJ] = memberDraftNodes;
  if (nodeI === nodeJ) {
    return { model, removedExisting: false, nextSelectedElement: null, valid: false };
  }

  const existingIndex = model.elements.findIndex(
    (element) =>
      (element.node_i === nodeI && element.node_j === nodeJ) ||
      (element.node_i === nodeJ && element.node_j === nodeI),
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
      valid: true,
    };
  }

  const next = {
    id: `e${model.elements.length}`,
    node_i: nodeI,
    node_j: nodeJ,
    area: parametric.area,
    youngs_modulus: model.materials?.[0]?.youngs_modulus ?? parametric.youngsModulusGpa * 1.0e9,
    material_id: model.materials?.[0]?.id,
  };

  return {
    model: { ...model, elements: [...model.elements, next] },
    removedExisting: false,
    nextSelectedElement: model.elements.length,
    valid: true,
  };
}

export function deleteTruss2dElement(model: Truss2dJobInput, selectedElement: number | null) {
  if (selectedElement === null) return model;
  return {
    ...model,
    elements: model.elements
      .filter((_, index) => index !== selectedElement)
      .map((element, index) => ({ ...element, id: `e${index}` })),
  };
}
