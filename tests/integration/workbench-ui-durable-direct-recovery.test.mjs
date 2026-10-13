import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const endpoint = "127.0.0.1:5001";
const entry = { schema_version: "kyuubiki.agent-durable-execution/v1", execution_id: "a".repeat(32),
  request_id: "request", request_digest: "b".repeat(64), process_instance_id: "original-process", method: "solve_bar_1d" };

async function setupRun(page, loseReply = false) {
  const calls = [], state = { id: "", status: "running", input: null };
  await page.route("**/api/direct-mesh/agents", route => route.fulfill({ json: { mode: "direct_mesh_gui", discovery: "manual", endpoint_count: 1,
    agents: [{ id: "agent", host: "127.0.0.1", port: 5001, descriptor: {
      program: "kyuubiki-cli", role: "solver", runtime: { runtime_mode: "standalone", headless: true, peers: [] },
      protocol: { name: "kyuubiki.solver-rpc/v1", rpc_version: 1, transport: { kind: "tcp" }, methods: ["submit_durable_agent_task", "fetch_durable_agent_execution"] },
      capabilities: [], deployment_modes: ["local"], durable_execution: { schema_version: entry.schema_version, enabled: true },
    } }] } }));
  await page.route(/\/api\/direct-mesh\/tasks(?:\/inspect)?$/, async route => {
    const body = route.request().postDataJSON(), path = new URL(route.request().url()).pathname;
    calls.push({ path, body });
    assert.deepEqual(body.endpoints, [endpoint]);
    if (path.endsWith("/tasks")) {
      state.id = body.execution_id; state.input = body.input;
      if (loseReply) return route.abort("failed");
    } else assert.equal(body.execution_id, state.id);
    const result = { tip_displacement: 0, reaction_force: 0, max_displacement: 0, max_stress: 0,
      nodes: [{ index: 0, x: 0, displacement: 0 }, { index: 1, x: 1, displacement: 0 }],
      elements: [{ index: 0, x1: 0, x2: 1, strain: 0, stress: 0, axial_force: 0 }], input: state.input };
    await route.fulfill({ json: { endpoint, gateway_runtime: "rust-native", execution: {
      ...entry, execution_id: state.id, status: state.status, automatic_replay_authorized: false, resumable: false,
      response: state.status === "completed" ? { rpc_version: 1, id: entry.request_id, ok: true, result } : null,
    } } });
  });
  await openWorkbench(page);
  await invoke(page, "settings/patch", { language: "en", frontendRuntimeMode: "direct_mesh_gui", directMeshEndpointsText: endpoint });
  await invoke(page, "nav/setStudyKind", { studyKind: "axial_bar_1d" });
  return { calls, state };
}

test("ordinary PWDT run uses durable single-Agent submission and applies a terminal result", { timeout: 120_000 }, async () => {
  await usingWorkbench(async page => {
    const { calls, state } = await setupRun(page);
    state.status = "completed";
    const result = await invoke(page, "job/run");
    assert.equal(result.ok, true); assert.equal(result.completion, "terminal");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), true);
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
    assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
  });
});

test("lost run reply survives browser reload; a second run cannot submit and recovery does not replace the model", { timeout: 120_000 }, async () => {
  await usingWorkbench(async page => {
    const { calls, state } = await setupRun(page, true);
    await assert.rejects(invoke(page, "job/run"), /Submission unconfirmed/);
    const original = (await invoke(page, "directExecution/pending", { endpoint })).handles[0];
    assert.equal(original.execution_id, state.id);
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    const recovered = (await invoke(page, "directExecution/pending", { endpoint })).handles;
    assert.deepEqual(recovered, [original]);
    await assert.rejects(invoke(page, "job/run"), /recovery_required/);
    assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
    await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "runtime" }));
    const panel = page.locator("[data-durable-direct-recovery]");
    await panel.locator("summary").click();
    await panel.getByRole("button", { name: "Local Pending Executions" }).click();
    await panel.getByLabel("Local Pending Executions").selectOption(original.execution_id);
    state.status = "completed";
    await panel.getByRole("button", { name: "Inspect Receipt" }).click();
    await panel.getByRole("status").filter({ hasText: /^Completed$/ }).waitFor();
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
    assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
  });
});

async function setup(page) {
  page.setDefaultTimeout(10_000);
  const calls = [], state = { status: "running", failCancel: false };
  await page.route("**/api/direct-mesh/tasks/**", async route => {
    const path = new URL(route.request().url()).pathname;
    const body = route.request().postDataJSON();
    calls.push({ path, body });
    assert.deepEqual(body.endpoints, [endpoint]);
    const envelope = { endpoint, gateway_runtime: "rust-native" };
    if (path.endsWith("/list")) return route.fulfill({ json: { ...envelope, recovery: {
      schema_version: "kyuubiki.agent-durable-execution-index/v1", executions: [entry], next_cursor: null, automatic_replay_authorized: false,
    } } });
    if (path.endsWith("/inspect")) return route.fulfill({ json: { ...envelope, execution: {
      ...entry, status: state.status, automatic_replay_authorized: false, resumable: false,
      response: state.status === "failed" ? { rpc_version: 1, id: entry.request_id, ok: false, error: { code: "cancelled", message: "cancelled at safe point" } } : null,
    } } });
    assert.ok(path.endsWith("/cancel"), "no other operation may run during recovery");
    if (state.failCancel) return route.abort("failed");
    assert.equal(body.process_instance_id, entry.process_instance_id);
    assert.equal(body.request_digest, entry.request_digest);
    return route.fulfill({ json: { ...envelope, recovery: {
      schema_version: "kyuubiki.agent-durable-cancellation/v1", execution_id: entry.execution_id,
      process_instance_id: entry.process_instance_id, request_digest: entry.request_digest,
      status: "requested", cancel_registered: true, execution_terminal_confirmed: false, automatic_replay_authorized: false,
    } } });
  });
  await openWorkbench(page);
  await invoke(page, "settings/patch", { language: "en", frontendRuntimeMode: "direct_mesh_gui", directMeshEndpointsText: endpoint });
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "runtime" }));
  const panel = page.locator("[data-durable-direct-recovery]");
  await panel.locator("summary").click();
  return { panel, calls, state };
}

test("GUI durable recovery lists, inspects and cancels through PWDT without replacing the model", { timeout: 120_000 }, async () => {
  await usingWorkbench(async page => {
    const { panel, calls, state } = await setup(page);
    const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(calls.length, 0, "opening panel must not prefetch receipts");
    await panel.getByRole("button", { name: "Read First Page" }).click();
    await page.waitForFunction(() => !document.querySelector("[data-durable-direct-recovery] input")?.disabled);
    assert.equal(await panel.getByRole("status").textContent(), "Not inspected", `${await panel.innerText()} calls=${JSON.stringify(calls)}`);
    await panel.getByLabel("Select Execution").selectOption(entry.execution_id);
    assert.equal(await panel.getByRole("button", { name: "Request Cancellation" }).isDisabled(), true);
    await panel.getByRole("button", { name: "Inspect Receipt" }).click();
    await page.waitForFunction(() => !document.querySelector("[data-durable-direct-recovery] input")?.disabled);
    assert.equal(await panel.getByRole("status").textContent(), "Running", `${await panel.innerText()} calls=${JSON.stringify(calls)}`);
    await panel.getByRole("status").filter({ hasText: /^Running$/ }).waitFor();
    page.once("dialog", dialog => dialog.accept());
    await panel.getByRole("button", { name: "Request Cancellation" }).click();
    await panel.getByRole("status").filter({ hasText: /Cancellation registered/ }).waitFor();
    assert.equal(await panel.getByRole("button", { name: "Request Cancellation" }).isDisabled(), true);
    state.status = "failed";
    await panel.getByRole("button", { name: "Inspect Receipt" }).click();
    await panel.getByRole("status").filter({ hasText: /^Failed$/ }).waitFor();
    assert.deepEqual(calls.map(call => call.path.split("/").at(-1)), ["list", "inspect", "cancel", "inspect"]);
    const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(after.studyKind, before.studyKind);
    assert.equal(after.loadedModelName, before.loadedModelName);
    assert.equal(after.hasResult, before.hasResult);
    assert.equal(after.jobStatus, before.jobStatus);
  });
});

test("uncertain cancellation is not retried, and unknown receipts never enable cancel or replay", { timeout: 120_000 }, async () => {
  await usingWorkbench(async page => {
    const { panel, calls, state } = await setup(page);
    await panel.getByLabel("Execution ID", { exact: true }).fill(entry.execution_id);
    await panel.getByRole("button", { name: "Inspect Receipt" }).click();
    await panel.getByRole("status").filter({ hasText: /^Running$/ }).waitFor();
    state.failCancel = true;
    page.once("dialog", dialog => dialog.accept());
    await panel.getByRole("button", { name: "Request Cancellation" }).click();
    await page.waitForFunction(() => !document.querySelector("[data-durable-direct-recovery] input")?.disabled);
    assert.equal(calls.filter(call => call.path.endsWith("/cancel")).length, 1);
    assert.equal(await panel.getByRole("button", { name: "Request Cancellation" }).isDisabled(), true);
    state.status = "outcome_unknown";
    await panel.getByRole("button", { name: "Inspect Receipt" }).click();
    await panel.getByRole("status").filter({ hasText: /Outcome unknown/ }).waitFor();
    assert.equal(await panel.getByRole("button", { name: "Request Cancellation" }).isDisabled(), true);
    const receipt = await invoke(page, "directExecution/inspect", { endpoint, executionId: entry.execution_id });
    assert.equal(receipt.execution.status, "outcome_unknown");
    assert.equal(receipt.execution.automatic_replay_authorized, false);
    assert.ok(calls.every(call => /\/(inspect|cancel)$/.test(call.path)));
  });
});
