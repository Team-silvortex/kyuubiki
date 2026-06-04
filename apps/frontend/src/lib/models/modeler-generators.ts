import type {
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  Truss2dJobInput,
  TrussElementInput,
  TrussNodeInput,
} from "@/lib/api";
import type { ParametricPanelConfig, ParametricTrussConfig } from "@/lib/models/modeler-types";

export function generateRectangularPanelMesh(config: ParametricPanelConfig): PlaneTriangle2dJobInput {
  const width = Math.max(0.2, config.width);
  const height = Math.max(0.2, config.height);
  const divisionsX = Math.max(1, Math.round(config.divisionsX));
  const divisionsY = Math.max(1, Math.round(config.divisionsY));
  const thickness = Math.max(0.001, config.thickness);
  const modulus = Math.max(0.1, config.youngsModulusGpa) * 1.0e9;
  const poissonRatio = Math.min(0.49, Math.max(0.01, config.poissonRatio));
  const loadY = config.loadY;

  const nodes: PlaneTriangle2dJobInput["nodes"] = [];
  const elements: PlaneTriangle2dJobInput["elements"] = [];
  const dx = width / divisionsX;
  const dy = height / divisionsY;

  for (let row = 0; row <= divisionsY; row += 1) {
    for (let col = 0; col <= divisionsX; col += 1) {
      const index = row * (divisionsX + 1) + col;
      const onLeftEdge = col === 0;
      const onRightEdge = col === divisionsX;
      nodes.push({
        id: `p${index}`,
        x: round(col * dx),
        y: round(row * dy),
        fix_x: onLeftEdge,
        fix_y: onLeftEdge,
        load_x: 0,
        load_y: onRightEdge ? loadY / (divisionsY + 1) : 0,
      });
    }
  }

  for (let row = 0; row < divisionsY; row += 1) {
    for (let col = 0; col < divisionsX; col += 1) {
      const n0 = row * (divisionsX + 1) + col;
      const n1 = n0 + 1;
      const n2 = n0 + divisionsX + 1;
      const n3 = n2 + 1;
      const base = elements.length;
      elements.push({
        id: `pt${base}`,
        node_i: n0,
        node_j: n1,
        node_k: n3,
        thickness,
        youngs_modulus: modulus,
        poisson_ratio: poissonRatio,
      });
      elements.push({
        id: `pt${base + 1}`,
        node_i: n0,
        node_j: n3,
        node_k: n2,
        thickness,
        youngs_modulus: modulus,
        poisson_ratio: poissonRatio,
      });
    }
  }

  return { nodes, elements };
}

export function generateRectangularQuadPanelMesh(config: ParametricPanelConfig): PlaneQuad2dJobInput {
  const width = Math.max(0.2, config.width);
  const height = Math.max(0.2, config.height);
  const divisionsX = Math.max(1, Math.round(config.divisionsX));
  const divisionsY = Math.max(1, Math.round(config.divisionsY));
  const thickness = Math.max(0.001, config.thickness);
  const modulus = Math.max(0.1, config.youngsModulusGpa) * 1.0e9;
  const poissonRatio = Math.min(0.49, Math.max(0.01, config.poissonRatio));
  const loadY = config.loadY;

  const nodes: PlaneQuad2dJobInput["nodes"] = [];
  const elements: PlaneQuad2dJobInput["elements"] = [];
  const dx = width / divisionsX;
  const dy = height / divisionsY;

  for (let row = 0; row <= divisionsY; row += 1) {
    for (let col = 0; col <= divisionsX; col += 1) {
      const index = row * (divisionsX + 1) + col;
      const onLeftEdge = col === 0;
      const onRightEdge = col === divisionsX;
      nodes.push({
        id: `q${index}`,
        x: round(col * dx),
        y: round(row * dy),
        fix_x: onLeftEdge,
        fix_y: onLeftEdge,
        load_x: 0,
        load_y: onRightEdge ? loadY / (divisionsY + 1) : 0,
      });
    }
  }

  for (let row = 0; row < divisionsY; row += 1) {
    for (let col = 0; col < divisionsX; col += 1) {
      const n0 = row * (divisionsX + 1) + col;
      const n1 = n0 + 1;
      const n2 = n0 + divisionsX + 2;
      const n3 = n0 + divisionsX + 1;
      const base = elements.length;
      elements.push({
        id: `pq${base}`,
        node_i: n0,
        node_j: n1,
        node_k: n2,
        node_l: n3,
        thickness,
        youngs_modulus: modulus,
        poisson_ratio: poissonRatio,
      });
    }
  }

  return { nodes, elements };
}

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
    elements.push(member(`d${index}`, index % 2 === 0 ? index : index + 1, top, area, modulus));
  }

  return { nodes, elements };
}

function member(id: string, nodeI: number, nodeJ: number, area: number, youngsModulus: number): TrussElementInput {
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
