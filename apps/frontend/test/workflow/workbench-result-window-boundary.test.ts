import test from "node:test";
import assert from "node:assert/strict";

import { resolveResultWindowStudyKind } from "../../src/components/workbench/workbench-result-window-kind.ts";
import {
  chunkCacheKey,
  clampChunkOffset,
  computeResultWindowSize,
  computeVisibleResultWindowOffset,
  RESULT_WINDOW_BASE_SIZE,
  RESULT_WINDOW_CACHE_LIMIT,
  writeChunkCache,
} from "../../src/lib/workbench/result-window.ts";

function guards(overrides: Record<string, () => boolean> = {}) {
  return {
    isAxialResult: () => false,
    isTrussResult: () => false,
    isHeatBar1dResult: () => false,
    isElectrostaticPlaneQuad2dResult: () => false,
    isElectrostaticPlaneTriangle2dResult: () => false,
    isHeatPlaneQuad2dResult: () => false,
    isHeatPlaneTriangle2dResult: () => false,
    isThermalBar1dResult: () => false,
    isThermalBeam1dResult: () => false,
    isThermalFrame2dResult: () => false,
    isThermalTruss2dResult: () => false,
    isThermalTruss3dResult: () => false,
    isTruss3dResult: () => false,
    isSpring1dResult: () => false,
    isSpring2dResult: () => false,
    isSpring3dResult: () => false,
    isBeam1dResult: () => false,
    isTorsion1dResult: () => false,
    isFrame2dResult: () => false,
    ...overrides,
  };
}

test("result window preserves the authoritative non-axial study kind", () => {
  const broadTrussGuard = guards({ isTrussResult: () => true });

  for (const studyKind of [
    "heat_bar_1d",
    "thermal_frame_2d",
    "spring_2d",
    "plane_quad_2d",
    "thermal_plane_triangle_2d",
  ] as const) {
    assert.equal(resolveResultWindowStudyKind({}, studyKind, broadTrussGuard), studyKind);
  }
});

test("result window fallback prefers a specific 3D spring guard over broad truss shape", () => {
  const detected = resolveResultWindowStudyKind({}, "axial_bar_1d", guards({
    isTruss3dResult: () => true,
    isSpring3dResult: () => true,
  }));

  assert.equal(detected, "spring_3d");
});

test("result window math stays finite for invalid runtime dimensions", () => {
  assert.equal(computeResultWindowSize(1_000, Number.NaN), RESULT_WINDOW_BASE_SIZE);
  assert.equal(clampChunkOffset(100, 1_000, 0), 0);
  assert.equal(clampChunkOffset(Number.NaN, 1_000, 240), 0);
  assert.ok(Number.isFinite(computeVisibleResultWindowOffset(1_000, 0, 0, Number.NaN, Number.NaN)));
});

test("result chunk cache keys cannot collide across delimited backend and job IDs", () => {
  assert.notEqual(
    chunkCacheKey("backend:a", "job", "nodes", 0, 100),
    chunkCacheKey("backend", "a:job", "nodes", 0, 100),
  );
});

test("result chunk cache evicts an empty oldest key instead of exceeding its limit", () => {
  const payload = {
    job_id: "job",
    items: [],
    kind: "nodes",
    limit: 1,
    offset: 0,
    returned: 0,
    total: 0,
  } as never;
  const cache = new Map<string, never>();
  cache.set("", payload);
  for (let index = 1; index < RESULT_WINDOW_CACHE_LIMIT; index += 1) {
    cache.set(`key-${index}`, payload);
  }

  writeChunkCache(cache, "newest", payload);

  assert.equal(cache.size, RESULT_WINDOW_CACHE_LIMIT);
  assert.equal(cache.has(""), false);
});
