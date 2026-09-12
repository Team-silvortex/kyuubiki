import { performance } from "node:perf_hooks";
import { getTrussBounds, findNearestConnectableNode } from "../../src/lib/workbench/model-geometry";
import {
  updateTruss3dSelectedNodes, nudgeTruss3dSelectedNodes, applyTruss3dSelectedLoads,
} from "../../src/lib/workbench/truss3d-commands";
import { buildProjectedBounds, cameraForPreset } from "../../src/components/workbench/workbench-viewport-core";
import {
  buildTruss3dSceneBuffers, computeTruss3dSceneBounds, resolveDeformationScale,
} from "../../src/components/workbench/workbench-truss3d-webgl-scene";
import { modelingFixture, modelingSceneFixture } from "../support/modeling-fixtures";

// Opt-in CPU microbenchmark; no solver, GPU driver, or end-to-end frame-rate claim.
const round = (value: number) => Math.round(value * 1000) / 1000;
const model = modelingFixture(20_000);
const selected = model.nodes.filter((_, index) => index % 2 === 0).map((_, index) => index * 2);
const nearestModel = modelingFixture(10_000);
const scene = modelingSceneFixture(20_000);
const largeNodes = modelingSceneFixture(200_000).displayTruss3dNodes;
const camera = cameraForPreset("iso");
const cases: Array<{ name: string; nodes: number; run: () => unknown }> = [
  { name: "batch-property-10k-selected", nodes: model.nodes.length, run: () => updateTruss3dSelectedNodes(model, selected, null, "fix_x", true) },
  { name: "batch-nudge-10k-selected", nodes: model.nodes.length, run: () => nudgeTruss3dSelectedNodes(model, selected, null, "x", 0.1, round) },
  { name: "batch-load-10k-selected", nodes: model.nodes.length, run: () => applyTruss3dSelectedLoads(model, selected, null, "apply", { x: 1, y: 2, z: 3 }) },
  { name: "nearest-connectable", nodes: nearestModel.nodes.length, run: () => findNearestConnectableNode(nearestModel, 0) },
  { name: "scene-buffer-build", nodes: scene.displayTruss3dNodes.length, run: () => buildTruss3dSceneBuffers(scene) },
  { name: "planar-bounds", nodes: largeNodes.length, run: () => getTrussBounds(largeNodes) },
  { name: "projected-bounds", nodes: largeNodes.length, run: () => buildProjectedBounds(largeNodes, camera) },
  { name: "spatial-bounds", nodes: largeNodes.length, run: () => computeTruss3dSceneBounds(largeNodes) },
  { name: "deformation-scale", nodes: largeNodes.length, run: () => resolveDeformationScale(largeNodes, true) },
];

const results = cases.map(({ name, nodes, run }) => {
  try {
    for (let warmup = 0; warmup < 2; warmup += 1) run();
    const samples = Array.from({ length: 7 }, () => {
      const start = performance.now();
      run();
      return performance.now() - start;
    }).sort((a, b) => a - b);
    return { name, nodes, status: "pass", median_ms: round(samples[3]), p95_ms: round(samples[6]) };
  } catch (error) {
    return { name, nodes, status: "error", error: error instanceof Error ? error.message : String(error) };
  }
});

console.log(JSON.stringify({
  schema: "kyuubiki.workbench-modeling-benchmark/v1", generated_at: new Date().toISOString(),
  runtime: process.version, platform: `${process.platform}-${process.arch}`,
  warmup: 2, samples: 7, scope: "CPU geometry editing and scene preparation; no browser or GPU", results,
}, null, 2));
if (results.some((result) => result.status !== "pass")) process.exitCode = 1;
