import type { ElectrostaticPlaneQuad2dResult, ElectrostaticPlaneTriangle2dResult } from "../../src/lib/api/index.ts";

export function electrostaticQuadResult(): ElectrostaticPlaneQuad2dResult {
  const input = {
    project_id: "source-project", model_version_id: "source-version",
    nodes: [[0, 0], [1, 0], [2, 0], [0, 1], [1, 1], [2, 1]].map(([x, y], index) => ({
      id: `e${index}`, x, y, fix_potential: index === 0 || index === 2,
      potential: [10, 6, 0, 10, 4, 0][index], charge_density: 0,
    })),
    elements: [[0, 1, 4, 3], [1, 2, 5, 4]].map(([node_i, node_j, node_k, node_l], index) => ({
      id: `q${index}`, node_i, node_j, node_k, node_l, permittivity: 2,
      thickness: 0.02 * (index + 1), material_id: `material-${index}`,
    })),
    materials: [0, 1].map((index) => ({ id: `material-${index}`, name: `Material ${index}`, youngs_modulus: (index + 1) * 70e9 })),
  };
  return {
    input, max_potential: 10, max_electric_field: 6, max_flux_density: 12,
    nodes: input.nodes.map((node, index) => ({ ...node, index })),
    elements: input.elements.map((element, index) => ({
      ...element, index, area: 1, average_potential: 5,
      potential_gradient_x: -(index + 1) * 2, potential_gradient_y: 0,
      electric_field_x: (index + 1) * 2, electric_field_y: 0, electric_field_magnitude: (index + 1) * 2,
      electric_flux_density_x: (index + 1) * 4, electric_flux_density_y: 0, electric_flux_density_magnitude: (index + 1) * 4,
    })),
  };
}

export function electrostaticTriangleResult(): ElectrostaticPlaneTriangle2dResult {
  const quad = electrostaticQuadResult();
  const input = {
    ...quad.input,
    elements: quad.input.elements.flatMap(({ node_l, ...element }) => [
      { ...element, id: `${element.id}a` },
      { ...element, id: `${element.id}b`, node_j: element.node_k, node_k: node_l },
    ]),
  };
  return {
    ...quad, input,
    elements: input.elements.map((element, index) => {
      const { node_l, ...response } = quad.elements[Math.floor(index / 2)];
      return { ...response, ...element, index };
    }),
  };
}
