import test from "node:test";
import assert from "node:assert/strict";

import {
  buildFrontendMacroBridgePayload,
  moveItem,
  parseFrontendMacroBridgePayload,
} from "@/components/workbench/workbench-headless-workflow-panel-state";

test("headless workflow panel state round-trips frontend macro bridge payload", () => {
  const payload = buildFrontendMacroBridgePayload({
    id: "macro/demo",
    steps: [
      { action: "project_create", payload: { name: "Demo" } },
      { action: "service_health" },
    ],
  });

  assert.deepEqual(payload, {
    macro_id: "macro/demo",
    replay_mode: "bridge",
    step_count: 2,
    steps: [
      { action: "project_create", payload: { name: "Demo" } },
      { action: "service_health", payload: {} },
    ],
  });
  assert.deepEqual(parseFrontendMacroBridgePayload(payload), {
    id: "macro/demo",
    steps: [
      { action: "project_create", payload: { name: "Demo" } },
      { action: "service_health", payload: {} },
    ],
  });
});

test("headless workflow panel state ignores invalid bridge steps and preserves invalid moves", () => {
  assert.deepEqual(
    parseFrontendMacroBridgePayload({
      steps: [{ action: "service_health" }, { action: 42 }, null],
    }),
    { id: "macro/frontend-bridge-restored", steps: [{ action: "service_health" }] },
  );

  const items = ["a", "b", "c"];
  assert.deepEqual(moveItem(items, 0, 2), ["b", "c", "a"]);
  assert.equal(moveItem(items, 0, -1), items);
});
