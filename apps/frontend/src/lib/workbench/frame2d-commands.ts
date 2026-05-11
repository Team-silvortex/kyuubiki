import type { Frame2dJobInput } from "@/lib/api";

function fallbackElement(model: Frame2dJobInput) {
  return model.elements[0] ?? {
    id: "e0",
    node_i: 0,
    node_j: 1,
    area: 0.02,
    youngs_modulus: 210e9,
    moment_of_inertia: 1.0e-4,
    section_modulus: 1.0e-3,
    material_id: model.materials?.[0]?.id,
  };
}

export function updateFrame2dNode(
  model: Frame2dJobInput,
  selectedNode: number | null,
  key: keyof Frame2dJobInput["nodes"][number],
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

export function updateFrame2dElement(
  model: Frame2dJobInput,
  selectedElement: number | null,
  key: keyof Frame2dJobInput["elements"][number],
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

export function assignFrame2dElementMaterial(
  model: Frame2dJobInput,
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

export function addFrame2dNode(
  model: Frame2dJobInput,
  connectToSelected: boolean,
  selectedNode: number | null,
  round: (value: number) => number,
) {
  const anchorIndex = connectToSelected ? selectedNode : null;
  const anchor =
    anchorIndex !== null ? model.nodes[anchorIndex] : model.nodes[model.nodes.length - 1];
  const id = `f${model.nodes.length}`;
  const nextNode = {
    id,
    x: round((anchor?.x ?? 0) + 1.5),
    y: round((anchor?.y ?? 0) + (anchorIndex !== null ? 0.6 : 0.8)),
    fix_x: false,
    fix_y: false,
    fix_rz: false,
    load_x: 0,
    load_y: 0,
    moment_z: 0,
  };
  const nodes = [...model.nodes, nextNode];
  const baseElement = fallbackElement(model);
  const elements =
    anchorIndex !== null
      ? [
          ...model.elements,
          {
            id: `e${model.elements.length}`,
            node_i: anchorIndex,
            node_j: nodes.length - 1,
            area: baseElement.area,
            youngs_modulus: baseElement.youngs_modulus,
            moment_of_inertia: baseElement.moment_of_inertia,
            section_modulus: baseElement.section_modulus,
            material_id: baseElement.material_id,
          },
        ]
      : model.elements;

  return {
    model: { ...model, nodes, elements },
    nextSelectedNode: model.nodes.length,
    nextSelectedElement: connectToSelected && selectedNode !== null ? model.elements.length : null,
    createdBranch: connectToSelected && selectedNode !== null,
  };
}

export function deleteFrame2dNode(model: Frame2dJobInput, selectedNode: number | null) {
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

export function toggleFrame2dMember(model: Frame2dJobInput, memberDraftNodes: number[]) {
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

  const baseElement = fallbackElement(model);
  const next = {
    id: `e${model.elements.length}`,
    node_i: nodeI,
    node_j: nodeJ,
    area: baseElement.area,
    youngs_modulus: baseElement.youngs_modulus,
    moment_of_inertia: baseElement.moment_of_inertia,
    section_modulus: baseElement.section_modulus,
    material_id: baseElement.material_id,
  };

  return {
    model: { ...model, elements: [...model.elements, next] },
    removedExisting: false,
    nextSelectedElement: model.elements.length,
    valid: true,
  };
}

export function deleteFrame2dElement(model: Frame2dJobInput, selectedElement: number | null) {
  if (selectedElement === null) return model;
  return {
    ...model,
    elements: model.elements
      .filter((_, index) => index !== selectedElement)
      .map((element, index) => ({ ...element, id: `e${index}` })),
  };
}
