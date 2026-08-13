import test from "node:test";
import assert from "node:assert/strict";
import { assertMatches } from "../../desktop-shared/test/smoke-test-helpers.mjs";
import { suggestWorkflowCatalogEntries } from "../ui/hub-workflow-catalog.js";
import { runHubProjectAction } from "../ui/hub-project-actions.js";
import {
  buildProjectCliCommand,
  buildPythonMacroStub,
  rememberProjectBundleAction,
} from "../ui/hub-project-history.js";
import {
  HUB_INFORMATION_ARCHITECTURE_PATTERNS,
  HUB_PLATFORM_HELPER_PATTERNS,
  read,
} from "./smoke-fixtures.mjs";

test("hub shell defines a least-privilege main-window capability", () => {
  const tauriConfig = JSON.parse(read("src-tauri/tauri.conf.json"));
  const capability = JSON.parse(read("src-tauri/capabilities/main.json"));
  const permissions = read("src-tauri/permissions/hub.toml");

  assert.equal(tauriConfig.app.windows[0]?.label, "main");
  assert.equal(capability.identifier, "main");
  assert.deepEqual(capability.windows, ["main"]);
  assert.ok(Array.isArray(capability.permissions));
  assert.ok(capability.permissions.includes("core:default"));
  assert.ok(capability.permissions.includes("allow-guarded-mutation-action"));
  assert.ok(capability.permissions.includes("allow-service-status"));
  assert.ok(capability.permissions.includes("allow-project-bundle-inspect"));
  assert.ok(capability.permissions.includes("allow-hub-environment"));
  assert.ok(capability.permissions.includes("allow-hub-regression-gate-report"));
  assert.match(permissions, /identifier = "allow-service-status"/);
  assert.match(permissions, /commands\.allow = \["service_status"\]/);
  assert.match(permissions, /identifier = "allow-guarded-mutation-action"/);
  assert.match(permissions, /identifier = "allow-hub-regression-gate-report"/);
  assert.match(permissions, /commands\.allow = \["hub_regression_gate_report"\]/);
});

test("hub shell exposes the desktop information architecture", () => {
  const shellSource = [
    read("ui/index.html"),
    read("ui/hub-static-partials.js"),
    read("ui/hub-home-copy.js"),
  ].join("\n");
  assertMatches(shellSource, HUB_INFORMATION_ARCHITECTURE_PATTERNS);
});

test("hub shell normalizes host platform through shared desktop helpers", () => {
  const js = read("ui/app.js");
  const state = read("ui/hub-state.js");
  const toolsPlatformLabel = read("ui/hub-tools-platform-label.js");
  const guidesPanel = read("ui/hub-guides-panel.js");
  const platform = read("ui/shared/platform.js");
  const shellSource = [js, state, toolsPlatformLabel, guidesPanel].join("\n");

  assertMatches(shellSource, HUB_PLATFORM_HELPER_PATTERNS);
  assert.doesNotMatch(js, /hostPlatform:\s*"macos"/);
  assert.match(platform, /export function normalizeDesktopPlatform/);
  assert.doesNotMatch(platform, /desktop-shared\/ui\/platform\.js/);
});

test("hub assistant secret storage is cleanup-only", () => {
  const storage = read("ui/hub-storage.js");

  assert.match(storage, /sessionStorage\.removeItem\(HUB_ASSISTANT_LEGACY_SECRETS_KEY\)/);
  assert.doesNotMatch(storage, /sessionStorage\.getItem\(HUB_ASSISTANT_LEGACY_SECRETS_KEY\)/);
  assert.doesNotMatch(storage, /sessionStorage\.setItem\(HUB_ASSISTANT_LEGACY_SECRETS_KEY/);
  assert.doesNotMatch(storage, /localStorage\.(?:getItem|setItem)\(HUB_ASSISTANT_LEGACY_SECRETS_KEY/);
});

test("hub workflow catalog suggestions rank the closest workflow first", () => {
  const entries = [
    {
      id: "workflow.bridge-thermal-export",
      name: "Bridge Thermal Export",
      summary: "Bridge thermal diagnostics into an export artifact.",
      version: "v1",
      entry_inputs: { thermal_model: {} },
      output_artifacts: ["thermal_summary.json"],
    },
    {
      id: "workflow.bridge-mechanical-report",
      name: "Bridge Mechanical Report",
      summary: "Bridge structural diagnostics into a report artifact.",
      version: "v1",
      entry_inputs: { frame_model: {} },
      output_artifacts: ["mechanical_report.md"],
    },
    {
      id: "workflow.summary-bundle",
      name: "Summary Bundle",
      summary: "Thermal bridge export bundle.",
      version: "v2",
      entry_inputs: { heat_model: {} },
      output_artifacts: ["thermal_export.json"],
    },
  ];

  const suggestions = suggestWorkflowCatalogEntries(entries, "bridge thermal export");
  assert.equal(suggestions.length, 2);
  assert.equal(suggestions[0].entry.id, "workflow.bridge-thermal-export");
  assert.deepEqual(suggestions[0].matchedTerms, ["bridge", "thermal", "export"]);
  assert.ok(suggestions[0].score > suggestions[1].score);
});

test("hub workflow catalog suggestions return empty for unmatched queries", () => {
  const entries = [
    {
      id: "workflow.bridge-thermal-export",
      name: "Bridge Thermal Export",
      summary: "Bridge thermal diagnostics into an export artifact.",
      version: "v1",
      entry_inputs: { thermal_model: {} },
      output_artifacts: ["thermal_summary.json"],
    },
  ];

  const suggestions = suggestWorkflowCatalogEntries(entries, "electrostatic mesh");
  assert.deepEqual(suggestions, []);
});

test("hub project history records actions without hidden storage dependencies", () => {
  const previous = {
    action: "project create",
    bundlePath: "/tmp/Research.kyuubiki",
    comparePath: "",
    outputPath: "",
    status: "ok",
    note: "older result",
    executedAt: "2026-07-26T00:00:00.000Z",
    pinned: true,
    favoriteLabel: "Research seed",
  };
  const actions = rememberProjectBundleAction(
    [previous],
    "project create",
    {
      bundlePath: "/tmp/Research.kyuubiki",
      status: "ok",
      note: "created",
      executedAt: "2026-07-27T00:00:00.000Z",
    },
  );

  assert.equal(actions.length, 1);
  assert.equal(actions[0].note, "created");
  assert.equal(actions[0].pinned, true);
  assert.equal(actions[0].favoriteLabel, "Research seed");
});

test("hub new-bundle action stores the native path as active context", async () => {
  let activePath = "";
  let actionOptions;
  const options = {
    currentProjectBundlePayload: () => ({ path: "" }),
    runProjectBundleAction: async (value) => {
      actionOptions = value;
      const rendered = value.successOutput(
        JSON.stringify({ created: true, path: "/tmp/Research.kyuubiki" }),
      );
      assert.match(rendered, /Research\.kyuubiki/);
    },
    setProjectBundlePath: (value) => {
      activePath = value;
    },
    setProjectBundleOutput: () => {},
  };

  assert.equal(await runHubProjectAction("project-create", options), true);
  assert.equal(actionOptions.command, "guarded_mutation_action");
  assert.deepEqual(actionOptions.payload, { path: "" });
  assert.equal(activePath, "/tmp/Research.kyuubiki");
});

test("hub new-bundle history exports only implemented automation routes", () => {
  const entry = {
    action: "project create",
    bundlePath: "/tmp/Research.kyuubiki",
    favoriteLabel: "Research seed",
  };

  assert.equal(buildProjectCliCommand(entry), "");
  assert.match(buildPythonMacroStub(entry), /hub\/projectCreate/);
  assert.match(buildPythonMacroStub(entry), /Research\.kyuubiki/);
});
