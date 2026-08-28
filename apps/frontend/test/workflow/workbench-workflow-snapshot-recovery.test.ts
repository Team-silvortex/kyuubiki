import test from "node:test";
import assert from "node:assert/strict";

const SNAPSHOT_INDEX_KEY = "kyuubiki.workbench.workflowSnapshots.index.v1";
const SNAPSHOT_PAYLOAD_PREFIX = "kyuubiki.workbench.workflowSnapshots.payload.v1:";
const LEGACY_SNAPSHOT_ID = "snapshot_legacy_payload";
const LEGACY_PAYLOAD_KEY = `${SNAPSHOT_PAYLOAD_PREFIX}${LEGACY_SNAPSHOT_ID}`;

function createMemoryStorage(initial: Record<string, string>): Storage {
  const records = new Map(Object.entries(initial));
  return {
    get length() {
      return records.size;
    },
    clear: () => records.clear(),
    getItem: (key) => records.get(key) ?? null,
    key: (index) => [...records.keys()][index] ?? null,
    removeItem: (key) => records.delete(key),
    setItem: (key, value) => records.set(key, String(value)),
  };
}

const storedIndex = JSON.stringify([
  {
    id: "snapshot_missing_payload",
    workflowId: "workflow.recovery",
    workflowName: "Recovery workflow",
    createdAt: "2026-08-27T00:00:00.000Z",
    reason: "interrupted deferred write",
    summary: ["payload was not committed"],
    payloadState: "full",
  },
  {
    id: LEGACY_SNAPSHOT_ID,
    workflowId: "workflow.legacy-recovery",
    workflowName: "Legacy recovery workflow",
    createdAt: "2026-08-27T00:00:01.000Z",
    reason: "legacy payload migration",
    summary: [],
    payloadState: "full",
  },
  {
    id: "snapshot_missing_payload",
    workflowId: "workflow.recovery",
    workflowName: "Duplicate recovery workflow",
    createdAt: "2026-08-27T00:00:02.000Z",
    reason: "duplicate id",
    summary: [],
    payloadState: "summary_only",
  },
  {
    id: "snapshot_invalid_date",
    workflowId: "workflow.invalid",
    workflowName: "Invalid workflow",
    createdAt: "not-a-date",
    reason: "invalid timestamp",
    summary: [],
    payloadState: "summary_only",
  },
  ...Array.from({ length: 25 }, (_, index) => ({
    id: `snapshot_over_limit_${index}`,
    workflowId: "workflow.over-limit",
    workflowName: "Bounded workflow",
    createdAt: `2026-08-27T00:${String(index).padStart(2, "0")}:00.000Z`,
    reason: "bounded recovery",
    summary: [],
    payloadState: "summary_only",
  })),
]);
const legacyPayload = JSON.stringify({
  graph: {
    schema_version: "kyuubiki.workflow/v1",
    id: "workflow.legacy-recovery",
    name: "Legacy recovery workflow",
    nodes: [],
    edges: [],
  },
  inputArtifactTexts: { model: "legacy-sensitive-input" },
});
const backingStorage = createMemoryStorage({
  [SNAPSHOT_INDEX_KEY]: storedIndex,
  [LEGACY_PAYLOAD_KEY]: legacyPayload,
});
const localStorage = {
  ...backingStorage,
  setItem: (key: string, value: string) => {
    if (key === LEGACY_PAYLOAD_KEY) throw new Error("storage is read-only");
    backingStorage.setItem(key, value);
  },
} as Storage;
const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: { localStorage } as unknown as Window,
});

test.after(() => {
  if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
  else Reflect.deleteProperty(globalThis, "window");
});

test("startup reconciliation downgrades full snapshots with missing payloads", async () => {
  const snapshots = await import(
    "../../src/components/workbench/workflow/workbench-workflow-snapshot-storage.ts"
  );

  const [summary] = snapshots.listStoredWorkflowSnapshots("workflow.recovery");
  assert.equal(summary?.payloadState, "summary_only");
  assert.equal(snapshots.listStoredWorkflowSnapshots("workflow.recovery").length, 1);
  assert.equal(snapshots.listStoredWorkflowSnapshots("workflow.invalid").length, 0);

  const persisted = JSON.parse(localStorage.getItem(SNAPSHOT_INDEX_KEY) ?? "[]") as Array<{
    id?: string;
    payloadState?: string;
  }>;
  assert.equal(persisted[0]?.payloadState, "summary_only");
  assert.equal(persisted.length, 20);
  assert.equal(new Set(persisted.map((entry) => entry.id)).size, persisted.length);
  assert.equal(snapshots.listStoredWorkflowSnapshots("workflow.over-limit").length, 18);

  const [legacySummary] = snapshots.listStoredWorkflowSnapshots("workflow.legacy-recovery");
  assert.equal(legacySummary?.payloadState, "full");
  assert.equal(
    snapshots.loadStoredWorkflowSnapshot(LEGACY_SNAPSHOT_ID)?.graph.id,
    "workflow.legacy-recovery",
  );
});
