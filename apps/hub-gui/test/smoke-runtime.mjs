import test from "node:test";
import assert from "node:assert/strict";
import { assertMatches } from "../../desktop-shared/test/smoke-test-helpers.mjs";
import {
  HUB_APP_RUNTIME_PATTERNS,
  HUB_MODULE_PATTERNS,
  read,
} from "./smoke-fixtures.mjs";

test("hub shell registers section switching behavior", () => {
  const js = read("ui/app.js");
  const actionRunner = read("ui/hub-action-runner.js");
  const assistantAuditStore = read("ui/hub-assistant-audit.js");
  const assistantAudit = read("ui/hub-assistant-audit-panel.js");
  const assistantLocal = read("ui/hub-assistant-local.js");
  const assistantPanel = read("ui/hub-assistant-panel.js");
  const appEvents = read("ui/hub-app-events.js");
  const bridge = read("ui/shared/tauri-bridge.js");
  const elements = read("ui/hub-elements.js");
  const libraryCopy = read("ui/hub-library-copy.js");
  const localizedShell = read("ui/hub-localized-shell.js");
  const networkContext = read("ui/hub-network-context.js");
  const operatorErrors = read("ui/hub-operator-errors.js");
  const outputPanel = read("ui/hub-output-panel.js");
  const projectBundles = read("ui/hub-project-bundles.js");
  const projectHistory = read("ui/hub-project-history.js");
  const projectHistoryPanel = read("ui/hub-project-history-panel.js");
  const runtimeHelpers = read("ui/hub-runtime-helpers.js");
  const runtimeLogController = read("ui/hub-runtime-log-controller.js");
  const runtimePanel = read("ui/hub-runtime-panel.js");
  const shellPanel = read("ui/hub-shell-panel.js");
  const state = read("ui/hub-state.js");
  const workflowCatalog = read("ui/hub-workflow-catalog.js");
  const workflowPanel = read("ui/hub-workflow-panel.js");
  const workloadAdapter = read("ui/hub-workload-adapter.js");
  const assistantEngine = read("ui/hub-assistant-engine.js");
  const englishCopy = read("ui/hub-i18n-en.js");
  const assistantI18n = read("ui/hub-i18n-assistant.js");
  const homeCopy = read("ui/hub-home-copy.js");
  const recentActions = read("ui/hub-recent-actions.js");
  const workloadLibrary = read("ui/hub-workload-library.js");
  const workloadRuntime = read("ui/hub-workload-runtime.js");
  const workloadActions = read("ui/hub-workload-actions.js");
  const runtimeActions = read("ui/hub-runtime-actions.js");
  const projectActions = read("ui/hub-project-actions.js");
  const desktopActions = read("ui/hub-desktop-actions.js");
  const startupPhases = read("ui/hub-startup-phases.js");
  const appRuntimeSource = [
    js,
    startupPhases,
    actionRunner,
    assistantAuditStore,
    assistantAudit,
    assistantEngine,
    assistantI18n,
    englishCopy,
    assistantLocal,
    assistantPanel,
    appEvents,
    elements,
    libraryCopy,
    localizedShell,
    networkContext,
    operatorErrors,
    outputPanel,
    projectBundles,
    projectActions,
    projectHistory,
    projectHistoryPanel,
    recentActions,
    runtimeActions,
    runtimeHelpers,
    runtimeLogController,
    runtimePanel,
    shellPanel,
    state,
    workflowCatalog,
    workflowPanel,
    workloadActions,
    workloadAdapter,
    workloadLibrary,
    workloadRuntime,
    desktopActions,
    homeCopy,
  ].join("\n");

  assertMatches(appRuntimeSource, HUB_APP_RUNTIME_PATTERNS);
  assertMatches(bridge, HUB_MODULE_PATTERNS.bridge);
  assertMatches(projectBundles, HUB_MODULE_PATTERNS.projectBundles);
  assert.doesNotMatch(projectHistoryPanel, /button\.innerHTML/);
  assert.match(projectHistoryPanel, /titleElement\.textContent = title/);
  assert.match(projectHistoryPanel, /detailsElement\.textContent = details/);
  assert.match(projectHistoryPanel, /badge\.textContent = entry\.status \|\| "idle"/);
  assertMatches(runtimeHelpers, HUB_MODULE_PATTERNS.runtimeHelpers);
  assertMatches(projectActions, HUB_MODULE_PATTERNS.projectActions);
  assertMatches(runtimeActions, HUB_MODULE_PATTERNS.runtimeActions);
  assertMatches(desktopActions, HUB_MODULE_PATTERNS.desktopActions);
  assertMatches(assistantEngine, HUB_MODULE_PATTERNS.assistantEngine);
  assertMatches(assistantI18n, HUB_MODULE_PATTERNS.assistantI18n);
  assertMatches(homeCopy, HUB_MODULE_PATTERNS.homeCopy);
  assertMatches(recentActions, HUB_MODULE_PATTERNS.recentActions);
  assertMatches(workloadLibrary, HUB_MODULE_PATTERNS.workloadLibrary);
  assertMatches(workloadRuntime, HUB_MODULE_PATTERNS.workloadRuntime);
  assertMatches(workloadActions, HUB_MODULE_PATTERNS.workloadActions);
});
