import test from "node:test";
import assert from "node:assert/strict";
import { createRuntimeApiClient } from "../../src/lib/api/runtime-client.ts";
import { createDurableDirectActions } from "../../src/lib/workbench/durable-direct-actions.ts";
import { validateDurableDirectCancellation, validateDurableDirectIndex } from "../../src/lib/api/durable-direct-controls.ts";
import { handleWorkbenchScriptStateAction } from "../../src/components/workbench/workbench-script-state-controller.ts";
import { getWorkbenchScriptActionDefinition } from "../../src/lib/scripting/workbench-script-runtime.ts";

const endpoint = "127.0.0.1:5001";
const entry = { schema_version: "kyuubiki.agent-durable-execution/v1", execution_id: "a".repeat(32),
  process_instance_id: "incarnation", request_id: "request", request_digest: "b".repeat(64), method: "solve_bar_1d" };
const target = { endpoint, execution_id: entry.execution_id, process_instance_id: entry.process_instance_id, request_digest: entry.request_digest };
function index(): any { return { gateway_runtime: "rust-native", endpoint, recovery: {
  schema_version: "kyuubiki.agent-durable-execution-index/v1", executions: [{ ...entry }], next_cursor: null, automatic_replay_authorized: false,
} }; }
function cancel(): any { return { gateway_runtime: "rust-native", endpoint, recovery: {
  schema_version: "kyuubiki.agent-durable-cancellation/v1", execution_id: entry.execution_id,
  process_instance_id: entry.process_instance_id, request_digest: entry.request_digest,
  status: "requested", cancel_registered: true, execution_terminal_confirmed: false, automatic_replay_authorized: false,
} }; }
function inspect(status = "running"): any { return { gateway_runtime: "rust-native", endpoint, execution: {
  ...entry, status, resumable: false, automatic_replay_authorized: false,
  response: status === "completed" ? { rpc_version: 1, id: entry.request_id, ok: true, result: { huge: "x".repeat(200_000) } } : null,
} }; }

test("recovery controls pin routes, page limits and original cancellation identity", async () => {
  const calls: any[] = [];
  const client = createRuntimeApiClient(async <T>(url: string, init?: RequestInit) => {
    calls.push([url, JSON.parse(init!.body as string)]);
    return (url.endsWith("list") ? index() : cancel()) as T;
  });
  await client.listDurableDirectExecutions(endpoint);
  const response = await client.cancelDurableDirectExecution(target);
  assert.equal(response.recovery.execution_terminal_confirmed, false);
  assert.deepEqual(calls, [
    ["/api/direct-mesh/tasks/list", { endpoints: [endpoint], limit: 24, after: null }],
    ["/api/direct-mesh/tasks/cancel", { endpoints: [endpoint], execution_id: target.execution_id,
      process_instance_id: target.process_instance_id, request_digest: target.request_digest }],
  ]);
});

for (const [label, mutate] of Object.entries({
  foreign: (v: any) => { v.endpoint = "127.0.0.1:5002"; },
  duplicate: (v: any) => { v.recovery.executions.push({ ...entry }); },
  unbounded: (v: any) => { v.recovery.executions = Array(25).fill(entry); },
  result_in_index: (v: any) => { v.recovery.executions[0].response = { result: {} }; },
  invalid_cursor: (v: any) => { v.recovery.next_cursor = "c".repeat(32); },
  missing_cursor: (v: any) => { delete v.recovery.next_cursor; },
  malformed_identity: (v: any) => { v.recovery.executions[0].request_digest = "bad"; },
  replay: (v: any) => { v.recovery.automatic_replay_authorized = true; },
})) {
  test(`recovery index rejects ${label}`, () => {
    const v = index(); mutate(v);
    assert.throws(() => validateDurableDirectIndex(v, endpoint, 24, null), /Invalid/);
  });
}

test("pagination rejects backwards pages and validates last ID cursor", () => {
  assert.throws(() => validateDurableDirectIndex(index(), endpoint, 1, entry.execution_id));
  const value = index(); value.recovery.next_cursor = entry.execution_id;
  assert.equal(validateDurableDirectIndex(value, endpoint, 1, null).recovery.next_cursor, entry.execution_id);
});

for (const [label, mutate] of Object.entries({
  replaced_process: (v: any) => { v.recovery.process_instance_id = "replacement"; },
  changed_inputs: (v: any) => { v.recovery.request_digest = "c".repeat(64); },
  false_terminal: (v: any) => { v.recovery.execution_terminal_confirmed = true; },
  contradictory: (v: any) => { v.recovery.cancel_registered = false; },
  job_cancel: (v: any) => { v.recovery.status = "cancelled"; },
})) {
  test(`durable cancellation rejects ${label}`, () => {
    const v = cancel(); mutate(v);
    assert.throws(() => validateDurableDirectCancellation(v, target), /Invalid/);
  });
}

test("invalid queries never reach the network and an uncertain cancel is never retried", async () => {
  let requests = 0;
  const client = createRuntimeApiClient(async <T>(): Promise<T> => { requests++; throw new Error("lost acknowledgement"); });
  for (const limit of [0, 65, NaN, 1.5]) await assert.rejects(client.listDurableDirectExecutions(endpoint, limit));
  await assert.rejects(client.listDurableDirectExecutions("http://agent"));
  await assert.rejects(client.cancelDurableDirectExecution({ ...target, request_digest: "" }));
  assert.equal(requests, 0);
  await assert.rejects(client.cancelDurableDirectExecution(target), /lost acknowledgement/);
  assert.equal(requests, 1);
});

test("GUI and PWDT use the same recovery action handler without Orchestra fetches or model replacement", async () => {
  const routes: string[] = [];
  const client = createRuntimeApiClient(async <T>(url: string) => { routes.push(url); return inspect("completed") as T; });
  const actions = createDurableDirectActions(client, () => () => true);
  const result = await handleWorkbenchScriptStateAction({
    action: "directExecution/inspect", payload: { endpoint, executionId: entry.execution_id }, durableDirectActions: actions,
    setResult: () => assert.fail("must not replace result"), runAnalysis: () => assert.fail("must not rerun"),
  } as any);
  assert.equal((result!.execution as any).has_result, true);
  assert.ok(JSON.stringify(result).length < 1500, "large result must not enter PWDT action history");
  assert.equal((result!.execution as any).response, undefined);
  assert.deepEqual(routes, ["/api/direct-mesh/tasks/inspect"]);
  assert.equal(getWorkbenchScriptActionDefinition("directExecution/cancel")?.requiresConfirmation, true);
});

test("late responses are fenced when gateway authority changes", async () => {
  let current = true;
  const client = createRuntimeApiClient(async <T>() => { current = false; return inspect() as T; });
  const actions = createDurableDirectActions(client, () => () => current);
  await assert.rejects(actions("directExecution/inspect", { endpoint, executionId: entry.execution_id }), /authority changed/);
});

test("unknown and not-found statuses stay explicit; unsupported actions and payloads cannot dispatch", async () => {
  let status = "outcome_unknown", calls = 0;
  const client = createRuntimeApiClient(async <T>() => { calls++; return inspect(status) as T; });
  const actions = createDurableDirectActions(client, () => () => true);
  for (status of ["outcome_unknown", "not_found"]) {
    const result = await actions("directExecution/inspect", { endpoint, executionId: entry.execution_id });
    assert.equal((result!.execution as any).status, status);
    assert.equal((result!.execution as any).has_result, false);
  }
  assert.equal(await actions("job/run", {}), null);
  await assert.rejects(actions("directExecution/inspect", { endpoint, executionId: entry.execution_id, retry: true }));
  assert.equal(calls, 2);
});
