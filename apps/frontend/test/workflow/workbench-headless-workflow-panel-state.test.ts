import test from "node:test";
import assert from "node:assert/strict";

import {
  buildFrontendMacroBridgePayload,
  formatHeadlessExecutionResultLogs,
  formatHeadlessHandoffSnapshotLog,
  formatHeadlessHandoffStatusLog,
  moveItem,
  parseFrontendMacroBridgePayload,
  upsertLatestHeadlessHandoffReceipt,
} from "@/components/workbench/workbench-headless-workflow-panel-state";
import type { HeadlessHandoffReceipt, HeadlessHandoffSnapshot } from "@/lib/api/headless-handoff-client";

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

test("headless workflow panel state upserts the latest handoff receipt", () => {
  const first = receipt("handoff-a");
  const duplicate = { ...receipt("handoff-a"), status_message: "updated" };
  const second = receipt("handoff-b");

  assert.deepEqual(upsertLatestHeadlessHandoffReceipt([first, second], duplicate), [duplicate, second]);
});

test("headless workflow panel state formats stable execution and handoff logs", () => {
  assert.deepEqual(
    formatHeadlessExecutionResultLogs({
      steps: [
        { index: 1, action: "service_health", result: { status: "ok" } },
        { index: 2, action: "job_fetch", result: { job_id: "job-a", progress: 1 } },
      ],
    }),
    [
      '[result] step 1 service_health: {"status":"ok"}',
      '[result] step 2 job_fetch: {"job_id":"job-a","progress":1}',
    ],
  );

  assert.equal(formatHeadlessHandoffStatusLog({ handoff_id: "handoff-a", stage: "ready" }), '[handoff-status] {"handoff_id":"handoff-a","stage":"ready"}');
  assert.equal(formatHeadlessHandoffSnapshotLog(snapshot("handoff-a")), "[handoff-snapshot] handoff-a");
});

function receipt(handoffId: string): HeadlessHandoffReceipt {
  return {
    accepted: true,
    handoff_id: handoffId,
    workflow_id: "workflow-a",
    received_at: "2026-01-01T00:00:00.000Z",
    authority_mode: "orchestra",
    step_count: 2,
    chosen_agent_count: 1,
    warning_count: 0,
    target_clusters: [],
    has_dispatch_override: false,
    dispatch_override_count: 0,
    override_acknowledged: false,
    override_note: null,
    override_step_keys: [],
    override_summary: null,
    stage: "ready",
    status_message: "ready",
  };
}

function snapshot(handoffId: string): HeadlessHandoffSnapshot {
  return {
    ...receipt(handoffId),
    envelope: {} as HeadlessHandoffSnapshot["envelope"],
  };
}
