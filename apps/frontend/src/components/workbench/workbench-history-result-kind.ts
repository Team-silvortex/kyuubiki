import type { WorkbenchStudyKind } from "@/lib/workbench/history";

type RecordValue = Record<string, unknown>;
const isRecord = (value: unknown): value is RecordValue =>
  typeof value === "object" && value !== null && !Array.isArray(value);
const finite = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);
const INDEX_KEYS = ["node_i", "node_j", "node_k", "node_l"] as const;

export function invalidHistoryResult(reason: string): never {
  throw new Error(`HISTORY_RESULT_INVALID: ${reason}`);
}

function elementKind(element: RecordValue, dimension: number, arity: number): WorkbenchStudyKind {
  const fields = ["conductivity", "permittivity", "stiffness", "shear_modulus", "youngs_modulus"];
  const physics = fields.filter((key) => key in element);
  if (physics.length !== 1 || !finite(element[physics[0]])) invalidHistoryResult("Ambiguous element physics.");
  const plane = dimension === 2 && (arity === 3 || arity === 4);
  const suffix = arity === 4 ? "quad_2d" : "triangle_2d";
  if (physics[0] === "conductivity") {
    if (plane) return `heat_plane_${suffix}`;
    if (dimension === 1 && arity === 2) return "heat_bar_1d";
  }
  if (physics[0] === "permittivity" && plane) return `electrostatic_plane_${suffix}`;
  if (physics[0] === "stiffness" && arity === 2) return `spring_${dimension}d` as WorkbenchStudyKind;
  if (physics[0] === "shear_modulus" && dimension === 1 && arity === 2) return "torsion_1d";
  if (physics[0] === "youngs_modulus") {
    const thermal = "thermal_expansion" in element;
    if (thermal && !finite(element.thermal_expansion)) invalidHistoryResult("Invalid thermal expansion.");
    if (plane) return thermal ? `thermal_plane_${suffix}` : `plane_${suffix}`;
    if (arity === 2 && "moment_of_inertia" in element) {
      if (dimension === 1) return thermal ? "thermal_beam_1d" : "beam_1d";
      if (dimension === 2) return thermal ? "thermal_frame_2d" : "frame_2d";
    } else if (arity === 2) {
      if (dimension === 1 && thermal) return "thermal_bar_1d";
      if (dimension === 2) return thermal ? "thermal_truss_2d" : "truss_2d";
      if (dimension === 3) return thermal ? "thermal_truss_3d" : "truss_3d";
    }
  }
  return invalidHistoryResult("This result has no supported Workbench study representation.");
}

// Archived jobs do not carry a study discriminator. Require consistent input topology
// and constitutive fields instead of letting broad result-summary guards overlap.
export function resolveHistoryResultStudyKind(value: unknown): WorkbenchStudyKind {
  if (!isRecord(value) || !isRecord(value.input)) invalidHistoryResult("A solved input is required.");
  const input = value.input;
  if (finite(input.elements)) {
    if (!Number.isInteger(input.elements) || input.elements < 1 ||
      ![input.length, input.area, input.youngs_modulus].every((entry) => finite(entry) && entry > 0) ||
      !finite(input.tip_force) || !finite(value.tip_displacement)) invalidHistoryResult("Invalid axial result.");
    return "axial_bar_1d";
  }
  if (!Array.isArray(input.nodes) || !input.nodes.length || !Array.isArray(input.elements) || !input.elements.length ||
    !Array.isArray(value.nodes) || !Array.isArray(value.elements)) invalidHistoryResult("Mesh input and result arrays are required.");
  const nodes = input.nodes;
  if (!value.nodes.every(isRecord) || !value.elements.every(isRecord)) invalidHistoryResult("Invalid result node or element record.");
  let dimension = 0;
  const nodeIds = new Set<string>();
  for (const node of nodes) {
    if (!isRecord(node) || !finite(node.x) || typeof node.id !== "string" || !node.id.trim() || nodeIds.has(node.id)) {
      invalidHistoryResult("Invalid node record or identity.");
    }
    const nextDimension = "z" in node ? 3 : "y" in node ? 2 : 1;
    if ((nextDimension >= 2 && !finite(node.y)) || (nextDimension === 3 && !finite(node.z)) ||
      (dimension !== 0 && dimension !== nextDimension)) invalidHistoryResult("Mixed or invalid node dimensions.");
    dimension = nextDimension;
    nodeIds.add(node.id);
  }
  let kind: WorkbenchStudyKind | null = null;
  let arity = 0;
  const elementIds = new Set<string>();
  for (const element of input.elements) {
    if (!isRecord(element) || typeof element.id !== "string" || !element.id.trim() || elementIds.has(element.id)) {
      invalidHistoryResult("Invalid element record or identity.");
    }
    const keys = INDEX_KEYS.filter((key) => key in element);
    const indices = keys.map((key) => element[key]);
    if (keys.length < 2 || keys.some((key, index) => key !== INDEX_KEYS[index]) ||
      indices.some((index) => !finite(index) || !Number.isInteger(index) || index < 0 || index >= nodes.length) ||
      new Set(indices).size !== indices.length || (arity !== 0 && arity !== keys.length)) invalidHistoryResult("Mixed or invalid element connectivity.");
    arity = keys.length;
    const nextKind = elementKind(element, dimension, arity);
    if (kind !== null && kind !== nextKind) invalidHistoryResult("Mixed study types in one result.");
    kind = nextKind;
    elementIds.add(element.id);
  }
  if (!kind) invalidHistoryResult("The result study could not be determined.");
  const summaryField = kind.startsWith("heat_") ? "max_temperature"
    : kind.startsWith("electrostatic_") ? "max_potential"
      : kind.startsWith("spring_") ? "max_force" : "max_stress";
  if (!finite(value[summaryField])) invalidHistoryResult("The result summary does not match its input physics.");
  return kind;
}
