import test from "node:test";
import assert from "node:assert/strict";

import {
  buildWorkbenchUxGuardrailSummary,
  localizeWorkbenchUxGuardrailSummary,
  type WorkbenchUxGuardrailInput,
} from "../../src/components/workbench/workbench-ux-guardrails.ts";
import { resolveWorkbenchBaseCopy } from "../../src/components/workbench/workbench-copy.ts";

const BASE_INPUT: WorkbenchUxGuardrailInput = {
  frontendRuntimeMode: "orchestrated_gui",
  healthStatus: "ok",
  protocolOnline: true,
  watchdogOnline: true,
  controlPlaneApiToken: "control-token",
  clusterApiToken: "",
  directMeshApiToken: "",
  directMeshEndpointsText: "",
  selectedProjectId: "project-a",
  selectedVersionId: "version-a",
  languagePackCount: 1,
};

test("orchestrated UX guardrails block when control plane is offline", () => {
  const summary = buildWorkbenchUxGuardrailSummary({
    ...BASE_INPUT,
    healthStatus: "offline",
  });

  assert.equal(summary.tone, "block");
  assert.equal(summary.blockedActionCount, 1);
  assert.match(summary.nextAction, /Runtime/);
  assert.ok(summary.items.some((item) => item.id === "backend-offline" && item.tone === "block"));
});

test("direct mesh UX guardrails block missing endpoints and warn missing token", () => {
  const summary = buildWorkbenchUxGuardrailSummary({
    ...BASE_INPUT,
    frontendRuntimeMode: "direct_mesh_gui",
    directMeshEndpointsText: "",
    directMeshApiToken: "",
    clusterApiToken: "",
  });

  assert.equal(summary.tone, "block");
  assert.equal(summary.blockedActionCount, 1);
  assert.equal(summary.warningCount, 1);
  assert.ok(summary.items.some((item) => item.id === "missing-mesh-endpoints"));
  assert.ok(summary.items.some((item) => item.id === "missing-mesh-token"));
});

test("UX guardrails report ready when runtime, workspace, and language pack are present", () => {
  const summary = buildWorkbenchUxGuardrailSummary(BASE_INPUT);

  assert.equal(summary.tone, "ok");
  assert.equal(summary.blockedActionCount, 0);
  assert.equal(summary.warningCount, 0);
  assert.equal(summary.items[0]?.id, "ready");
});

test("UX guardrail presentation uses the active workbench language copy", () => {
  const summary = buildWorkbenchUxGuardrailSummary({
    ...BASE_INPUT,
    selectedVersionId: null,
    languagePackCount: 0,
  });
  const localized = localizeWorkbenchUxGuardrailSummary(summary, resolveWorkbenchBaseCopy("zh"));

  assert.equal(localized.items[0]?.title, "还没有版本记录。");
  assert.equal(localized.items[1]?.title, "当前还没有安装自定义语言包。");
  assert.equal(localized.nextAction, "另存为");
  assert.doesNotMatch(localized.items.map((item) => `${item.title} ${item.detail} ${item.nextAction}`).join(" "), /No |Save |Install /);
});
