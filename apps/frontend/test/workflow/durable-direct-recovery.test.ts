import test from "node:test";
import assert from "node:assert/strict";
import { createRuntimeApiClient } from "../../src/lib/api/runtime-client.ts";
import {
  DURABLE_DIRECT_SCHEMA, validateDurableDirectEnvelope,
  type DurableDirectHandle,
} from "../../src/lib/api/durable-direct-client.ts";
import {
  createDurableDirectRecovery, DurableDirectSubmissionUncertain,
  type DurableDirectRecoveryHandle,
} from "../../src/lib/workbench/durable-direct-recovery.ts";

const handle: DurableDirectHandle = { execution_id: "a".repeat(32), endpoint: "127.0.0.1:5001" };
const input = { length: 1, area: 1, youngs_modulus_gpa: 0.000001, elements: 2, tip_force: 10 };
function report(status = "running"): any {
  return { gateway_runtime: "rust-native", endpoint: handle.endpoint, execution: {
    schema_version: DURABLE_DIRECT_SCHEMA, execution_id: handle.execution_id, status,
    request_id: "original-request", request_digest: "b".repeat(64), process_instance_id: "producer",
    method: "solve_bar_1d", resumable: false, automatic_replay_authorized: false,
    response: status === "completed" ? { rpc_version: 1, id: "original-request", ok: true, result: { stress: 10 } }
      : status === "failed" ? { rpc_version: 1, id: "original-request", ok: false, error: { code: "cancelled", message: "cancelled" } } : null,
  } };
}

test("runtime durable API uses pinned native routes and preserves nonterminal state", async () => {
  const calls: Array<[string, any]> = [];
  const client = createRuntimeApiClient(async <T>(url: string, init?: RequestInit) => {
    calls.push([url, JSON.parse(init!.body as string)]);
    return report(url.endsWith("inspect") ? "completed" : "running") as T;
  });
  assert.equal((await client.submitDurableDirectStudy(handle, "axial_bar_1d", input)).execution.status, "running");
  assert.equal((await client.inspectDurableDirectExecution(handle)).execution.status, "completed");
  assert.deepEqual(calls.map(([url]) => url), ["/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect"]);
  assert.deepEqual(calls[1][1], { execution_id: handle.execution_id, endpoints: [handle.endpoint] });
  assert.equal(calls[0][1].input.youngs_modulus, 1000);
  assert.equal(calls[0][1].selection_mode, undefined);
});

for (const status of ["running", "completed", "failed", "outcome_unknown", "not_found"]) {
  test(`durable contract preserves ${status} without converting it to a job success`, () => {
    assert.equal(validateDurableDirectEnvelope(report(status), handle).execution.status, status);
  });
}

for (const [label, mutate] of Object.entries({
  endpoint: (v: any) => { v.endpoint = "127.0.0.1:5002"; },
  identity: (v: any) => { v.execution.execution_id = "c".repeat(32); },
  schema: (v: any) => { v.execution.schema_version = "other"; },
  replay: (v: any) => { v.execution.automatic_replay_authorized = true; },
  digest: (v: any) => { v.execution.request_digest = "bad"; },
  nested_identity: (v: any) => { v.execution.response.id = "foreign"; },
  absent_result: (v: any) => { v.execution.response.result = null; },
  contradictory_failure: (v: any) => { v.execution.response.ok = false; },
  false_running: (v: any) => { v.execution.status = "running"; },
  unknown_status: (v: any) => { v.execution.status = "accepted"; },
})) {
  test(`durable contract rejects ${label}`, () => {
    const value = report("completed"); mutate(value);
    assert.throws(() => validateDurableDirectEnvelope(value, handle), /mismatched/u);
  });
}

test("submission cannot start before handle storage completes, and input is snapshotted", async () => {
  const calls: any[] = [];
  let release!: () => void;
  const gate = new Promise<void>(resolve => { release = resolve; });
  const service = createDurableDirectRecovery({
    newId: () => handle.execution_id, gatewayId: () => "gateway-a",
    saveHandle: async value => { calls.push(value); await gate; },
    client: createRuntimeApiClient(async <T>(url: string, init?: RequestInit) => {
      calls.push([url, JSON.parse(init!.body as string)]); return report() as T;
    }),
  });
  const model = { ...input };
  const pending = service.submit("axial_bar_1d", model, handle.endpoint);
  model.tip_force = 999;
  assert.equal(calls.length, 1);
  assert.equal(calls[0].gateway_id, "gateway-a");
  assert.equal(calls[0].input, undefined, "recovery handles must not retain large models");
  release();
  const result = await pending;
  assert.equal(result.envelope.execution.status, "running");
  assert.equal(calls[1][1].input.tip_force, 10);
});

test("storage failure or a gateway change before send cannot dispatch", async () => {
  for (const changeGateway of [false, true]) {
    let gateway = "gateway-a";
    let requests = 0;
    const service = createDurableDirectRecovery({
      newId: () => handle.execution_id, gatewayId: () => gateway,
      saveHandle: async () => { if (changeGateway) gateway = "gateway-b"; else throw new Error("disk unavailable"); },
      client: createRuntimeApiClient(async <T>() => { requests++; return report() as T; }),
    });
    await assert.rejects(service.submit("axial_bar_1d", input, handle.endpoint), /disk unavailable|original gateway/u);
    assert.equal(requests, 0);
  }
});

test("lost admission reply exposes the saved handle and reconnect only inspects the original Agent", async () => {
  let saved!: DurableDirectRecoveryHandle;
  const calls: string[] = [];
  const client = createRuntimeApiClient(async <T>(url: string) => {
    calls.push(url);
    if (url === "/api/direct-mesh/tasks") throw new Error("response lost");
    return report("completed") as T;
  });
  const options = { client, gatewayId: () => "gateway-a", saveHandle: async (value: DurableDirectRecoveryHandle) => { saved = value; }, newId: () => handle.execution_id };
  await assert.rejects(createDurableDirectRecovery(options).submit("axial_bar_1d", input, handle.endpoint), error => {
    assert.ok(error instanceof DurableDirectSubmissionUncertain);
    assert.equal(error.handle, saved);
    return true;
  });
  const reloaded = createDurableDirectRecovery(options);
  assert.equal((await reloaded.inspect(JSON.parse(JSON.stringify(saved)))).execution.status, "completed");
  assert.deepEqual(calls, ["/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect"]);
});

test("uncertain and missing receipts remain inspect-only, including repeated reconnect", async () => {
  for (const status of ["not_found", "outcome_unknown", "failed"]) {
    const calls: string[] = [];
    const client = createRuntimeApiClient(async <T>(url: string) => { calls.push(url); return report(status) as T; });
    const service = createDurableDirectRecovery({ client, gatewayId: () => "a", saveHandle: async () => {} });
    const saved = { ...handle, schema_version: "kyuubiki.direct-execution-handle/v1", gateway_id: "a", study_kind: "axial_bar_1d" } as const;
    for (let i = 0; i < 2; i++) assert.equal((await service.inspect(saved)).execution.status, status);
    assert.deepEqual(calls, ["/api/direct-mesh/tasks/inspect", "/api/direct-mesh/tasks/inspect"]);
  }
});

test("changing gateways during inspection cannot publish the old response", async () => {
  let gateway = "a";
  const service = createDurableDirectRecovery({
    gatewayId: () => gateway, saveHandle: async () => {},
    client: createRuntimeApiClient(async <T>() => { gateway = "b"; return report("completed") as T; }),
  });
  await assert.rejects(service.inspect({ ...handle, schema_version: "kyuubiki.direct-execution-handle/v1", gateway_id: "a", study_kind: "axial_bar_1d" }), /original gateway/u);
});

test("malformed endpoints and IDs are rejected before any HTTP request", async () => {
  let requests = 0;
  const client = createRuntimeApiClient(async <T>() => { requests++; return report() as T; });
  for (const endpoint of ["", "agent:0", "agent:65536", "agent:05001", "a:1,b:2", "http://agent:1", "u@agent:1"]) {
    await assert.rejects(client.inspectDurableDirectExecution({ ...handle, endpoint }));
  }
  await assert.rejects(client.inspectDurableDirectExecution({ ...handle, execution_id: "../a" }));
  assert.equal(requests, 0);
});

test("changing a caller handle during a request cannot rebind response validation", async () => {
  const mutable = { ...handle };
  const client = createRuntimeApiClient(async <T>() => {
    mutable.execution_id = "d".repeat(32);
    mutable.endpoint = "127.0.0.1:5002";
    const foreign = report("completed");
    foreign.execution.execution_id = mutable.execution_id;
    foreign.endpoint = mutable.endpoint;
    return foreign as T;
  });
  await assert.rejects(client.inspectDurableDirectExecution(mutable), /mismatched/u);
});
