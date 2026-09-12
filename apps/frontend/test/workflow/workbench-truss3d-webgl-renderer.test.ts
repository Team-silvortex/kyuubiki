import assert from "node:assert/strict";
import { test } from "node:test";
import { createTruss3dWebglRenderer } from "../../src/components/workbench/workbench-truss3d-webgl-renderer";
import { buildTruss3dSceneBuffers } from "../../src/components/workbench/workbench-truss3d-webgl-scene";
import { buildProjectedBounds, cameraForPreset } from "../../src/components/workbench/workbench-viewport-core";
import { modelingSceneFixture } from "../support/modeling-fixtures";
import { webglFixture } from "../support/webgl-context-fixture";

test("camera and resize frames reuse one program and do not re-upload geometry", () => {
  const { gl, calls, scissors, liveBuffers, livePrograms, liveShaders } = webglFixture();
  const args = modelingSceneFixture(2_000);
  const scene = buildTruss3dSceneBuffers(args);
  const camera = cameraForPreset("iso");
  const view = { camera, projected3d: buildProjectedBounds(args.displayTruss3dNodes, camera), projectionMode: "ortho" as const };
  const renderer = createTruss3dWebglRenderer(gl)!;
  assert.ok(renderer);
  renderer.draw(scene, view, 980, 460);
  assert.deepEqual(scissors[0], [48, 44, 884, 340]);
  const uploads = calls.bufferData;
  for (let frame = 0; frame < 60; frame += 1) {
    renderer.draw(scene, { ...view, camera: { ...camera, panX: frame } }, 980 + frame, 460);
  }
  assert.equal(calls.createProgram, 1);
  assert.equal(calls.compileShader, 2);
  assert.equal(calls.bufferData, uploads);
  assert.equal(calls.drawArrays, 122);
  assert.equal(liveShaders.size, 0);
  renderer.draw(buildTruss3dSceneBuffers({ ...args, selectedTruss3dNode: 1 }), view, 980, 460);
  assert.equal(calls.bufferData, uploads * 2, "edited scene must replace the GPU data");
  renderer.dispose();
  renderer.dispose();
  assert.equal(liveBuffers.size, 0);
  assert.equal(livePrograms.size, 0);
  const draws = calls.drawArrays;
  renderer.draw(scene, view, 980, 460);
  assert.equal(calls.drawArrays, draws);
});

test("shader, link, and partial buffer failures clean up every allocated resource", () => {
  for (const fail of [{ shader: 1 }, { shader: 2 }, { program: true }, { link: true }, { buffer: 1 }, { buffer: 5 }, { buffer: 9 }]) {
    const fixture = webglFixture(fail);
    assert.equal(createTruss3dWebglRenderer(fixture.gl), null);
    assert.equal(fixture.liveBuffers.size, 0);
    assert.equal(fixture.livePrograms.size, 0);
    assert.equal(fixture.liveShaders.size, 0);
  }
});

test("lost contexts do not receive draw calls", () => {
  const fixture = webglFixture();
  const renderer = createTruss3dWebglRenderer(fixture.gl)!;
  const args = modelingSceneFixture(2);
  const camera = cameraForPreset("iso");
  fixture.lose();
  renderer.draw(buildTruss3dSceneBuffers(args), { camera, projected3d: buildProjectedBounds(args.displayTruss3dNodes, camera), projectionMode: "ortho" }, 980, 460);
  assert.equal(fixture.calls.bufferData ?? 0, 0);
  assert.equal(fixture.calls.drawArrays ?? 0, 0);
  renderer.dispose();
  assert.equal(fixture.liveBuffers.size, 0);
});
