import type {
  AxialBarJobInput,
  PlaneTriangle2dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
  TrussElementInput,
  TrussNodeInput,
} from "@/lib/api";

export type ParametricTrussConfig = {
  bays: number;
  span: number;
  height: number;
  area: number;
  youngsModulusGpa: number;
  loadY: number;
};

const MODEL_SCHEMA_VERSION = "kyuubiki.model/v1";

export function generatePrattTruss(config: ParametricTrussConfig): Truss2dJobInput {
  const bays = Math.max(2, Math.round(config.bays));
  const span = Math.max(1, config.span);
  const height = Math.max(0.2, config.height);
  const bayWidth = span / bays;
  const modulus = Math.max(0.1, config.youngsModulusGpa) * 1.0e9;
  const area = Math.max(1.0e-5, config.area);
  const loadY = config.loadY;

  const nodes: TrussNodeInput[] = [];
  const elements: TrussElementInput[] = [];

  for (let index = 0; index <= bays; index += 1) {
    nodes.push({
      id: `b${index}`,
      x: round(index * bayWidth),
      y: 0,
      fix_x: index === 0,
      fix_y: index === 0 || index === bays,
      load_x: 0,
      load_y: 0,
    });
  }

  for (let index = 0; index < bays; index += 1) {
    nodes.push({
      id: `t${index}`,
      x: round(index * bayWidth + bayWidth / 2),
      y: round(height),
      fix_x: false,
      fix_y: false,
      load_x: 0,
      load_y: index === Math.floor((bays - 1) / 2) ? loadY : 0,
    });
  }

  for (let index = 0; index < bays; index += 1) {
    elements.push(member(`bb${index}`, index, index + 1, area, modulus));
  }

  for (let index = 0; index < bays - 1; index += 1) {
    elements.push(member(`tt${index}`, bays + 1 + index, bays + 2 + index, area, modulus));
  }

  for (let index = 0; index < bays; index += 1) {
    const top = bays + 1 + index;
    elements.push(member(`v${index}`, index + 1, top, area, modulus));

    if (index % 2 === 0) {
      elements.push(member(`d${index}`, index, top, area, modulus));
    } else {
      elements.push(member(`d${index}`, index + 1, top, area, modulus));
    }
  }

  return { nodes, elements };
}

export function exportStudyModel(
  kind: "axial_bar_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d",
  payload: {
    name: string;
    material: string;
    youngsModulusGpa: number;
    axial?: AxialBarJobInput;
    truss?: Truss2dJobInput;
    truss3d?: Truss3dJobInput;
    plane?: PlaneTriangle2dJobInput;
  },
): string {
  if (kind === "plane_triangle_2d" && payload.plane) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        nodes: payload.plane.nodes,
        elements: payload.plane.elements,
      },
      null,
      2,
    );
  }

  if (kind === "truss_2d" && payload.truss) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        nodes: payload.truss.nodes,
        elements: payload.truss.elements,
      },
      null,
      2,
    );
  }

  if (kind === "truss_3d" && payload.truss3d) {
    return JSON.stringify(
      {
        kind,
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        youngs_modulus_gpa: payload.youngsModulusGpa,
        nodes: payload.truss3d.nodes,
        elements: payload.truss3d.elements,
      },
      null,
      2,
    );
  }

  if (payload.axial) {
    return JSON.stringify(
      {
        kind: "axial_bar_1d",
        model_schema_version: MODEL_SCHEMA_VERSION,
        name: payload.name,
        material: payload.material,
        length: payload.axial.length,
        area: payload.axial.area,
        elements: payload.axial.elements,
        tip_force: payload.axial.tip_force,
        youngs_modulus_gpa: payload.youngsModulusGpa,
      },
      null,
      2,
    );
  }

  return JSON.stringify({}, null, 2);
}

function member(
  id: string,
  nodeI: number,
  nodeJ: number,
  area: number,
  youngsModulus: number,
): TrussElementInput {
  return {
    id,
    node_i: nodeI,
    node_j: nodeJ,
    area,
    youngs_modulus: youngsModulus,
  };
}

function round(value: number): number {
  return Math.round(value * 1000) / 1000;
}
