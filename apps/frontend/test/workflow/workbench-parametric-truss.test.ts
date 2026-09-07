import assert from "node:assert/strict";
import test from "node:test";
import { generatePrattTruss } from "../../src/lib/models/modeler-generators.ts";

function model(bays: number) {
  return generatePrattTruss({
    bays, span: 12, height: 2, area: 0.01, youngsModulusGpa: 210, loadY: -800,
  });
}

for (const bays of [2, 3, 4, 6, 8, 15, 32]) {
  test(`parametric truss with ${bays} bays has unique members and triangulates every bay`, () => {
    const mesh = model(bays);
    assert.equal(mesh.nodes.length, 2 * bays + 1);
    assert.equal(mesh.elements.length, 4 * bays - 1);
    const edges = new Set(mesh.elements.map(({ node_i, node_j }) =>
      [node_i, node_j].sort((a, b) => a - b).join(":"),
    ));
    assert.equal(edges.size, mesh.elements.length, "duplicated members leave missing restraints");
    for (let bay = 0; bay < bays; bay += 1) {
      const top = bays + 1 + bay;
      assert.ok(edges.has(`${bay}:${top}`));
      assert.ok(edges.has(`${bay + 1}:${top}`));
    }
    assert.equal(mesh.nodes.reduce((sum, node) => sum + (node.load_y ?? 0), 0), -800);
  });

  test(`parametric truss with ${bays} bays has positive-definite constrained stiffness`, () => {
    const mesh = model(bays);
    const free = mesh.nodes.flatMap((node, index) => [
      ...(!node.fix_x ? [2 * index] : []),
      ...(!node.fix_y ? [2 * index + 1] : []),
    ]);
    const positions = new Map(free.map((dof, index) => [dof, index]));
    const stiffness = Array.from({ length: free.length }, () => Array(free.length).fill(0));
    // Independently assemble axial element stiffness, then test for unconstrained mechanisms.
    for (const element of mesh.elements) {
      const a = mesh.nodes[element.node_i];
      const b = mesh.nodes[element.node_j];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const length = Math.hypot(dx, dy);
      const directions = [-dx / length, -dy / length, dx / length, dy / length];
      const dofs = [2 * element.node_i, 2 * element.node_i + 1, 2 * element.node_j, 2 * element.node_j + 1];
      const scale = element.youngs_modulus * element.area / length;
      for (let i = 0; i < 4; i += 1) {
        const row = positions.get(dofs[i]);
        if (row === undefined) continue;
        for (let j = 0; j < 4; j += 1) {
          const col = positions.get(dofs[j]);
          if (col !== undefined) stiffness[row][col] += scale * directions[i] * directions[j];
        }
      }
    }
    const tolerance = Math.max(...stiffness.map((row, index) => row[index])) * 1e-12;
    const lower = Array.from({ length: free.length }, () => Array(free.length).fill(0));
    for (let row = 0; row < free.length; row += 1) {
      for (let col = 0; col <= row; col += 1) {
        let value = stiffness[row][col];
        for (let k = 0; k < col; k += 1) value -= lower[row][k] * lower[col][k];
        if (row === col) {
          assert.ok(Number.isFinite(value) && value > tolerance, `unstable free DOF ${free[row]}`);
          lower[row][col] = Math.sqrt(value);
        } else lower[row][col] = value / lower[col][col];
      }
    }
  });
}
