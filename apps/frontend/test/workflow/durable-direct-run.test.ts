import test from "node:test";
import assert from "node:assert/strict";
import { createDurableDirectRunService } from "../../src/lib/workbench/durable-direct-run.ts";
import { createBrowserDirectExecutionJournal, type DirectExecutionIntent, type DirectExecutionJournal } from "../../src/lib/workbench/direct-execution-journal.ts";
import { createDurableDirectApiClient, type DurableDirectEnvelope } from "../../src/lib/api/durable-direct-client.ts";
import { observeWorkbenchDurableRun } from "../../src/components/workbench/workbench-durable-run-observer.ts";
import { cancelWorkbenchJob } from "../../src/components/workbench/workbench-job-history-controller.ts";
import { checkpointDigest } from "../../src/lib/workbench/checkpoint-journal.ts";

const endpoint = "127.0.0.1:5001", id = "a".repeat(32);
const input = { length: 1, area: 0.01, elements: 4, youngs_modulus: 200e9, tip_force: 10 };
function report(executionId = id, status = "running"): DurableDirectEnvelope<any> {
  return { gateway_runtime: "rust-native", endpoint, execution: {
    schema_version: "kyuubiki.agent-durable-execution/v1", execution_id: executionId, status,
    request_id: "original-request", request_digest: "b".repeat(64), process_instance_id: "original-process", method: "solve_bar_1d",
    automatic_replay_authorized: false, resumable: false,
    response: status === "completed" ? { rpc_version: 1, id: "original-request", ok: true, result: { displacement: 1 } } : null,
  } } as DurableDirectEnvelope<any>;
}
function fixture() {
  const entries: DirectExecutionIntent[] = [], calls: { path: string; body: any }[] = [], order: string[] = [];
  let identity = "http://gateway.test/api/direct-mesh/tasks", authorized = true, fail = false, next = 0;
  const journal: DirectExecutionJournal = {
    async list() { return structuredClone(entries); },
    async reserve(intent) {
      order.push("save");
      const existing = entries.find(e => e.scope === intent.scope && e.endpoint === intent.endpoint);
      if (existing) return { intent: structuredClone(existing), is_new: false };
      entries.push(structuredClone(intent)); return { intent, is_new: true };
    },
    async remove(intent) { const index = entries.findIndex(e => e.execution_id === intent.execution_id); if (index >= 0) entries.splice(index, 1); },
  };
  const client = { async fetchDirectMeshAgents() { return { agents: [{ host: "127.0.0.1", port: 5001,
    descriptor: { durable_execution: { schema_version: "kyuubiki.agent-durable-execution/v1", enabled: true } } }] } as never; }, ...createDurableDirectApiClient(async (path, init) => {
    const body = JSON.parse(String(init?.body)); order.push("http"); calls.push({ path, body });
    if (fail) throw new Error("response lost");
    return report(body.execution_id) as never;
  }), async cancelDurableDirectExecution(target: any) { calls.push({ path: "cancel", body: target }); return { recovery: { status: "requested" }, endpoint } as never; } };
  const options = { journal, client, gatewayIdentity: () => identity, captureAuthority: () => {
    const captured = authorized; return () => captured === authorized;
  }, newId: () => (++next).toString(16).padStart(32, "0") };
  return { entries, calls, order, journal, client, options, service: createDurableDirectRunService(options),
    setIdentity: (value: string) => { identity = value; }, revoke: () => { authorized = false; }, loseReply: () => { fail = true; } };
}
test("default durable run saves only bounded identity metadata before dispatch and cleans after a terminal receipt", async () => {
  const f = fixture();
  const submitted = await f.service.submit("axial_bar_1d", input, endpoint);
  assert.deepEqual(f.order, ["save", "http"]);
  assert.equal(f.entries.length, 1);
  assert.deepEqual(Object.keys(f.entries[0]).sort(), ["created_at", "endpoint", "execution_id", "scope", "study_kind"]);
  assert.equal(f.calls[0].path, "/api/direct-mesh/tasks");
  assert.equal(f.calls[0].body.execution_id, submitted.handle.execution_id);
  await f.service.acknowledge(submitted.handle, report(submitted.handle.execution_id, "completed"));
  assert.equal(f.entries.length, 0);
});
test("a local write failure prevents all network dispatch, with no browser RAM fallback", async () => {
  const f = fixture();
  const service = createDurableDirectRunService({ ...f.options, journal: { ...f.journal, reserve: async () => { throw new Error("disk unavailable"); } } });
  await assert.rejects(service.submit("axial_bar_1d", input, endpoint), /disk unavailable/);
  assert.equal(f.calls.length, 0);
  await assert.rejects(createBrowserDirectExecutionJournal(() => undefined as never).list(), /journal_unavailable/);
});
test("lost submission response survives service recreation and blocks new calculation IDs", async () => {
  const f = fixture(); f.loseReply();
  await assert.rejects(f.service.submit("axial_bar_1d", input, endpoint), /Submission unconfirmed/);
  const reopened = createDurableDirectRunService(f.options);
  const [handle] = await reopened.pending();
  assert.equal(handle.execution_id, f.calls[0].body.execution_id);
  await assert.rejects(reopened.submit("axial_bar_1d", { ...input, tip_force: 20 }, endpoint), /recovery_required/);
  assert.equal(f.calls.length, 1);
});
test("two views racing to run one Agent cannot both dispatch", async () => {
  const f = fixture(), other = createDurableDirectRunService(f.options);
  const first = f.service.submit("axial_bar_1d", input, endpoint);
  const second = other.submit("axial_bar_1d", input, endpoint);
  const outcomes = await Promise.allSettled([first, second]);
  assert.equal(outcomes.filter(r => r.status === "fulfilled").length, 1);
  assert.equal(f.calls.length, 1); assert.equal(f.entries.length, 1);
});
test("authority changed during persistence leaves the identity but dispatches nothing", async () => {
  const f = fixture();
  const service = createDurableDirectRunService({ ...f.options, journal: { ...f.journal, reserve: async intent => {
    const value = await f.journal.reserve(intent); f.revoke(); return value;
  } } });
  await assert.rejects(service.submit("axial_bar_1d", input, endpoint), /authority_changed/);
  assert.equal(f.calls.length, 0); assert.equal(f.entries.length, 1);
});
test("model edits during the async scope digest cannot change the captured request", async () => {
  const f = fixture(), mutable = { ...input };
  const pending = f.service.submit("axial_bar_1d", mutable, endpoint); mutable.tip_force = 99;
  await pending;
  assert.equal(f.calls[0].body.input.tip_force, 10);
});
test("switching gateways hides pending identities and rejects querying an old handle", async () => {
  const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  f.setIdentity("http://another.test/api/direct-mesh/tasks");
  assert.deepEqual(await f.service.pending(), []);
  await assert.rejects(f.service.inspect(handle), /original gateway/);
  assert.equal(f.calls.length, 1);
});
test("unknown and missing receipts never authorize local identity cleanup", async () => {
  const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  for (const state of ["running", "outcome_unknown", "not_found"]) {
    await assert.rejects(f.service.acknowledge(handle, report(handle.execution_id, state)), /terminal_receipt_required/);
  }
  assert.equal(f.entries.length, 1);
});
test("cancellation first inspects and binds the original producer and request digest", async () => {
  const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  await f.service.cancel(handle);
  assert.equal(f.calls[1].path, "/api/direct-mesh/tasks/inspect");
  assert.deepEqual(f.calls[2].body, { endpoint, execution_id: handle.execution_id, process_instance_id: "original-process", request_digest: "b".repeat(64) });
  assert.equal(f.entries.length, 1);
});
test("invalid local study payload never consumes a journal slot", async () => {
  const f = fixture();
  await assert.rejects(f.service.submit("beam_1d", {}, endpoint), /nodes/);
  assert.equal(f.entries.length, 0); assert.equal(f.calls.length, 0);
});
test("old or disabled Agent is rejected before reserving an unqueryable execution", async () => {
  const f = fixture();
  const service = createDurableDirectRunService({ ...f.options, client: { ...f.client, fetchDirectMeshAgents: async () => ({ agents: [] }) as never } });
  await assert.rejects(service.submit("axial_bar_1d", input, endpoint), /durable_agent_required/);
  assert.equal(f.entries.length, 0); assert.equal(f.calls.length, 0);
});
test("observation keeps durable ownership, queries only the original handle, then applies a confirmed result", async () => {
  const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  const jobs: any[] = [], results: any[] = [], messages: string[] = [];
  const result = await observeWorkbenchDurableRun({ handle, envelope: report(handle.execution_id),
    service: { ...f.service, inspect: async value => { assert.deepEqual(value, handle); return report(handle.execution_id, "completed"); } },
    isCurrent: () => true, setJob: value => jobs.push(value), setResult: value => results.push(value), setMessage: value => messages.push(value), sleep: async () => {} });
  assert.equal(result.completion, "terminal");
  assert.deepEqual(jobs.map(j => j.status), ["solving", "completed"]);
  assert.ok(jobs.every(j => j.direct_execution.execution_id === handle.execution_id));
  assert.deepEqual(results, [{ displacement: 1 }]); assert.equal(f.entries.length, 0);
});
test("superseded observation publishes nothing and leaves recovery metadata", async () => {
  const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  await assert.rejects(observeWorkbenchDurableRun({ handle, envelope: report(handle.execution_id, "completed"), service: f.service,
    isCurrent: () => false, setJob: () => assert.fail(), setResult: () => assert.fail(), setMessage: () => assert.fail() }), /superseded/);
  assert.equal(f.entries.length, 1);
});
test("changed producer or input receipt cannot replace a running observation", async () => {
  const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  const replacement = report(handle.execution_id, "completed");
  if (replacement.execution.status !== "not_found") replacement.execution.request_digest = "c".repeat(64);
  const jobs: any[] = [];
  await assert.rejects(observeWorkbenchDurableRun({ handle, envelope: report(handle.execution_id),
    service: { ...f.service, inspect: async () => replacement }, isCurrent: () => true,
    setJob: value => jobs.push(value), setResult: () => assert.fail(), setMessage: () => {}, sleep: async () => {} }), /receipt_identity_changed/);
  assert.deepEqual(jobs.map(j => j.status), ["solving"]); assert.equal(f.entries.length, 1);
});
test("ambiguous endpoint cannot cause even a capability lookup", async () => {
  const f = fixture();
  const service = createDurableDirectRunService({ ...f.options, client: { ...f.client, fetchDirectMeshAgents: async () => assert.fail() } });
  await assert.rejects(service.submit("axial_bar_1d", input, "agent:05001"), /canonical/);
  assert.equal(f.entries.length, 0);
});
test("bounded observation detaches without cancelling, acknowledging or resubmitting", async () => {
  const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  let checks = 0;
  const result = await observeWorkbenchDurableRun({ handle, envelope: report(handle.execution_id), service: { ...f.service,
    inspect: async () => { checks++; return report(handle.execution_id); } }, isCurrent: () => true,
    setJob: () => {}, setResult: () => assert.fail(), setMessage: () => {}, sleep: async () => {} });
  assert.equal(result.completion, "detached"); assert.equal(checks, 40); assert.equal(f.calls.length, 1); assert.equal(f.entries.length, 1);
});
test("ordinary cancel button routes a durable handle without calling Orchestra or pretending it stopped", async () => {
  const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  const token = { current: 3 }, messages: string[] = [];
  const result = await cancelWorkbenchJob({ jobId: `direct-durable-${handle.execution_id}`, directExecution: handle,
    durableDirectService: f.service, jobPollTokenRef: token, jobHistoryBackendService: { cancelJob: async () => assert.fail() } as never,
    labels: { jobCancelled: "cancelled", initialFailed: "failed", requestTimedOut: "timeout" }, refreshJobHistory: async () => assert.fail(),
    setJob: () => assert.fail(), setResult: () => assert.fail(), setMessage: message => { messages.push(message as string); } });
  assert.equal(result.ok, true); assert.equal(token.current, 3); assert.match(messages[0], /cancel_requested/);
  assert.equal(f.entries.length, 1);
});
test("default gateway scope is a hash, not persisted connection credentials", async () => {
  const f = fixture(); await f.service.submit("axial_bar_1d", input, endpoint);
  assert.equal(f.entries[0].scope, await checkpointDigest(f.options.gatewayIdentity()));
  assert.doesNotMatch(JSON.stringify(f.entries), /http|authorization|token/);
});
