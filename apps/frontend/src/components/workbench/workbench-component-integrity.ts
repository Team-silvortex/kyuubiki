"use client";

import type {
  Beam1dJobInput,
  Frame2dJobInput,
  HeatBar1dJobInput,
  HeatPlaneQuad2dJobInput,
  HeatPlaneTriangle2dJobInput,
  ModelMaterial,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  Spring1dJobInput,
  Spring2dJobInput,
  Spring3dJobInput,
  ThermalBar1dJobInput,
  ThermalBeam1dJobInput,
  ThermalFrame2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
  ThermalTruss2dJobInput,
  ThermalTruss3dJobInput,
  Torsion1dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
} from "@/lib/api";
import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import type { TrussDiagnostics } from "@/components/workbench/workbench-defaults";
import type { StudyKind } from "@/components/workbench/workbench-types";

type NodeLike = { id: string; x: number; y?: number; z?: number };
type ElementLike = { id: string; node_i: number; node_j: number; node_k?: number; node_l?: number; material_id?: string };
type ModelLike = { nodes: NodeLike[]; elements: ElementLike[]; materials?: ModelMaterial[] };

type AnalyzeWorkbenchComponentIntegrityArgs = {
  beamModel: Beam1dJobInput;
  frameModel: Frame2dJobInput;
  heatBarModel: HeatBar1dJobInput;
  heatPlaneModel: HeatPlaneTriangle2dJobInput | HeatPlaneQuad2dJobInput;
  planeModel: PlaneTriangle2dJobInput | PlaneQuad2dJobInput;
  springModel: Spring1dJobInput;
  spring2dModel: Spring2dJobInput;
  spring3dModel: Spring3dJobInput;
  studyKind: StudyKind;
  thermalBarModel: ThermalBar1dJobInput;
  thermalBeamModel: ThermalBeam1dJobInput;
  thermalFrameModel: ThermalFrame2dJobInput;
  thermalPlaneModel: ThermalPlaneTriangle2dJobInput | ThermalPlaneQuad2dJobInput;
  thermalTrussModel: ThermalTruss2dJobInput;
  thermalTruss3dModel: ThermalTruss3dJobInput;
  torsionModel: Torsion1dJobInput;
  trussModel: Truss2dJobInput;
  truss3dModel: Truss3dJobInput;
};

function format(template: string, values: Record<string, string | number>) {
  return Object.entries(values).reduce(
    (text, [key, value]) => text.replaceAll(`{${key}}`, String(value)),
    template,
  );
}

function pushUnique(messages: string[], message: string) {
  if (!messages.includes(message)) messages.push(message);
}

function positiveFields(element: Record<string, unknown>) {
  return ["area", "youngs_modulus", "moment_of_inertia", "section_modulus", "shear_modulus", "polar_moment", "stiffness", "thickness", "conductivity", "permittivity", "section_depth"]
    .filter((field) => field in element)
    .map((field) => [field, element[field]] as const);
}

function triangleArea(a: NodeLike, b: NodeLike, c: NodeLike) {
  const ay = a.y ?? 0;
  const by = b.y ?? 0;
  const cy = c.y ?? 0;
  return Math.abs((a.x * (by - cy) + b.x * (cy - ay) + c.x * (ay - by)) / 2);
}

function quadArea(a: NodeLike, b: NodeLike, c: NodeLike, d: NodeLike) {
  return triangleArea(a, b, c) + triangleArea(a, c, d);
}

function lineLength(a: NodeLike, b: NodeLike) {
  return Math.hypot(a.x - b.x, (a.y ?? 0) - (b.y ?? 0), (a.z ?? 0) - (b.z ?? 0));
}

function getStudyModel(args: AnalyzeWorkbenchComponentIntegrityArgs): ModelLike | null {
  switch (args.studyKind) {
    case "truss_2d": return args.trussModel;
    case "thermal_truss_2d": return args.thermalTrussModel;
    case "truss_3d": return args.truss3dModel;
    case "thermal_truss_3d": return args.thermalTruss3dModel;
    case "frame_2d": return args.frameModel;
    case "thermal_frame_2d": return args.thermalFrameModel;
    case "beam_1d": return args.beamModel;
    case "thermal_beam_1d": return args.thermalBeamModel;
    case "torsion_1d": return args.torsionModel;
    case "spring_1d": return args.springModel;
    case "spring_2d": return args.spring2dModel;
    case "spring_3d": return args.spring3dModel;
    case "heat_bar_1d": return args.heatBarModel;
    case "thermal_bar_1d": return args.thermalBarModel;
    case "plane_triangle_2d":
    case "plane_quad_2d": return args.planeModel;
    case "thermal_plane_triangle_2d":
    case "thermal_plane_quad_2d": return args.thermalPlaneModel;
    case "heat_plane_triangle_2d":
    case "heat_plane_quad_2d": return args.heatPlaneModel;
    default: return null;
  }
}

export function analyzeWorkbenchComponentIntegrity(
  args: AnalyzeWorkbenchComponentIntegrityArgs,
  t: WorkbenchCopy,
): TrussDiagnostics | null {
  const model = getStudyModel(args);
  if (!model) return null;

  const blockingMessages: string[] = [];
  const nodeIssues: Record<number, string[]> = {};
  const suggestions: TrussDiagnostics["suggestions"] = [];

  if (model.nodes.length === 0) pushUnique(blockingMessages, t.integrityMissingNodes);
  if (model.elements.length === 0) pushUnique(blockingMessages, t.integrityMissingElements);

  const nodeIdSet = new Set<string>();
  for (const [index, node] of model.nodes.entries()) {
    if (nodeIdSet.has(node.id)) pushUnique(blockingMessages, format(t.integrityDuplicateNodeId, { id: node.id }));
    nodeIdSet.add(node.id);
    for (const field of ["x", "y", "z"] as const) {
      if (field in node && !Number.isFinite(node[field] ?? 0)) {
        const message = format(t.integrityInvalidNodeField, { id: node.id, field });
        pushUnique(blockingMessages, message);
        nodeIssues[index] = [...(nodeIssues[index] ?? []), message];
      }
    }
  }

  const materialIdSet = new Set<string>();
  for (const material of model.materials ?? []) {
    if (materialIdSet.has(material.id)) pushUnique(blockingMessages, format(t.integrityDuplicateMaterialId, { id: material.id }));
    materialIdSet.add(material.id);
  }

  const elementIdSet = new Set<string>();
  for (const element of model.elements) {
    if (elementIdSet.has(element.id)) pushUnique(blockingMessages, format(t.integrityDuplicateElementId, { id: element.id }));
    elementIdSet.add(element.id);

    const refs = [element.node_i, element.node_j, element.node_k, element.node_l].filter((value): value is number => typeof value === "number");
    if (refs.some((nodeIndex) => nodeIndex < 0 || nodeIndex >= model.nodes.length)) {
      pushUnique(blockingMessages, format(t.integrityMissingNodeReference, { id: element.id }));
      continue;
    }

    if (element.material_id && !materialIdSet.has(element.material_id)) {
      pushUnique(blockingMessages, format(t.integrityMissingMaterialReference, { id: element.id, materialId: element.material_id }));
    }

    for (const [field, value] of positiveFields(element as Record<string, unknown>)) {
      if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
        pushUnique(blockingMessages, format(t.integrityInvalidElementField, { id: element.id, field }));
      }
    }

    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    if (nodeI && nodeJ && lineLength(nodeI, nodeJ) <= 1.0e-9) {
      pushUnique(blockingMessages, format(t.integrityDegenerateLineElement, { id: element.id }));
    }

    if (typeof element.node_k === "number") {
      const nodeK = model.nodes[element.node_k];
      if (nodeI && nodeJ && nodeK && triangleArea(nodeI, nodeJ, nodeK) <= 1.0e-10) {
        pushUnique(blockingMessages, format(t.integrityDegenerateSurfaceElement, { id: element.id }));
      }
    }

    if (typeof element.node_l === "number") {
      const nodeK = typeof element.node_k === "number" ? model.nodes[element.node_k] : null;
      const nodeL = model.nodes[element.node_l];
      if (nodeI && nodeJ && nodeK && nodeL && quadArea(nodeI, nodeJ, nodeK, nodeL) <= 1.0e-10) {
        pushUnique(blockingMessages, format(t.integrityDegenerateSurfaceElement, { id: element.id }));
      }
    }
  }

  return { blockingMessages, nodeIssues, suggestions };
}

export function mergeWorkbenchDiagnostics(
  primary: TrussDiagnostics | null,
  secondary: TrussDiagnostics | null,
): TrussDiagnostics | null {
  if (!primary) return secondary;
  if (!secondary) return primary;
  const nodeIssues = { ...secondary.nodeIssues };
  for (const [nodeIndex, issues] of Object.entries(primary.nodeIssues)) {
    nodeIssues[Number(nodeIndex)] = [...new Set([...(secondary.nodeIssues[Number(nodeIndex)] ?? []), ...issues])];
  }
  return {
    blockingMessages: [...new Set([...primary.blockingMessages, ...secondary.blockingMessages])],
    nodeIssues,
    suggestions: [...primary.suggestions, ...secondary.suggestions],
  };
}
