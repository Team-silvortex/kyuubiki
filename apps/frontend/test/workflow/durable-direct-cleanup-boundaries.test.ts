import test from "node:test";
import assert from "node:assert/strict";
import { createDurableDirectRunService } from "@/lib/workbench/durable-direct-run";
import { createDurableDirectActions } from "@/lib/workbench/durable-direct-actions";
import { observeWorkbenchDurableRun } from "@/components/workbench/workbench-durable-run-observer";
import { checkpointDigest } from "@/lib/workbench/checkpoint-journal";
import type { DirectExecutionIntent, DirectExecutionJournal } from "@/lib/workbench/direct-execution-journal";
import type { DurableDirectEnvelope } from "@/lib/api/durable-direct-client";
import type { DurableDirectRecoveryHandle } from "@/lib/workbench/durable-direct-recovery";

function gate() {
  let release!: () => void;
  const promise = new Promise<void>(resolve => { release = resolve; });
  return { promise, release };
}

async function fixture() {
  const identity = "https://gateway.invalid/api/direct-mesh/tasks", scope = await checkpointDigest(identity);
  const intent: DirectExecutionIntent = { scope, endpoint: "agent:5001", execution_id: "a".repeat(32),
    store_id: "c".repeat(32), study_kind: "axial_bar_1d", created_at: 1 };
  const handle: DurableDirectRecoveryHandle = { schema_version: "kyuubiki.direct-execution-handle/v1",
    gateway_id: scope, endpoint: intent.endpoint, execution_id: intent.execution_id, store_id: intent.store_id, study_kind: "axial_bar_1d" };
  const receipt: DurableDirectEnvelope<any> = { endpoint: intent.endpoint, gateway_runtime: "rust-native", execution: {
    schema_version: "kyuubiki.agent-durable-execution/v1", execution_id: intent.execution_id, store_id: intent.store_id!,
    status: "completed", request_id: "request", request_digest: "b".repeat(64), process_instance_id: "process",
    method: "solve_bar_1d", automatic_replay_authorized: false, resumable: false,
    response: { rpc_version: 1, id: "request", ok: true, result: { displacement: 1 } },
  } };
  const started = gate(), release = gate(), entries = [intent];
  const state = { epoch: 0, caller: true, reads: 0, deletes: 0, inspections: 0 };
  const journal: DirectExecutionJournal = {
    async list() { state.reads++; return structuredClone(entries); },
    async reserve() { throw new Error("cleanup may not reserve another execution"); },
    async remove(_intent, assertCurrent?: () => void) {
      started.release(); await release.promise;
      assertCurrent?.();
      state.deletes++; entries.splice(0);
    },
  };
  const client = {
    async inspectDurableDirectExecution() { state.inspections++; return receipt; },
    async fetchDirectMeshAgents() { throw new Error("cleanup must not discover or migrate"); },
    async submitDurableDirectStudy() { throw new Error("cleanup must not resubmit"); },
    async cancelDurableDirectExecution() { throw new Error("cleanup must not cancel"); },
    async listDurableDirectExecutions() { throw new Error("cleanup must not adopt an index"); },
  };
  const service = createDurableDirectRunService({ journal, client: client as never, gatewayIdentity: () => identity,
    captureAuthority: () => { const epoch = state.epoch; return () => state.epoch === epoch; } });
  return { handle, receipt, state, entries, started, release, service, client };
}

for (const owner of ["authority", "caller"]) {
  test(`terminal cleanup checks ${owner} again at the journal deletion boundary`, async () => {
    const f = await fixture(), before = structuredClone(f.entries);
    const pending = f.service.acknowledge(f.handle, f.receipt, () => f.state.caller);
    await f.started.promise;
    if (owner === "authority") f.state.epoch++;
    else f.state.caller = false;
    f.release.release();
    await assert.rejects(pending, /authority_changed/);
    assert.deepEqual(f.entries, before); assert.equal(f.state.deletes, 0);
    f.state.caller = true;
    await f.service.acknowledge(f.handle, f.receipt, () => f.state.caller);
    assert.deepEqual(f.entries, []); assert.equal(f.state.deletes, 1);
    assert.equal(f.state.inspections, 0);
  });
}

for (const when of ["before entry", "during scope digest"]) {
  test(`terminal cleanup cannot rebase an expired caller ${when}`, async () => {
    const f = await fixture(); f.release.release();
    if (when === "before entry") f.state.caller = false;
    const pending = f.service.acknowledge(f.handle, f.receipt, () => f.state.caller);
    f.state.caller = false;
    await assert.rejects(pending, /authority_changed/);
    assert.equal(f.state.reads, 0); assert.equal(f.state.deletes, 0); assert.equal(f.entries.length, 1);
  });
}

test("an unreadable caller guard fails cleanup closed without touching the journal", async () => {
  const f = await fixture(); f.release.release();
  await assert.rejects(f.service.acknowledge(f.handle, f.receipt, () => { throw new Error("unreadable caller"); }), /unreadable caller/);
  assert.equal(f.state.reads, 0); assert.equal(f.state.deletes, 0); assert.equal(f.entries.length, 1);
});

test("ordinary observation carries its original owner through terminal journal cleanup", async () => {
  const f = await fixture(), messages: string[] = [], results: unknown[] = [];
  const pending = observeWorkbenchDurableRun({ handle: f.handle, envelope: f.receipt, service: f.service,
    isCurrent: () => f.state.caller, setJob: () => {}, setResult: result => results.push(result), setMessage: value => messages.push(value) });
  await f.started.promise;
  assert.equal(results.length, 1, "the terminal result was published while the caller still owned observation");
  f.state.caller = false; f.release.release();
  await assert.rejects(pending, /superseded/);
  assert.equal(f.entries.length, 1); assert.equal(f.state.deletes, 0);
  assert.deepEqual(messages, []); assert.equal(results.length, 1);
});

test("manual recovery carries its original owner through terminal journal cleanup", async () => {
  const f = await fixture();
  const actions = createDurableDirectActions(f.client as never, () => () => f.state.caller, f.service);
  const pending = actions("directExecution/inspect", { endpoint: f.handle.endpoint, executionId: f.handle.execution_id });
  await f.started.promise;
  f.state.caller = false; f.release.release();
  await assert.rejects(pending, /authority changed/);
  assert.equal(f.entries.length, 1); assert.equal(f.state.deletes, 0); assert.equal(f.state.inspections, 1);
  f.state.caller = true;
  const recovered = await actions("directExecution/inspect", { endpoint: f.handle.endpoint, executionId: f.handle.execution_id });
  assert.equal(recovered?.journal_cleanup_pending, undefined);
  assert.equal(f.entries.length, 0); assert.equal(f.state.inspections, 2);
});
