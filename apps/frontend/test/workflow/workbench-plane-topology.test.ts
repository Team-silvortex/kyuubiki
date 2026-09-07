import assert from "node:assert/strict";
import { test } from "node:test";
import { buildPlaneNodeIndex, findPlaneItemByIndex, resolvePlaneElementNodes } from "../../src/components/workbench/workbench-plane-topology";

const nodes = [0, 1, 2, 3].map((index) => ({ index, x: index % 2, y: Math.floor(index / 2) }));
const triangle = { node_i: 0, node_j: 1, node_k: 2 };
const quad = { ...triangle, node_l: 3 };

test("plane selection resolves dense, sparse, and reordered records by identity", () => {
  assert.equal(findPlaneItemByIndex(nodes, 2), nodes[2]);
  assert.equal(findPlaneItemByIndex([...nodes].reverse(), 2), nodes[2]);
  assert.equal(findPlaneItemByIndex(nodes.slice(2), 2), nodes[2]);
  for (const index of [null, -1, 0.5, 0, 100]) assert.equal(findPlaneItemByIndex(nodes.slice(2), index), null);
});

test("plane connectivity follows node indices, not response array order", () => {
  const lookup = buildPlaneNodeIndex([...nodes].reverse());
  assert.deepEqual(resolvePlaneElementNodes(triangle, lookup), nodes.slice(0, 3));
  assert.deepEqual(resolvePlaneElementNodes(quad, lookup), nodes);
});

test("sparse plane results skip only elements whose nodes are missing", () => {
  const lookup = buildPlaneNodeIndex(nodes.slice(0, 3));
  assert.deepEqual(resolvePlaneElementNodes(triangle, lookup), nodes.slice(0, 3));
  assert.equal(resolvePlaneElementNodes(quad, lookup), null);
});

test("ambiguous plane node identities cannot render an arbitrary copy", () => {
  const lookup = buildPlaneNodeIndex([...nodes, nodes[0], { ...nodes[0], x: 2 }]);
  assert.equal(lookup.has(0), false);
  assert.equal(resolvePlaneElementNodes(triangle, lookup), null);
});

test("non-finite coordinates and invalid node indices do not enter the viewport", () => {
  for (const invalid of [{ x: NaN }, { y: Infinity }, { index: -1 }, { index: 0.5 }]) {
    const lookup = buildPlaneNodeIndex([{ ...nodes[0], ...invalid }, ...nodes.slice(1)]);
    assert.equal(resolvePlaneElementNodes(triangle, lookup), null);
  }
  assert.equal(buildPlaneNodeIndex([{ ...nodes[0], x: NaN }, ...nodes]).has(0), false);
});

test("invalid or repeated element references are not drawn as valid polygons", () => {
  const lookup = buildPlaneNodeIndex(nodes);
  for (const index of [-1, 0.5, 100, NaN, 1]) {
    assert.equal(resolvePlaneElementNodes({ ...quad, node_l: index }, lookup), null);
  }
});
