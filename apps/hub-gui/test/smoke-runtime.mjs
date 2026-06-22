import test from "node:test";
import { assertMatches } from "../../desktop-shared/test/smoke-test-helpers.mjs";
import {
  HUB_APP_RUNTIME_PATTERNS,
  HUB_MODULE_PATTERNS,
  read,
} from "./smoke-fixtures.mjs";

test("hub shell registers section switching behavior", () => {
  const js = read("ui/app.js");
  const bridge = read("ui/shared/tauri-bridge.js");
  const projectBundles = read("ui/hub-project-bundles.js");
  const runtimeHelpers = read("ui/hub-runtime-helpers.js");
  const assistantEngine = read("ui/hub-assistant-engine.js");
  const assistantI18n = read("ui/hub-i18n-assistant.js");
  const homeCopy = read("ui/hub-home-copy.js");
  const recentActions = read("ui/hub-recent-actions.js");
  const workloadLibrary = read("ui/hub-workload-library.js");
  const workloadRuntime = read("ui/hub-workload-runtime.js");
  const workloadActions = read("ui/hub-workload-actions.js");
  const runtimeActions = read("ui/hub-runtime-actions.js");
  const projectActions = read("ui/hub-project-actions.js");
  const desktopActions = read("ui/hub-desktop-actions.js");

  assertMatches(js, HUB_APP_RUNTIME_PATTERNS);
  assertMatches(bridge, HUB_MODULE_PATTERNS.bridge);
  assertMatches(projectBundles, HUB_MODULE_PATTERNS.projectBundles);
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
