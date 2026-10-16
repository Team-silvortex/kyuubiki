import test from "node:test";
import assert from "node:assert/strict";
import { createDurableDirectRunService } from "../../src/lib/workbench/durable-direct-run.ts";
import { createBrowserDirectExecutionJournal, type DirectExecutionIntent, type DirectExecutionJournal } from "../../src/lib/workbench/direct-execution-journal.ts";
import { createDurableDirectApiClient, DurableDirectSubmissionRejected, type DurableDirectEnvelope } from "../../src/lib/api/durable-direct-client.ts";
import { observeWorkbenchDurableRun } from "../../src/components/workbench/workbench-durable-run-observer.ts";
import { cancelWorkbenchJob } from "../../src/components/workbench/workbench-job-history-controller.ts";
import { createWorkbenchFeedback, withWorkbenchFeedbackInvalidation } from "../../src/components/workbench/workbench-feedback.ts";
import { checkpointDigest } from "../../src/lib/workbench/checkpoint-journal.ts";
import { captureWorkbenchApiAuthority } from "../../src/lib/api/auth-context.ts";
import type { WorkbenchStudyResult } from "../../src/lib/workbench/study-run-backend-service-core.ts";

const endpoint = "127.0.0.1:5001", id = "a".repeat(32);
const storeId = "c".repeat(32);
const input = { length: 1, area: 0.01, elements: 4, youngs_modulus: 200e9, tip_force: 10 };
function report(executionId = id, status = "running", failureCode = "durable_admission_rejected"): DurableDirectEnvelope<any> {
  return { gateway_runtime: "rust-native", endpoint, execution: {
    store_id: storeId,
    schema_version: "kyuubiki.agent-durable-execution/v1", execution_id: executionId, status,
    request_id: "original-request", request_digest: "b".repeat(64), process_instance_id: "original-process", method: "solve_bar_1d",
    automatic_replay_authorized: false, resumable: false,
    response: status === "completed" ? { rpc_version: 1, id: "original-request", ok: true, result: { displacement: 1 } }
      : status === "failed" ? { rpc_version: 1, id: "original-request", ok: false, error: {
        code: failureCode, message: "execution failed", details: { automatic_replay_authorized: false,
          ...(failureCode === "durable_response_unrecordable" ? { phase: "receipt_validation", reason: "byte_limit",
            max_receipt_bytes: 8 * 1024 * 1024, execution_may_have_started: true,
            original_response_discarded: true, external_effects_rolled_back: false }
            : failureCode === "durable_execution_panicked" ? { phase: "async_worker_unwind", execution_may_have_started: true,
            partial_result_discarded: true, external_effects_rolled_back: false }
            : { execution_started: false, ...(failureCode === "durable_execution_start_failed"
              ? { phase: "before_engine_dispatch" } : { reason: "execution_unavailable" }) }) },
      } } : null,
  } } as DurableDirectEnvelope<any>;
}
function fixture() {
  const entries: DirectExecutionIntent[] = [], calls: { path: string; body: any }[] = [], order: string[] = [];
  let identity = "http://gateway.test/api/direct-mesh/tasks", authorized = true, fail = false, next = 0;
  let reply: (body: any) => any = body => report(body.execution_id);
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
    descriptor: { durable_execution: { schema_version: "kyuubiki.agent-durable-execution/v1", enabled: true, store_id: storeId } } }] } as never; }, ...createDurableDirectApiClient(async (path, init) => {
    const body = JSON.parse(String(init?.body)); order.push("http"); calls.push({ path, body });
    if (fail) throw new Error("response lost");
    return reply(body) as never;
  }), async cancelDurableDirectExecution(target: any) { calls.push({ path: "cancel", body: target }); return { recovery: { status: "requested" }, endpoint } as never; } };
  const options = { journal, client, gatewayIdentity: () => identity, captureAuthority: () => {
    const captured = authorized; return () => captured === authorized;
  }, newId: () => (++next).toString(16).padStart(32, "0") };
  return { entries, calls, order, journal, client, options, service: createDurableDirectRunService(options),
    setReply: (value: (body: any) => any) => { reply = value; },
    setIdentity: (value: string) => { identity = value; }, revoke: () => { authorized = false; }, loseReply: () => { fail = true; } };
}
test("default durable run saves only bounded identity metadata before dispatch and cleans after a terminal receipt", async () => {
  const f = fixture();
  const submitted = await f.service.submit("axial_bar_1d", input, endpoint);
  assert.deepEqual(f.order, ["save", "http"]);
  assert.equal(f.entries.length, 1);
  assert.deepEqual(Object.keys(f.entries[0]).sort(), ["created_at", "endpoint", "execution_id", "scope", "store_id", "study_kind"]);
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
test("authority changed during persistence releases only the new unsent identity", async () => {
  const f = fixture();
  const service = createDurableDirectRunService({ ...f.options, journal: { ...f.journal, reserve: async intent => {
    const value = await f.journal.reserve(intent); f.revoke(); return value;
  } } });
  await assert.rejects(service.submit("axial_bar_1d", input, endpoint), /authority_changed/);
  assert.equal(f.calls.length, 0); assert.equal(f.entries.length, 0);
});
for (const phase of ["reservation", "submission receipt"]) {
  test(`document lifecycle during ${phase} cannot resume the original submission`, async () => {
    const previous = globalThis.window, events = new EventTarget(), f = fixture();
    globalThis.window = { location: { search: "" }, localStorage: { getItem: () => null },
      addEventListener: events.addEventListener.bind(events),
    } as unknown as Window & typeof globalThis;
    const restore = () => { events.dispatchEvent(new Event("pagehide")); events.dispatchEvent(new Event("pageshow")); };
    const journal = { ...f.journal, reserve: async (intent: DirectExecutionIntent) => {
      const result = await f.journal.reserve(intent);
      if (phase === "reservation") restore();
      return result;
    } };
    const service = createDurableDirectRunService({ ...f.options, journal, captureAuthority: captureWorkbenchApiAuthority });
    try {
      if (phase === "submission receipt") f.setReply(body => { restore(); return report(body.execution_id, "completed"); });
      await assert.rejects(service.submit("axial_bar_1d", input, endpoint),
        phase === "reservation" ? /authority_changed/ : /Submission unconfirmed/);
      assert.equal(f.calls.length, phase === "reservation" ? 0 : 1);
      // Only this invocation's confirmed, unsent reservation may be released.
      assert.equal(f.entries.length, phase === "reservation" ? 0 : 1);
      if (phase === "submission receipt") {
        const [handle] = await service.pending();
        assert.equal(handle.execution_id, f.calls[0].body.execution_id);
        f.setReply(body => report(body.execution_id, "completed"));
        await service.acknowledge(handle, await service.inspect(handle));
        assert.equal(f.entries.length, 0);
        assert.deepEqual(f.calls.map(call => call.path), ["/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect"]);
      }
    } finally { globalThis.window = previous; }
  });
}
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
  assert.deepEqual(f.calls[2].body, { endpoint, store_id: storeId, execution_id: handle.execution_id, process_instance_id: "original-process", request_digest: "b".repeat(64) });
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

test("busy store metadata is not mistaken for an old Agent or allowed to erase a pending handle", async () => {
  const f = fixture();
  const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  const saved = structuredClone(f.entries);
  for (const identity of [undefined, storeId]) {
    const service = createDurableDirectRunService({ ...f.options, client: { ...f.client,
      fetchDirectMeshAgents: async () => ({ agents: [{ host: "127.0.0.1", port: 5001,
        descriptor: { durable_execution: { schema_version: "kyuubiki.agent-durable-execution/v1",
          enabled: true, available: false, unavailable_reason: "store_busy", store_id: identity } } }] }) as never,
    } });
    await assert.rejects(service.submit("axial_bar_1d", input, endpoint), /direct:store_busy/);
    assert.deepEqual(await service.pending(), [handle]);
    assert.deepEqual(f.entries, saved);
    assert.equal(f.calls.length, 1);
  }
});

test("unavailable store and failed commit have distinct preflight errors without submission", async () => {
  for (const [policy, code] of [
    [{ available: false, unavailable_reason: "store_unavailable" }, "store_unavailable"],
    [{ available: true, admission_blocked_after_write_failure: true }, "store_write_failed"],
  ] as const) {
    const f = fixture();
    const service = createDurableDirectRunService({ ...f.options, client: { ...f.client,
      fetchDirectMeshAgents: async () => ({ agents: [{ host: "127.0.0.1", port: 5001,
        descriptor: { durable_execution: { schema_version: "kyuubiki.agent-durable-execution/v1",
          enabled: true, store_id: storeId, ...policy } } }] }) as never,
    } });
    await assert.rejects(service.submit("axial_bar_1d", input, endpoint), new RegExp(`direct:${code}`));
    assert.equal(f.calls.length, 0); assert.equal(f.entries.length, 0);
  }
});

test("failed recovery calls retain the pinned handle and never submit or cancel in the background", async () => {
  const f = fixture();
  const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  f.setReply(() => { throw new Error("durable_execution_busy"); });
  await assert.rejects(f.service.inspect(handle), /durable_execution_busy/);
  await assert.rejects(f.service.cancel(handle), /durable_execution_busy/);
  assert.deepEqual(await createDurableDirectRunService(f.options).pending(), [handle]);
  assert.deepEqual(f.calls.map(call => call.path), ["/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect", "/api/direct-mesh/tasks/inspect"]);
  f.setReply(body => report(body.execution_id, "completed"));
  const receipt = await f.service.inspect(handle);
  await f.service.acknowledge(handle, receipt);
  assert.equal(f.entries.length, 0);
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
  const token = { current: 3 }, messages: string[] = [], feedback = createWorkbenchFeedback();
  const result = await cancelWorkbenchJob({ jobId: `direct-durable-${handle.execution_id}`, directExecution: handle, feedback,
    durableDirectService: f.service, jobPollTokenRef: token, jobHistoryBackendService: { cancelJob: async () => assert.fail() } as never,
    labels: { jobCancelled: "cancelled", initialFailed: "failed", requestTimedOut: "timeout" }, refreshJobHistory: async () => assert.fail(),
    setJob: () => assert.fail(), setResult: () => assert.fail(),
    setMessage: withWorkbenchFeedbackInvalidation(feedback, message => { messages.push(message as string); }) });
  assert.equal(result.ok, true); assert.equal(token.current, 3); assert.match(messages[0], /cancel_requested/);
  assert.equal(f.entries.length, 1);
});
for (const terminal of ["completed", "failed"]) {
  for (const lostReply of [false, true]) {
    test(`late cancel ${lostReply ? "failure" : "reply"} cannot overwrite ${terminal} feedback for the same execution`, async () => {
      const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
      const feedback = createWorkbenchFeedback(), token = { current: 3 }, messages: unknown[] = [], results: unknown[] = [];
      const setMessage = withWorkbenchFeedbackInvalidation(feedback, (value: unknown) => { messages.push(value); });
      let release!: () => void;
      const gate = new Promise<void>(resolve => { release = resolve; });
      let cancellations = 0;
      const args = { jobId: `direct-durable-${handle.execution_id}`, directExecution: handle, feedback,
        durableDirectService: { ...f.service, cancel: async () => {
          cancellations++; await gate;
          if (lostReply) throw new Error("cancel response lost");
          return { status: "requested" } as never;
        } }, jobPollTokenRef: token, jobHistoryBackendService: { cancelJob: async () => assert.fail() } as never,
        labels: { jobCancelled: "cancelled", initialFailed: "failed", requestTimedOut: "timeout" }, refreshJobHistory: async () => assert.fail(),
        setJob: () => assert.fail(), setResult: () => assert.fail(), setMessage };
      const cancelling = cancelWorkbenchJob(args);
      await observeWorkbenchDurableRun({ handle, envelope: report(handle.execution_id, terminal, "cancelled"), service: f.service,
        isCurrent: () => true, setJob: () => {}, setResult: value => results.push(value), setMessage,
      }).catch(error => { setMessage(error.message); });
      const terminalMessages = [...messages];
      assert.equal(terminalMessages.length, 1);
      assert.match(String(terminalMessages[0]), terminal === "completed" ? /direct:completed/ : /cancelled/);
      release();
      assert.equal((await cancelling).ok, !lostReply);
      assert.deepEqual(messages, terminalMessages);
      assert.deepEqual(results, terminal === "completed" ? [{ displacement: 1 }] : [null]);
      assert.equal(token.current, 3); assert.equal(cancellations, 1); assert.equal(f.entries.length, 0);
    });
  }
}
test("default gateway scope is a hash, not persisted connection credentials", async () => {
  const f = fixture(); await f.service.submit("axial_bar_1d", input, endpoint);
  assert.equal(f.entries[0].scope, await checkpointDigest(f.options.gatewayIdentity()));
  assert.doesNotMatch(JSON.stringify(f.entries), /http|authorization|token/);
});

function rejection(executionId: string, reason = "execution_unavailable"): any {
  return { endpoint, gateway_runtime: "rust-native", execution: {
    store_id: storeId,
    schema_version: "kyuubiki.agent-durable-admission-rejection/v1", status: "not_admitted",
    execution_id: executionId, method: "solve_bar_1d", request_id: "submit-request",
    process_instance_id: "producer", request_digest: "b".repeat(64), reason,
    message: "Agent is busy", execution_started: false, automatic_replay_authorized: false,
  } };
}

test("persistent owner fence survives service recreation and is sent on inspection and cancellation", async () => {
  const f = fixture();
  const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  assert.equal(f.entries[0].store_id, storeId);
  assert.equal(f.calls[0].body.expected_store_id, storeId);
  const reopened = createDurableDirectRunService(f.options);
  const [saved] = await reopened.pending();
  assert.deepEqual(saved, handle);
  await reopened.inspect(saved);
  assert.equal(f.calls[1].body.expected_store_id, storeId);
  await reopened.cancel(saved);
  assert.equal(f.calls.at(-1)?.body.store_id, storeId);
});

for (const status of ["running", "outcome_unknown", "not_found", "completed", "failed"]) {
  test(`another store at the same endpoint cannot supply a ${status} recovery receipt`, async () => {
    const f = fixture();
    const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
    f.setReply(body => { const value = report(body.execution_id, status); value.execution.store_id = "d".repeat(32); return value; });
    const reopened = createDurableDirectRunService(f.options);
    await assert.rejects(reopened.inspect(handle), /store identity mismatch/);
    await assert.rejects(reopened.cancel(handle), /store identity mismatch/);
    const foreign = report(handle.execution_id, "completed"); foreign.execution.store_id = "d".repeat(32);
    await assert.rejects(reopened.acknowledge(handle, foreign), /store identity mismatch/);
    assert.equal(f.calls.filter(c => c.path === "cancel").length, 0);
    assert.equal(f.entries.length, 1);
    assert.equal(f.entries[0].store_id, storeId);
  });
}

test("missing owner echo or foreign rejection cannot release the pre-dispatch reservation", async () => {
  for (const reject of [false, true]) {
    const f = fixture();
    f.setReply(body => {
      const value = reject ? rejection(body.execution_id) : report(body.execution_id);
      if (reject) value.execution.store_id = "d".repeat(32); else delete value.execution.store_id;
      return value;
    });
    await assert.rejects(f.service.submit("axial_bar_1d", input, endpoint), /Submission unconfirmed/);
    assert.equal(f.entries.length, 1);
    assert.equal(f.calls.length, 1);
  }
});

test("legacy unpinned recovery remains visible but cannot be silently adopted or removed", async () => {
  const f = fixture();
  const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  delete f.entries[0].store_id;
  const [legacy] = await f.service.pending();
  assert.equal(legacy.store_id, undefined);
  await assert.rejects(f.service.inspect(legacy), /owner_fence_required/);
  await assert.rejects(f.service.cancel(legacy), /owner_fence_required/);
  await assert.rejects(f.service.acknowledge(legacy, report(handle.execution_id, "completed")), /owner_fence_required/);
  await assert.rejects(f.service.submit("axial_bar_1d", input, endpoint), /recovery_required/);
  assert.equal(f.calls.length, 1); assert.equal(f.entries.length, 1);
});
for (const reason of ["store_capacity", "execution_unavailable"]) {
  test(`confirmed ${reason} releases the caller slot but never automatically retries`, async () => {
    const f = fixture(); f.setReply(body => rejection(body.execution_id, reason));
    await assert.rejects(f.service.submit("axial_bar_1d", input, endpoint), error => error instanceof DurableDirectSubmissionRejected);
    assert.equal(f.calls.length, 1); assert.equal(f.entries.length, 0);
    f.setReply(body => report(body.execution_id));
    await f.service.submit("axial_bar_1d", input, endpoint);
    assert.equal(f.calls.length, 2); assert.equal(f.entries.length, 1);
    assert.notEqual(f.calls[0].body.execution_id, f.calls[1].body.execution_id);
  });
}
for (const [name, change] of Object.entries({
  foreign_id: (v: any) => { v.execution.execution_id = "f".repeat(32); },
  foreign_endpoint: (v: any) => { v.endpoint = "agent:5002"; },
  foreign_method: (v: any) => { v.execution.method = "solve_beam_1d"; },
  schema: (v: any) => { v.execution.schema_version = "unknown"; },
  started: (v: any) => { v.execution.execution_started = true; },
  replay: (v: any) => { v.execution.automatic_replay_authorized = true; },
  missing_producer: (v: any) => { delete v.execution.process_instance_id; },
  bad_digest: (v: any) => { v.execution.request_digest = "no"; },
  transport: (v: any) => { v.gateway_runtime = "unknown"; },
  extra_response: (v: any) => { v.execution.response = { result: "already executed" }; },
  unknown_reason: (v: any) => { v.execution.reason = "corrupt_store"; },
  non_string_reason: (v: any) => { v.execution.reason = ["execution_unavailable"]; },
  missing_request_id: (v: any) => { v.execution.request_id = ""; },
})) {
  test(`untrusted rejection ${name} never erases the original pending identity`, async () => {
    const f = fixture(); f.setReply(body => { const value = rejection(body.execution_id); change(value); return value; });
    await assert.rejects(f.service.submit("axial_bar_1d", input, endpoint), /Submission unconfirmed/);
    assert.equal(f.entries.length, 1);
    await assert.rejects(f.service.submit("axial_bar_1d", input, endpoint), /recovery_required/);
    assert.equal(f.calls.length, 1);
  });
}
test("rejection cannot clear a pending identity after an authority change", async () => {
  const f = fixture(); f.setReply(body => { f.revoke(); return rejection(body.execution_id); });
  await assert.rejects(f.service.submit("axial_bar_1d", input, endpoint), /Submission unconfirmed/);
  assert.equal(f.entries.length, 1); assert.equal(f.calls.length, 1);
});
test("journal deletion failure reports cleanup pending without silently starting a replacement", async () => {
  const f = fixture(); f.setReply(body => rejection(body.execution_id));
  const service = createDurableDirectRunService({ ...f.options,
    journal: { ...f.journal, remove: async () => { throw new Error("disk write failed"); } } });
  await assert.rejects(service.submit("axial_bar_1d", input, endpoint), /journal_cleanup_pending: no execution started/);
  assert.equal(f.entries.length, 1);
  await assert.rejects(service.submit("axial_bar_1d", input, endpoint), /recovery_required/);
  assert.equal(f.calls.length, 1);
});
test("lost local reservation acknowledgement does not dispatch or delete an uncertain reservation", async () => {
  const f = fixture();
  const service = createDurableDirectRunService({ ...f.options, journal: { ...f.journal, reserve: async intent => {
    await f.journal.reserve(intent); throw new Error("local acknowledgement lost");
  } } });
  await assert.rejects(service.submit("axial_bar_1d", input, endpoint), /local acknowledgement lost/);
  assert.equal(f.calls.length, 0); assert.equal(f.entries.length, 1);
});
test("an existing reservation cannot be erased by another caller losing authority", async () => {
  const f = fixture(); await f.service.submit("axial_bar_1d", input, endpoint);
  const saved = structuredClone(f.entries);
  const service = createDurableDirectRunService({ ...f.options, journal: { ...f.journal, reserve: async intent => {
    const reserved = await f.journal.reserve(intent); f.revoke(); return reserved;
  } } });
  await assert.rejects(service.submit("axial_bar_1d", input, endpoint), /recovery_required/);
  assert.deepEqual(f.entries, saved); assert.equal(f.calls.length, 1);
});
test("an inspection response can never substitute a transient rejection for a terminal receipt", async () => {
  const f = fixture(); const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
  f.setReply(body => rejection(body.execution_id));
  await assert.rejects(f.service.inspect(handle), /mismatched/);
  assert.equal(f.entries.length, 1);
});

for (const field of ["code", "message"]) {
  for (const blank of ["", " ", "\t\r\n", "\u2003"]) {
    test(`blank terminal ${field} ${JSON.stringify(blank)} cannot clear a recovery journal`, async () => {
      const f = fixture();
      const { handle } = await f.service.submit("axial_bar_1d", input, endpoint);
      const malformed: any = report(handle.execution_id, "failed");
      malformed.execution.response.error[field] = blank;
      f.setReply(() => malformed);
      await assert.rejects(f.service.inspect(handle), /Invalid or mismatched/);
      await assert.rejects(f.service.acknowledge(handle, malformed), /Invalid or mismatched/);
      const reopened = createDurableDirectRunService(f.options);
      assert.deepEqual(await reopened.pending(), [handle]);
      await assert.rejects(reopened.submit("axial_bar_1d", input, endpoint), /recovery_required/);
      assert.deepEqual(f.calls.map(c => c.path), ["/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect"]);
      assert.equal(f.entries.length, 1);
    });
  }
}

for (const failureCode of ["durable_admission_rejected", "durable_execution_start_failed", "durable_execution_panicked", "durable_response_unrecordable"]) {
test(`${failureCode} reports failure and releases the journal without observation or retry`, async () => {
  const f = fixture(); f.setReply(body => report(body.execution_id, "failed", failureCode));
  const { handle, envelope } = await f.service.submit<WorkbenchStudyResult>("axial_bar_1d", input, endpoint);
  const jobs: any[] = [], results: any[] = [];
  await assert.rejects(observeWorkbenchDurableRun({ handle, envelope, service: f.service, isCurrent: () => true,
    setJob: value => jobs.push(value), setResult: value => results.push(value), setMessage: () => {}, sleep: async () => assert.fail() }), new RegExp(failureCode));
  assert.deepEqual(jobs.map(j => j.status), ["failed"]); assert.deepEqual(results, [null]);
  assert.equal(f.entries.length, 0); assert.equal(f.calls.length, 1);
});

test(`lost ${failureCode} response is recoverable through its retained terminal receipt`, async () => {
  const f = fixture(); let first = true;
  f.setReply(body => { if (first) { first = false; throw new Error("reply lost"); } return report(body.execution_id, "failed", failureCode); });
  await assert.rejects(f.service.submit("axial_bar_1d", input, endpoint), /Submission unconfirmed/);
  const recovered = createDurableDirectRunService(f.options);
  const [handle] = await recovered.pending();
  await assert.rejects(recovered.submit("axial_bar_1d", input, endpoint), /recovery_required/);
  const receipt = await recovered.inspect(handle);
  assert.equal(receipt.execution.status, "failed");
  await recovered.acknowledge(handle, receipt);
  assert.deepEqual(await recovered.pending(), []);
  assert.deepEqual(f.calls.map(c => c.path), ["/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect"]);
});

test(`${failureCode} cleanup error is visible and a later exact inspection can finish cleanup`, async () => {
  const f = fixture(); f.setReply(body => report(body.execution_id, "failed", failureCode));
  let failRemoval = true;
  const journal = { ...f.journal, remove: async (intent: DirectExecutionIntent) => {
    if (failRemoval) throw new Error("disk unavailable");
    await f.journal.remove(intent);
  } };
  const service = createDurableDirectRunService({ ...f.options, journal });
  const { handle, envelope } = await service.submit<WorkbenchStudyResult>("axial_bar_1d", input, endpoint);
  await assert.rejects(observeWorkbenchDurableRun({ handle, envelope, service, isCurrent: () => true,
    setJob: () => {}, setResult: value => assert.equal(value, null), setMessage: () => {} }), new RegExp(`${failureCode}:.*journal_cleanup_pending`));
  assert.equal(f.entries.length, 1);
  failRemoval = false;
  const reopened = createDurableDirectRunService({ ...f.options, journal });
  await reopened.acknowledge(handle, await reopened.inspect(handle));
  assert.equal(f.entries.length, 0); assert.equal(f.calls.length, 2);
});
}
