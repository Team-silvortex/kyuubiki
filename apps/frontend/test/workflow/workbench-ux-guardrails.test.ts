import test from "node:test";
import assert from "node:assert/strict";

import {
  buildWorkbenchUxGuardrailSummary,
  type WorkbenchUxGuardrailInput,
} from "../../src/components/workbench/workbench-ux-guardrails.ts";

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
