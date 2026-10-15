import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const endpoint = "127.0.0.1:5001";
const storeId = "c".repeat(32);
const entry = { schema_version: "kyuubiki.agent-durable-execution/v1", execution_id: "a".repeat(32),
  request_id: "request", request_digest: "b".repeat(64), process_instance_id: "original-process", method: "solve_bar_1d" };

async function setupRun(page, loseReply = false) {
  const calls = [], state = { id: "", status: "running", input: null, reject: false, loseReply,
    loseInspection: false, busyStore: false, busyInspection: false,
    cancelGate: null, inspectionGate: null, loseCancelReply: false,
    receiptStoreId: storeId, failureCode: "durable_admission_rejected" };
  await page.route("**/api/direct-mesh/agents", route => route.fulfill({ json: { mode: "direct_mesh_gui", discovery: "manual", endpoint_count: 1,
    agents: [{ id: "agent", host: "127.0.0.1", port: 5001, descriptor: {
      program: "kyuubiki-cli", role: "solver", runtime: { runtime_mode: "standalone", headless: true, peers: [] },
      protocol: { name: "kyuubiki.solver-rpc/v1", rpc_version: 1, transport: { kind: "tcp" }, methods: ["submit_durable_agent_task", "fetch_durable_agent_execution"] },
      capabilities: [], deployment_modes: ["local"], durable_execution: { schema_version: entry.schema_version, enabled: true,
        ...(state.busyStore ? { available: false, unavailable_reason: "store_busy" } : { available: true, store_id: storeId }) },
    } }] } }));
  await page.route(/\/api\/direct-mesh\/tasks(?:\/(?:inspect|cancel))?$/, async route => {
    const body = route.request().postDataJSON(), path = new URL(route.request().url()).pathname;
    calls.push({ path, body });
    assert.deepEqual(body.endpoints, [endpoint]);
    assert.equal(body.expected_store_id, storeId);
    if (path.endsWith("/cancel")) {
      assert.equal(body.execution_id, state.id);
      assert.equal(body.process_instance_id, entry.process_instance_id);
      assert.equal(body.request_digest, entry.request_digest);
      await state.cancelGate;
      if (state.loseCancelReply) return route.abort("failed");
      return route.fulfill({ json: { endpoint, gateway_runtime: "rust-native", recovery: {
        schema_version: "kyuubiki.agent-durable-cancellation/v1", store_id: storeId, execution_id: state.id,
        process_instance_id: entry.process_instance_id, request_digest: entry.request_digest,
        status: "requested", cancel_registered: true, execution_terminal_confirmed: false, automatic_replay_authorized: false,
      } } });
    }
    if (path.endsWith("/tasks")) {
      state.id = body.execution_id; state.input = body.input;
      if (state.loseReply) return route.abort("failed");
      if (state.reject) return route.fulfill({ json: { endpoint, gateway_runtime: "rust-native", execution: {
        ...entry, schema_version: "kyuubiki.agent-durable-admission-rejection/v1", execution_id: state.id,
        store_id: storeId,
        status: "not_admitted", reason: "store_capacity", message: "execution store full",
        execution_started: false, automatic_replay_authorized: false,
      } } });
    } else {
      assert.equal(body.execution_id, state.id);
      if (state.loseInspection) { state.loseInspection = false; return route.abort("failed"); }
      if (state.busyInspection) return route.fulfill({ status: 502, json: { error: "execution store is busy",
        agent_error_code: "durable_execution_busy", endpoint, execution_id: state.id,
        automatic_retry: false, automatic_replay_authorized: false, gateway_runtime: "rust-native" } });
      await state.inspectionGate;
    }
    const result = { tip_displacement: 0, reaction_force: 0, max_displacement: 0, max_stress: 0,
      nodes: [{ index: 0, x: 0, displacement: 0 }, { index: 1, x: 1, displacement: 0 }],
      elements: [{ index: 0, x1: 0, x2: 1, strain: 0, stress: 0, axial_force: 0 }], input: state.input };
    await route.fulfill({ json: { endpoint, gateway_runtime: "rust-native", execution: {
      ...entry, execution_id: state.id, status: state.status, automatic_replay_authorized: false, resumable: false,
      store_id: state.receiptStoreId,
      response: state.status === "completed" ? { rpc_version: 1, id: entry.request_id, ok: true, result }
        : state.status === "failed" ? { rpc_version: 1, id: entry.request_id, ok: false, error: {
          code: state.failureCode, message: "execution failed", details: {
            automatic_replay_authorized: false,
            ...(state.failureCode === "durable_execution_panicked" ? {
              phase: "async_worker_unwind", execution_may_have_started: true,
              partial_result_discarded: true, external_effects_rolled_back: false,
            } : { reason: "execution_unavailable", execution_started: false }),
          },
        } } : null,
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

for (const [observation, modeChange] of [
  ["ordinary run", "switch"], ["manual recovery", "switch"],
  ["ordinary run", "roundtrip"], ["manual recovery", "roundtrip"],
]) {
  test(`a token-free mode ${modeChange} fences a late ${observation} receipt without losing its saved owner`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async page => {
      const { calls, state } = await setupRun(page, observation === "manual recovery");
      let original;
      if (observation === "manual recovery") {
        await assert.rejects(invoke(page, "job/run"), /Submission unconfirmed/);
        [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
      }
      let release;
      state.inspectionGate = new Promise(resolve => { release = resolve; });
      const received = page.waitForRequest(request => request.url().endsWith("/api/direct-mesh/tasks/inspect"), { timeout: 10_000 });
      const observing = (observation === "ordinary run" ? invoke(page, "job/run")
        : invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id }))
        .catch(error => ({ error: error.message }));
      try {
        await received;
        if (!original) [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
        assert.equal(original.execution_id, state.id);
        const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(before.hasResult, false);
        await invoke(page, "settings/patch", { frontendRuntimeMode: "orchestrated_gui" });
        await page.waitForFunction(() => JSON.parse(localStorage.getItem("kyuubiki-workbench-settings")).frontendRuntimeMode === "orchestrated_gui");
        if (modeChange === "roundtrip") {
          await invoke(page, "settings/patch", { frontendRuntimeMode: "direct_mesh_gui" });
          await page.waitForFunction(() => JSON.parse(localStorage.getItem("kyuubiki-workbench-settings")).frontendRuntimeMode === "direct_mesh_gui");
        }
        state.status = "completed";
        release();
        const outcome = await observing;
        assert.match(outcome.error, /authority.changed|superseded/i);
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(after.hasResult, false);
        assert.equal(after.studyKind, before.studyKind);
        assert.equal(after.loadedModelName, before.loadedModelName);
        await invoke(page, "settings/patch", { frontendRuntimeMode: "direct_mesh_gui" });
        assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
        await assert.rejects(invoke(page, "job/run"), /recovery_required/);
        assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
        assert.equal(calls.filter(call => call.path.endsWith("/cancel")).length, 0);
        const recovered = await invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id });
        assert.equal(recovered.execution.status, "completed");
        assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
        assert.equal(calls.filter(call => call.path.endsWith("/inspect")).length, 2);
      } finally {
        release();
        await observing;
      }
    });
  });
}

for (const [observation, change] of [
  ["ordinary run", "mode"], ["manual recovery", "mode"],
  ["ordinary run", "gateway"], ["manual recovery", "gateway"],
  ["ordinary run", "pinned gateway"],
  ["ordinary run", "unrelated preference"],
]) {
  test(`delivered cross-window ${change} roundtrip protects a pending ${observation} without replay`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async page => {
      const { calls, state } = await setupRun(page, observation === "manual recovery");
      const peer = await page.context().newPage(), peerErrors = [];
      peer.on("pageerror", error => peerErrors.push(error.message));
      let release, observing;
      try {
        const peerRun = await setupRun(peer);
        if (change === "gateway") await page.evaluate(() => {
          const url = new URL(location.href);
          const target = url.searchParams.get("kyuubikiApiBaseUrl");
          if (!target) throw new Error("fixture must start with its isolated backend target");
          localStorage.setItem("kyuubiki-workbench-api-base-url", target);
          url.searchParams.delete("kyuubikiApiBaseUrl");
          url.searchParams.delete("apiBaseUrl");
          history.replaceState(history.state, "", url);
        });
        await page.evaluate(change => {
          window.__testAuthorityEvents = 0;
          window.addEventListener("storage", event => {
            if (event.storageArea !== localStorage) return;
            if (change === "gateway" || change === "pinned gateway") {
              if (event.key === "kyuubiki-workbench-api-base-url" && event.oldValue !== event.newValue) window.__testAuthorityEvents++;
            } else if (event.key === "kyuubiki-workbench-settings") {
              const before = JSON.parse(event.oldValue || "{}"), after = JSON.parse(event.newValue || "{}");
              const key = change === "mode" ? "frontendRuntimeMode" : "crossWindowTestPreference";
              if (before[key] !== after[key]) window.__testAuthorityEvents++;
            }
          });
        }, change);
        let original;
        if (observation === "manual recovery") {
          await assert.rejects(invoke(page, "job/run"), /Submission unconfirmed/);
          [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
        }
        state.inspectionGate = new Promise(resolve => { release = resolve; });
        const received = page.waitForRequest(request => request.url().endsWith("/api/direct-mesh/tasks/inspect"), { timeout: 10_000 });
        observing = (observation === "ordinary run" ? invoke(page, "job/run")
          : invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id }))
          .catch(error => ({ error: error.message }));
        await received;
        if (!original) [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
        const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(before.hasResult, false);
        if (change === "mode") {
          for (const mode of ["orchestrated_gui", "direct_mesh_gui"]) {
            await invoke(peer, "settings/patch", { frontendRuntimeMode: mode });
            await peer.waitForFunction(mode => JSON.parse(localStorage.getItem("kyuubiki-workbench-settings")).frontendRuntimeMode === mode, mode);
          }
        } else await peer.evaluate(change => {
          const gateway = change === "gateway" || change === "pinned gateway";
          const key = gateway ? "kyuubiki-workbench-api-base-url" : "kyuubiki-workbench-settings";
          const original = localStorage.getItem(key);
          localStorage.setItem(key, gateway ? "https://foreign.invalid"
            : JSON.stringify({ ...JSON.parse(original || "{}"), crossWindowTestPreference: true }));
          if (original === null) localStorage.removeItem(key);
          else localStorage.setItem(key, original);
        }, change);
        // This qualifies delivered storage notifications, not an event-vs-reply transaction lock.
        await page.waitForFunction(() => window.__testAuthorityEvents >= 2);
        state.status = "completed";
        release();
        const outcome = await observing;
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        const invalidates = change === "mode" || change === "gateway";
        if (!invalidates) {
          assert.equal(outcome.completion, "terminal");
          assert.equal(after.hasResult, true);
        } else {
          assert.match(outcome.error, /authority.changed|superseded/i);
          assert.equal(after.hasResult, false);
          assert.equal(after.studyKind, before.studyKind);
          assert.equal(after.loadedModelName, before.loadedModelName);
          assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
          await assert.rejects(invoke(page, "job/run"), /recovery_required/);
          const recovered = await invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id });
          assert.equal(recovered.execution.status, "completed");
          assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
        }
        assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
        assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
        assert.equal(calls.filter(call => call.path.endsWith("/cancel")).length, 0);
        assert.equal(calls.filter(call => call.path.endsWith("/inspect")).length, invalidates ? 2 : 1);
        assert.deepEqual(peerRun.calls, [], "the other window must never submit, cancel, inspect, or replay the execution");
        assert.deepEqual(peerErrors, []);
      } finally {
        release?.();
        await observing;
        await peer.close();
      }
    });
  });
}

async function holdNextJournalCleanup(page, phase) {
  await page.evaluate(phase => {
    const gate = window.__testCleanupGate = { waiting: false, release: false, modeEvents: 0 };
    window.addEventListener("storage", event => {
      if (event.storageArea === localStorage && event.key === "kyuubiki-workbench-settings") {
        const before = JSON.parse(event.oldValue || "{}"), after = JSON.parse(event.newValue || "{}");
        if (before.frontendRuntimeMode !== after.frontendRuntimeMode) gate.modeEvents++;
      }
    });
    const hold = (store, resume = () => {}) => {
      gate.waiting = true;
      const pump = () => {
        const read = store.getAll(undefined, 1);
        read.onsuccess = () => { if (gate.release) resume(); else pump(); };
      };
      pump();
    };
    if (phase === "queued transaction") {
      const original = IDBDatabase.prototype.transaction;
      IDBDatabase.prototype.transaction = function(...args) {
        if (this.name === "kyuubiki-direct-execution" && args[1] === "readwrite") {
          IDBDatabase.prototype.transaction = original;
          // An earlier real readwrite transaction makes the cleanup transaction wait.
          const blocker = original.apply(this, args);
          hold(blocker.objectStore("pending"));
        }
        return original.apply(this, args);
      };
    } else {
      const original = IDBObjectStore.prototype.delete;
      IDBObjectStore.prototype.delete = function(...args) {
        const request = original.apply(this, args);
        if (this.transaction.db.name === "kyuubiki-direct-execution" && this.name === "pending") {
          IDBObjectStore.prototype.delete = original;
          // Deletion has run, but keep this real transaction active until the owner changes.
          Object.defineProperty(request, "onsuccess", { configurable: true, set: handler => {
            request.addEventListener("success", event => hold(this, () => handler.call(request, event)), { once: true });
          } });
        }
        return request;
      };
    }
  }, phase);
}

for (const observation of ["ordinary run", "manual recovery"]) {
  for (const phase of ["queued transaction", "delete success before commit"]) {
    test(`owner loss during ${phase} preserves the ${observation} journal reservation`, { timeout: 120_000 }, async () => {
      await usingWorkbench(async page => {
        const { calls, state } = await setupRun(page, observation === "manual recovery");
        const peer = await page.context().newPage(), peerErrors = [];
        peer.on("pageerror", error => peerErrors.push(error.message));
        let releaseReply, observing;
        try {
          const peerRun = await setupRun(peer);
          let original;
          if (observation === "manual recovery") {
            await assert.rejects(invoke(page, "job/run"), /Submission unconfirmed/);
            [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
          }
          state.inspectionGate = new Promise(resolve => { releaseReply = resolve; });
          const received = page.waitForRequest(request => request.url().endsWith("/api/direct-mesh/tasks/inspect"), { timeout: 10_000 });
          observing = (observation === "ordinary run" ? invoke(page, "job/run")
            : invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id }))
            .catch(error => ({ error: error.message }));
          await received;
          if (!original) [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
          await holdNextJournalCleanup(page, phase);
          state.status = "completed"; releaseReply();
          await page.waitForFunction(() => window.__testCleanupGate.waiting);
          const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
          // Ordinary execution publishes its valid result before attempting journal cleanup.
          assert.equal(before.hasResult, observation === "ordinary run");
          for (const mode of ["orchestrated_gui", "direct_mesh_gui"]) {
            await invoke(peer, "settings/patch", { frontendRuntimeMode: mode });
            await peer.waitForFunction(mode => JSON.parse(localStorage.getItem("kyuubiki-workbench-settings")).frontendRuntimeMode === mode, mode);
          }
          await page.waitForFunction(() => window.__testCleanupGate.modeEvents >= 2);
          await page.evaluate(() => { window.__testCleanupGate.release = true; });
          assert.match((await observing).error, /authority.changed|superseded/i);
          assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
          const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
          assert.equal(after.hasResult, before.hasResult); assert.equal(after.loadedModelName, before.loadedModelName);
          await assert.rejects(invoke(page, "job/run"), /recovery_required/);
          assert.equal((await invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id })).execution.status, "completed");
          assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
          assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
          assert.equal(calls.filter(call => call.path.endsWith("/inspect")).length, 2);
          assert.equal(calls.filter(call => call.path.endsWith("/cancel")).length, 0);
          assert.deepEqual(peerRun.calls, []); assert.deepEqual(peerErrors, []);
        } finally {
          releaseReply?.();
          await page.evaluate(() => { if (window.__testCleanupGate) window.__testCleanupGate.release = true; }).catch(() => {});
          await observing;
          await peer.close();
        }
      });
    });
  }
}

for (const terminal of ["completed", "failed"]) {
  for (const lostReply of [false, true]) {
    test(`late PWDT cancel ${lostReply ? "failure" : "reply"} preserves the same execution's ${terminal} result and feedback`, { timeout: 120_000 }, async () => {
      await usingWorkbench(async page => {
        const { calls, state } = await setupRun(page);
        let release;
        state.cancelGate = new Promise(resolve => { release = resolve; });
        state.loseCancelReply = lostReply;
        state.failureCode = "cancelled";
        const running = invoke(page, "job/run").catch(error => ({ error: error.message }));
        let cancelling;
        try {
          await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "solving" }));
          page.once("dialog", dialog => dialog.accept());
          const received = page.waitForRequest(request => request.url().endsWith("/api/direct-mesh/tasks/cancel"), { timeout: 5000 });
          cancelling = invoke(page, "job/cancel").catch(error => ({ error: error.message }));
          await received;
          state.status = terminal;
          const outcome = await running;
          if (terminal === "completed") assert.equal(outcome.completion, "terminal");
          else assert.match(outcome.error, /cancelled/);
          const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
          assert.equal(before.jobStatus, terminal);
          assert.equal(before.hasResult, terminal === "completed");
          assert.match(before.message, terminal === "completed" ? /direct:completed/ : /cancelled/);
          release();
          const cancellation = await cancelling;
          if (lostReply) assert.equal(typeof cancellation.error, "string");
          else assert.equal(cancellation.ok, true);
          const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
          assert.equal(after.message, before.message);
          assert.equal(after.jobStatus, before.jobStatus);
          assert.equal(after.hasResult, before.hasResult);
          assert.equal(calls.filter(call => call.path.endsWith("/cancel")).length, 1);
          assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
          assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
        } finally {
          release(); state.status = terminal;
          await Promise.allSettled([running, cancelling]);
        }
      });
    });
  }
}

test("lost cancellation reply survives reload until an exact terminal receipt is inspected", { timeout: 120_000 }, async () => {
  await usingWorkbench(async page => {
    const { calls, state } = await setupRun(page);
    state.loseInspection = true;
    await assert.rejects(invoke(page, "job/run"));
    const [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
    state.loseCancelReply = true;
    page.once("dialog", dialog => dialog.accept());
    await assert.rejects(invoke(page, "job/cancel"));
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
    await assert.rejects(invoke(page, "job/run"), /recovery_required/);
    state.status = "outcome_unknown";
    assert.equal((await invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id })).execution.status, "outcome_unknown");
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
    state.status = "failed"; state.failureCode = "cancelled";
    const recovered = await invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id });
    assert.equal(recovered.execution.error_code, "cancelled");
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
    assert.equal(calls.filter(call => call.path.endsWith("/cancel")).length, 1);
    assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
  });
});

test("busy recovery after reload keeps the original owner and never invites a replacement run", { timeout: 120_000 }, async () => {
  await usingWorkbench(async page => {
    const { calls, state } = await setupRun(page, true);
    await assert.rejects(invoke(page, "job/run"), /Submission unconfirmed/);
    const [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
    state.busyStore = true; state.busyInspection = true;
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
    await assert.rejects(invoke(page, "job/run"), /direct:store_busy/);
    assert.equal(calls.length, 1);
    await assert.rejects(invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id }), /execution store is busy/);
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
    assert.equal(calls.length, 2);
    state.busyStore = false; state.busyInspection = false; state.status = "completed";
    const recovered = await invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id });
    assert.equal(recovered.execution.status, "completed");
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
    assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
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

test("PWDT rejected run clears IndexedDB reservation and permits only a new explicit run after reload", { timeout: 120_000 }, async () => {
  await usingWorkbench(async page => {
    const { calls, state } = await setupRun(page);
    state.reject = true;
    await assert.rejects(invoke(page, "job/run"), /not_admitted/);
    const rejectedId = state.id;
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
    assert.equal(calls.length, 1, "rejection must not trigger retry or inspection");
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
    assert.equal(calls.length, 1);
    state.reject = false; state.status = "completed";
    const result = await invoke(page, "job/run");
    assert.equal(result.completion, "terminal");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), true);
    assert.notEqual(state.id, rejectedId); assert.equal(calls.length, 2);
  });
});

for (const fault of ["lost rejection reply", "interrupted journal cleanup"]) {
  test(`retained rejection recovers after ${fault} and reload without repeating computation`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async page => {
      const { calls, state } = await setupRun(page, fault === "lost rejection reply");
      state.status = "failed";
      if (fault === "interrupted journal cleanup") await page.evaluate(() => {
        const original = IDBObjectStore.prototype.delete;
        IDBObjectStore.prototype.delete = function(key) {
          if (this.transaction.db.name === "kyuubiki-direct-execution" && this.name === "pending") {
            throw new DOMException("injected journal write interruption", "UnknownError");
          }
          return original.call(this, key);
        };
      });
      await assert.rejects(invoke(page, "job/run"), fault === "lost rejection reply"
        ? /Submission unconfirmed/ : /durable_admission_rejected:.*journal_cleanup_pending/);
      const original = (await invoke(page, "directExecution/pending", { endpoint })).handles[0];
      assert.equal(original.execution_id, state.id); assert.equal(calls.length, 1);
      await page.reload({ waitUntil: "networkidle" });
      await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
      await assert.rejects(invoke(page, "job/run"), /recovery_required/);
      assert.equal(calls.length, 1);
      const recovered = await invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id });
      assert.equal(recovered.execution.status, "failed");
      assert.equal(recovered.execution.error_code, "durable_admission_rejected");
      assert.equal(recovered.execution.automatic_replay_authorized, false);
      assert.equal(recovered.journal_cleanup_pending, undefined);
      assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
      assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
      assert.deepEqual(calls.map(c => c.path), ["/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect"]);
      state.loseReply = false; state.status = "completed";
      assert.equal((await invoke(page, "job/run")).completion, "terminal");
      assert.notEqual(state.id, original.execution_id); assert.equal(calls.length, 3);
    });
  });
}

test("worker panic after detached observation survives reload without publishing or replaying a result", { timeout: 120_000 }, async () => {
  await usingWorkbench(async page => {
    const { calls, state } = await setupRun(page);
    state.loseInspection = true;
    await assert.rejects(invoke(page, "job/run"));
    const [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
    assert.equal(original.execution_id, state.id);
    assert.deepEqual(calls.map(c => c.path), ["/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect"]);
    state.status = "failed"; state.failureCode = "durable_execution_panicked";
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
    await assert.rejects(invoke(page, "job/run"), /recovery_required/);
    const recovered = await invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id });
    assert.equal(recovered.execution.status, "failed");
    assert.equal(recovered.execution.error_code, "durable_execution_panicked");
    assert.equal(recovered.execution.has_result, false);
    assert.equal(recovered.execution.automatic_replay_authorized, false);
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
    assert.equal(calls.filter(c => c.path.endsWith("/tasks")).length, 1);
    state.status = "completed";
    assert.equal((await invoke(page, "job/run")).completion, "terminal");
    assert.notEqual(state.id, original.execution_id);
    assert.equal(calls.filter(c => c.path.endsWith("/tasks")).length, 2);
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
      store_id: storeId,
      schema_version: "kyuubiki.agent-durable-execution-index/v1", executions: [entry], next_cursor: null, automatic_replay_authorized: false,
    } } });
    assert.equal(body.expected_store_id, storeId);
    if (path.endsWith("/inspect")) return route.fulfill({ json: { ...envelope, execution: {
      store_id: storeId,
      ...entry, status: state.status, automatic_replay_authorized: false, resumable: false,
      response: state.status === "failed" ? { rpc_version: 1, id: entry.request_id, ok: false, error: { code: "cancelled", message: "cancelled at safe point" } } : null,
    } } });
    assert.ok(path.endsWith("/cancel"), "no other operation may run during recovery");
    if (state.failCancel) return route.abort("failed");
    assert.equal(body.process_instance_id, entry.process_instance_id);
    assert.equal(body.request_digest, entry.request_digest);
    return route.fulfill({ json: { ...envelope, recovery: {
      store_id: storeId,
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
    await panel.getByRole("button", { name: "Read First Page" }).click();
    await panel.getByLabel("Select Execution").selectOption(entry.execution_id);
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
    const receipt = await invoke(page, "directExecution/inspect", { endpoint, executionId: entry.execution_id, storeId });
    assert.equal(receipt.execution.status, "outcome_unknown");
    assert.equal(receipt.execution.automatic_replay_authorized, false);
    assert.ok(calls.every(call => /\/(list|inspect|cancel)$/.test(call.path)));
  });
});

test("reloaded PWDT recovery rejects a foreign store receipt without clearing the original handle", { timeout: 120_000 }, async () => {
  await usingWorkbench(async page => {
    const { calls, state } = await setupRun(page, true);
    await assert.rejects(invoke(page, "job/run"), /Submission unconfirmed/);
    const [original] = (await invoke(page, "directExecution/pending", { endpoint })).handles;
    assert.equal(original.store_id, storeId);
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    state.receiptStoreId = "d".repeat(32); state.status = "completed";
    await assert.rejects(invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id }), /store identity mismatch/);
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, [original]);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
    await assert.rejects(invoke(page, "job/run"), /recovery_required/);
    assert.equal(calls.filter(call => call.path.endsWith("/tasks")).length, 1);
    state.receiptStoreId = storeId;
    assert.equal((await invoke(page, "directExecution/inspect", { endpoint, executionId: original.execution_id })).execution.status, "completed");
    assert.deepEqual((await invoke(page, "directExecution/pending", { endpoint })).handles, []);
  });
});
