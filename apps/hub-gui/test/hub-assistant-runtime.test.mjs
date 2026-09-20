import assert from "node:assert/strict";
import test from "node:test";
import { assistantRuntimeModel } from "../ui/hub-assistant-runtime.js";
import { assistantRuntimeCopy, ASSISTANT_RUNTIME_TRANSLATIONS } from "../ui/hub-assistant-runtime-copy.js";
import { DESKTOP_LANGUAGE_LABELS } from "../../desktop-shared/ui/language-pack-loader.js";
import { refreshRuntimeStatusPanel } from "../ui/hub-runtime-helpers.js";
import { buildHubAssistantLocalCards, buildLocalGuideContext } from "../ui/hub-assistant-local.js";

test("assistant runtime reads structured states, not running words in paths or diagnostics", () => {
  const model = assistantRuntimeModel({
    rendered: "runtime-root: C:\\running\\installed\r\nfrontend: running (unmanaged pid)\r\n" +
      "frontend-build-command[npm]: development-only -> C:\\tools\\npm\r\n" +
      "lifecycle-policy: explicit-background (closing GUI does not stop services)",
    summary: { orchestrator_status: "stopped", frontend_status: "blocked",
      agents: [{ label: "agent[5001]", status: "starting" }, { label: "agent[5002]", status: "running" }] },
  });
  assert.equal(model.running, 1);
  assert.deepEqual(model.services.map(({ status }) => status), ["stopped", "blocked", "starting", "running"]);
  assert.deepEqual(model.fields, [
    { label: "runtime-root", value: "C:\\running\\installed" },
    { label: "frontend-build-command[npm]", value: "development-only -> C:\\tools\\npm" },
    { label: "lifecycle-policy", value: "explicit-background (closing GUI does not stop services)" },
  ]);
});

test("unknown, malformed, disabled and failed status never becomes running", () => {
  assert.equal(assistantRuntimeModel(null).pending, true);
  for (const summary of [null, [], "running", 3]) {
    const model = assistantRuntimeModel({ summary, rendered: "running" });
    assert.equal(model.available, false);
    assert.equal(model.pending, false);
  }
  const model = assistantRuntimeModel({ summary: { frontend_status: "disabled", orchestrator_status: "novel-state",
    agents: [null, 4, {}, { label: "worker", status: "installed" }] }, rendered: "" });
  assert.equal(model.running, 0);
  assert.deepEqual(model.services.map(({ status }) => status), ["unknown", "disabled", "unknown", "unknown"]);
  assert.deepEqual(assistantRuntimeModel({ summary: { frontend_status: "running" }, failed: true, rendered: "timeout" }).services, []);
});

test("assistant status labels cover every selectable desktop language", () => {
  for (const language of Object.keys(DESKTOP_LANGUAGE_LABELS)) {
    assert.ok(ASSISTANT_RUNTIME_TRANSLATIONS[language], language);
    assert.equal(Object.values(assistantRuntimeCopy(language)).length, 11);
    assert.ok(Object.values(assistantRuntimeCopy(language)).every((value) => value.trim()));
  }
  assert.equal(assistantRuntimeCopy("zh").configuration, "配置与路径");
});

test("a failed refresh replaces the prior assistant report rather than retaining healthy data", async () => {
  const reports = [];
  const report = { rendered: "frontend: running", summary: { frontend_status: "running" } };
  const options = { orchestratorBaseUrl: "", setRuntimeStatusOutput() {}, applyDesktopState() {},
    onRuntimeReport: (value) => reports.push(value) };
  await refreshRuntimeStatusPanel({ ...options, invokeTauri: async () => report });
  await refreshRuntimeStatusPanel({ ...options, invokeTauri: async () => { throw new Error("unreachable"); } });
  assert.equal(reports.length, 2);
  assert.deepEqual(reports[0], report);
  assert.equal(reports[1].summary, null);
  assert.equal(reports[1].failed, true);
  assert.match(reports[1].rendered, /unreachable/);
});

test("local guide and runtime summary agree without interpreting ready or healthy substrings", () => {
  const ready = assistantRuntimeModel({ rendered: "frontend: disabled by runtime configuration",
    summary: { frontend_status: "disabled", orchestrator_status: "running",
      agents: [{ label: "agent[5001]", status: "running" }] } }).ready;
  assert.equal(ready, true);
  for (const [runtimeReady, runtimeStatus] of [[ready, "native runtime report"], [false, "not ready / unhealthy"]]) {
    const options = { currentAssistantSnapshot: () => ({ runtimeReady, runtimeStatus }), hubDynamic: (key) => key };
    assert.equal(buildLocalGuideContext(options).runtimeReady, runtimeReady);
    assert.equal(buildHubAssistantLocalCards(options).some((card) => card.id === "start-local"), !runtimeReady);
  }
});
