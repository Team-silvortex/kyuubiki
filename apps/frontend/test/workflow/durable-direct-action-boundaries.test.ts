import test from "node:test";
import assert from "node:assert/strict";
import { createRuntimeApiClient } from "../../src/lib/api/runtime-client.ts";
import { createDurableDirectActions } from "../../src/lib/workbench/durable-direct-actions.ts";
import type { DurableDirectRunService } from "../../src/lib/workbench/durable-direct-run.ts";

const endpoint = "127.0.0.1:5001", storeId = "c".repeat(32), executionId = "a".repeat(32);
const processInstanceId = "original-process", requestDigest = "b".repeat(64);
const payload = () => ({ endpoint, executionId, storeId });

function gate() {
  let release!: () => void;
  const promise = new Promise<void>(resolve => { release = resolve; });
  return { promise, release };
}

function fixture() {
  const pendingGate = gate(), responseGate = gate(), started = gate();
  const calls: Array<{ path: string; body: Record<string, any> }> = [];
  const state = { current: true, terminal: false, removed: 0, pendingReads: 0, pauseResponse: false };
  const saved = { endpoint, execution_id: executionId, store_id: storeId };
  const client = createRuntimeApiClient(async <T>(path: string, init?: RequestInit) => {
    const body = JSON.parse(String(init?.body)); calls.push({ path, body });
    started.release();
    if (state.pauseResponse) await responseGate.promise;
    const envelope = { gateway_runtime: "rust-native", endpoint: body.endpoints[0] };
    if (path.endsWith("/cancel")) return { ...envelope, recovery: {
      schema_version: "kyuubiki.agent-durable-cancellation/v1", store_id: body.expected_store_id,
      execution_id: body.execution_id, process_instance_id: body.process_instance_id, request_digest: body.request_digest,
      status: "requested", cancel_registered: true, execution_terminal_confirmed: false, automatic_replay_authorized: false,
    } } as T;
    return { ...envelope, execution: {
      schema_version: "kyuubiki.agent-durable-execution/v1", store_id: body.expected_store_id,
      execution_id: body.execution_id, process_instance_id: processInstanceId, request_digest: requestDigest,
      request_id: "original", method: "solve_bar_1d", status: state.terminal ? "completed" : "running",
      resumable: false, automatic_replay_authorized: false,
      response: state.terminal ? { rpc_version: 1, id: "original", ok: true, result: { displacement: 1 } } : null,
    } } as T;
  });
  const service = {
    async pending() { state.pendingReads++; await pendingGate.promise; return [saved]; },
    async acknowledge() { state.removed++; },
  } as unknown as DurableDirectRunService;
  return { calls, state, pendingGate, responseGate, started, saved,
    actions: createDurableDirectActions(client, () => () => state.current, service) };
}

for (const action of ["inspect", "cancel"]) {
  test(`recovery ${action} snapshots caller fields before the pending-journal await`, async () => {
    const f = fixture();
    const input = { ...payload(), ...(action === "cancel" ? { processInstanceId, requestDigest } : {}) };
    const result = f.actions(`directExecution/${action}`, input);
    Object.assign(input, { endpoint: "127.0.0.1:5002", executionId: "d".repeat(32), storeId: "e".repeat(32),
      ...(action === "cancel" ? { processInstanceId: "replacement-process", requestDigest: "f".repeat(64) } : {}) });
    f.pendingGate.release();
    assert.equal((await result)?.ok, true);
    assert.equal(f.calls.length, 1);
    assert.deepEqual(f.calls[0].body.endpoints, [endpoint]);
    assert.equal(f.calls[0].body.execution_id, executionId);
    assert.equal(f.calls[0].body.expected_store_id, storeId);
    if (action === "cancel") {
      assert.equal(f.calls[0].body.process_instance_id, processInstanceId);
      assert.equal(f.calls[0].body.request_digest, requestDigest);
    }
    assert.equal(f.state.removed, 0);
  });

  test(`authority change during ${action} journal lookup prevents dispatch`, async () => {
    const f = fixture();
    const result = f.actions(`directExecution/${action}`, { ...payload(),
      ...(action === "cancel" ? { processInstanceId, requestDigest } : {}) });
    f.state.current = false; f.pendingGate.release();
    await assert.rejects(result, /authority changed/);
    assert.equal(f.calls.length, 0); assert.equal(f.state.removed, 0);
  });
}

for (const terminal of [false, true]) {
  test(`a late ${terminal ? "terminal" : "running"} recovery response cannot publish or clear a retired context`, { timeout: 5000 }, async () => {
    const f = fixture(); f.pendingGate.release(); f.state.terminal = terminal; f.state.pauseResponse = true;
    const result = f.actions("directExecution/inspect", payload());
    await f.started.promise;
    assert.equal(f.calls.length, 1);
    f.state.current = false; f.responseGate.release();
    await assert.rejects(result, /authority changed/);
    assert.equal(f.calls.length, 1); assert.equal(f.state.removed, 0);
  });
}

test("fresh explicit recovery after a rejected late response can clean only its original handle", { timeout: 5000 }, async () => {
  const f = fixture(); f.pendingGate.release(); f.state.terminal = true; f.state.pauseResponse = true;
  const old = f.actions("directExecution/inspect", payload());
  await f.started.promise;
  f.state.current = false; f.responseGate.release();
  await assert.rejects(old, /authority changed/);
  assert.equal(f.state.removed, 0);
  f.state.current = true;
  const recovered = await f.actions("directExecution/inspect", payload());
  assert.equal((recovered?.execution as { status: string }).status, "completed");
  assert.equal(f.state.removed, 1);
  assert.equal(f.calls.length, 2);
  assert.ok(f.calls.every(call => call.path.endsWith("/inspect") && call.body.execution_id === executionId));
});
