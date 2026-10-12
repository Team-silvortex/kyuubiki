import test from "node:test";
import assert from "node:assert/strict";
import { createAdminDataBackendService, type WorkbenchAdminDataBackendTransport } from "../../src/lib/workbench/admin-data-backend-service-core.ts";
import { saveWorkbenchAdminResultRecord, deleteWorkbenchAdminResultRecord, exportWorkbenchAdminResultRecord } from "../../src/components/workbench/workbench-admin-result-controller.ts";
import { EMPTY_WORKBENCH_RUNTIME_RECOVERY_STATE } from "../../src/components/workbench/workbench-runtime-recovery.ts";
import type { WorkbenchRuntimeRecoveryState } from "../../src/components/workbench/workbench-runtime-recovery.ts";
import type { ResultRecord } from "../../src/lib/api/security-results-types.ts";
import { downloadBlobFile } from "../../src/components/workbench/workbench-file-helpers.ts";
import { createWorkbenchFeedback } from "../../src/components/workbench/workbench-feedback.ts";

function fixture() {
  const initial = { job_id: "result", result: { metric: 1 } };
  const state = { records: [initial] as ResultRecord[], selected: "result" as string | null,
    messages: [] as string[], reads: 0, writes: 0, downloads: [] as Array<[string, string]>,
    recovery: EMPTY_WORKBENCH_RUNTIME_RECOVERY_STATE, pending: false };
  let receipt: unknown = { ...initial, deleted: true };
  let failure: unknown;
  const write = async () => {
    state.writes += 1;
    if (failure !== undefined) throw failure;
    return receipt;
  };
  const transport = {
    updateResult: write, deleteResult: write,
    fetchResults: async () => { state.reads += 1; return { results: state.records }; },
    fetchJob: async () => { throw new Error("unexpected job read"); },
    updateJob: async () => { throw new Error("unexpected job write"); },
    deleteJob: async () => { throw new Error("unexpected job deletion"); },
  } as WorkbenchAdminDataBackendTransport;
  const source = { isCurrent: () => true };
  const deps = {
    recordSource: source, draftSource: source,
    feedback: createWorkbenchFeedback(),
    adminDataBackendService: createAdminDataBackendService(transport), resultRefreshSeqRef: { current: 7 },
    setResultRecords: (next: ResultRecord[] | ((current: ResultRecord[]) => ResultRecord[])) => {
      state.records = typeof next === "function" ? next(state.records) : next;
    },
    setSelectedAdminResultJobId: (next: string | null | ((current: string | null) => string | null)) => {
      state.selected = typeof next === "function" ? next(state.selected) : next;
    },
    setRuntimeRecovery: (next: WorkbenchRuntimeRecoveryState | ((current: WorkbenchRuntimeRecoveryState) => WorkbenchRuntimeRecoveryState)) => {
      state.recovery = typeof next === "function" ? next(state.recovery) : next;
    },
    setResultReadPending: (next: boolean) => { state.pending = next; },
    selectedAdminResultJobId: "result" as string | null, adminResultDraft: '{"metric":42}',
    downloadTextFile: (name: string, contents: string) => { state.downloads.push([name, contents]); },
    setMessage: (message: string) => { state.messages.push(message); }, resultScopeLabel: "Results",
    labels: { resultSaved: "saved", resultDeleted: "deleted", resultJsonDownloaded: "exported", invalidJson: "invalid", initialFailed: "failed", sourceChanged: "source changed" },
  };
  return { state, deps, initial, setReceipt: (value: unknown) => { receipt = value; }, setFailure: (value: unknown) => { failure = value; } };
}

const invalidReceipts: Array<[string, unknown]> = [
  ["null", null], ["array", []], ["missing-id", { result: {} }],
  ["wrong-id", { job_id: "other", result: {} }], ["numeric-id", { job_id: 1, result: {} }],
  ["missing-result", { job_id: "result" }], ["null-result", { job_id: "result", result: null }],
  ["array-result", { job_id: "result", result: [] }], ["string-result", { job_id: "result", result: "invalid" }],
];
for (const action of ["save", "delete"] as const) for (const [label, payload] of invalidReceipts) {
  test(`result ${action} rejects ${label} receipt without follow-up reads or retry`, async () => {
    const { deps, state, setReceipt } = fixture();
    setReceipt(action === "delete" && payload && !Array.isArray(payload) ? { ...payload, deleted: true } : payload);
    await assert.rejects(action === "save" ? deps.adminDataBackendService.updateResult("result", {}) : deps.adminDataBackendService.deleteResult("result"),
      /RESULT_RECORD_MUTATION_UNCONFIRMED/u);
    assert.equal(state.writes, 1);
    assert.equal(state.reads, 0);
  });
}

for (const deleted of [undefined, false, "true", 1]) {
  test(`result deletion requires explicit boolean confirmation: ${String(deleted)}`, async () => {
    const { deps, setReceipt } = fixture();
    setReceipt({ job_id: "result", result: {}, deleted });
    await assert.rejects(deps.adminDataBackendService.deleteResult("result"), /RESULT_RECORD_MUTATION_UNCONFIRMED/u);
  });
}

test("result receipts preserve valid payload identity, empty objects and original transport failures", async () => {
  const { deps, state, setReceipt, setFailure } = fixture();
  const receipt = { job_id: "result", result: {}, deleted: true };
  setReceipt(receipt);
  assert.equal(await deps.adminDataBackendService.updateResult("result", {}), receipt);
  assert.equal(await deps.adminDataBackendService.deleteResult("result"), receipt);
  const failure = new Error("response lost");
  setFailure(failure);
  await assert.rejects(deps.adminDataBackendService.updateResult("result", {}), (error) => error === failure);
  await assert.rejects(deps.adminDataBackendService.deleteResult("result"), (error) => error === failure);
  assert.equal(state.writes, 4);
  assert.equal(state.reads, 0);
});

for (const action of ["save", "delete"] as const) {
  test(`unconfirmed result ${action} preserves local records and selection without starting reads`, async () => {
    const { deps, state, initial, setReceipt } = fixture();
    setReceipt(null);
    await (action === "save" ? saveWorkbenchAdminResultRecord(deps) : deleteWorkbenchAdminResultRecord(deps));
    assert.deepEqual(state.records, [initial]);
    assert.equal(state.selected, "result");
    assert.equal(deps.resultRefreshSeqRef.current, 7);
    assert.equal(state.writes, 1);
    assert.equal(state.reads, 0);
    assert.match(state.messages[0], /RESULT_RECORD_MUTATION_UNCONFIRMED/u);
  });
}

const invalidDrafts = ["{", "null", "[]", "42", "false", '"text"', '{"metric":1e400}', '{"nested":[-1e400]}'];
for (const action of ["save", "export"] as const) for (const value of invalidDrafts) {
  test(`result ${action} rejects invalid object draft ${value} before writes or downloads`, async () => {
    const { deps, state } = fixture();
    deps.adminResultDraft = value;
    if (action === "save") await saveWorkbenchAdminResultRecord(deps);
    else exportWorkbenchAdminResultRecord(deps);
    assert.deepEqual(state.messages, ["invalid"]);
    assert.equal(state.writes, 0);
    assert.equal(state.reads, 0);
    assert.deepEqual(state.downloads, []);
    assert.equal(state.selected, "result");
  });
}

for (const value of ["{}", '{"nested":[null,{},[],true,"x",1e-300]}']) {
  test(`result export accepts valid object ${value} without writing or reading the backend`, () => {
    const { deps, state } = fixture();
    deps.adminResultDraft = value;
    exportWorkbenchAdminResultRecord(deps);
    assert.deepEqual(state.messages, ["exported"]);
    assert.equal(state.downloads[0][0], "result-result.json");
    assert.deepEqual(JSON.parse(state.downloads[0][1]), JSON.parse(value));
    assert.equal(state.reads + state.writes, 0);
  });
}

for (const failure of [new Error("download blocked"), new SyntaxError("download failed"), "unavailable"]) {
  test(`download failure ${String(failure)} is not misreported as invalid result JSON`, () => {
    const { deps, state } = fixture();
    deps.downloadTextFile = () => { throw failure; };
    exportWorkbenchAdminResultRecord(deps);
    assert.deepEqual(state.messages, [failure instanceof Error ? failure.message : "failed"]);
    assert.equal(state.reads + state.writes, 0);
    assert.deepEqual(state.downloads, []);
  });
}

test("backend JSON decoding failure is not blamed on a valid result draft", async () => {
  const { deps, state, setFailure } = fixture();
  setFailure(new SyntaxError("backend response is not JSON"));
  await saveWorkbenchAdminResultRecord(deps);
  assert.deepEqual(state.messages, ["backend response is not JSON"]);
  assert.equal(state.writes, 1);
  assert.equal(state.reads, 0);
});

test("no selected result means no save, delete or export", async () => {
  const { deps, state } = fixture();
  deps.selectedAdminResultJobId = null;
  await saveWorkbenchAdminResultRecord(deps);
  await deleteWorkbenchAdminResultRecord(deps);
  exportWorkbenchAdminResultRecord(deps);
  assert.equal(state.reads + state.writes + state.downloads.length, 0);
  assert.deepEqual(state.messages, []);
});

for (const fails of [false, true]) {
  test(`download helper releases its object URL after ${fails ? "failed" : "successful"} activation`, () => {
    const originalDocument = Object.getOwnPropertyDescriptor(globalThis, "document");
    const originalCreate = URL.createObjectURL;
    const originalRevoke = URL.revokeObjectURL;
    const revoked: string[] = [];
    const failure = new Error("activation blocked");
    const link = { href: "", download: "", click: () => { if (fails) throw failure; } };
    Object.defineProperty(globalThis, "document", { configurable: true, value: { createElement: () => link } });
    URL.createObjectURL = () => "blob:owned-result";
    URL.revokeObjectURL = (url) => { revoked.push(url); };
    try {
      if (fails) assert.throws(() => downloadBlobFile("result.json", new Blob(["{}"])), (error) => error === failure);
      else downloadBlobFile("result.json", new Blob(["{}"]));
      assert.equal(link.download, "result.json");
      assert.deepEqual(revoked, ["blob:owned-result"]);
    } finally {
      if (originalDocument) Object.defineProperty(globalThis, "document", originalDocument);
      else Reflect.deleteProperty(globalThis, "document");
      URL.createObjectURL = originalCreate;
      URL.revokeObjectURL = originalRevoke;
    }
  });
}
