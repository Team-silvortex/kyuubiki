import test from "node:test";
import assert from "node:assert/strict";

import { evaluateWorkbenchUxActionGuardrail } from "../../src/components/workbench/workbench-ux-action-guardrails.ts";
import { invokeWorkbenchScriptAction } from "../../src/components/workbench/workbench-script-orchestration.ts";
import type { WorkbenchUxGuardrailSummary } from "../../src/components/workbench/workbench-ux-guardrails.ts";
import type { WorkbenchScriptActionDefinition } from "../../src/lib/scripting/workbench-script-runtime-types.ts";

const BLOCKED_SUMMARY: WorkbenchUxGuardrailSummary = {
  tone: "block",
  blockedActionCount: 1,
  warningCount: 0,
  nextAction: "Open System > Runtime.",
  items: [{
    id: "backend-offline",
    tone: "block",
    title: "Control plane is offline",
    detail: "Workbench actions that need orchestration may not apply.",
    nextAction: "Open System > Runtime and start or refresh the control plane.",
  }],
};

function definition(category: string, risk: WorkbenchScriptActionDefinition["risk"] = "normal"): WorkbenchScriptActionDefinition {
  return {
    id: `${category}/demo`,
    category,
    risk,
    summary: { en: "demo", zh: "demo" },
  };
}

test("UX action guardrails block runtime-backed job actions when a blocking system issue is active", () => {
  const decision = evaluateWorkbenchUxActionGuardrail({
    action: "job/run",
    definition: definition("job"),
    summary: BLOCKED_SUMMARY,
  });

  assert.equal(decision.allowed, false);
  assert.equal(decision.blockingItemId, "backend-offline");
  assert.match(decision.reason ?? "", /Control plane is offline/);
});

test("UX action guardrails keep local navigation usable during recovery", () => {
  const decision = evaluateWorkbenchUxActionGuardrail({
    action: "nav/setSidebarSection",
    definition: definition("navigation"),
    summary: BLOCKED_SUMMARY,
  });

  assert.equal(decision.allowed, true);
});

test("UX action guardrails allow all actions when only warnings are present", () => {
  const decision = evaluateWorkbenchUxActionGuardrail({
    action: "data/exportDatabase",
    definition: definition("data", "sensitive"),
    summary: { ...BLOCKED_SUMMARY, tone: "warn", blockedActionCount: 0, warningCount: 1 },
  });

  assert.equal(decision.allowed, true);
});

test("script invocation records a failed action when UX guardrails block execution", async () => {
  const audits: Array<Record<string, unknown>> = [];
  const logs: Array<Record<string, unknown>> = [];

  await assert.rejects(
    () =>
      invokeWorkbenchScriptAction({
        action: "job/run",
        payload: {},
        source: "script",
        language: "zh",
        uxGuardrailSummary: BLOCKED_SUMMARY,
        recordSecurityAuditEvent: (entry: Record<string, unknown>) => audits.push(entry),
        appendScriptActionLog: (entry: Record<string, unknown>) => logs.push(entry),
      }),
    /防呆保护拦截/,
  );

  assert.equal(audits[0]?.action, "job/run");
  assert.equal(audits[0]?.status, "failed");
  assert.equal(logs[0]?.status, "failed");
  assert.equal(logs[0]?.note, "backend-offline");
});

test("script invocation records download execution failures without a false completion", async () => {
  const audits: Array<Record<string, unknown>> = [];
  const logs: Array<Record<string, unknown>> = [];
  const failure = new Error("project bundle download failed");

  await assert.rejects(
    invokeWorkbenchScriptAction({
      action: "project/exportJson",
      appendScriptActionLog: (entry: Record<string, unknown>) => logs.push(entry),
      handleWorkbenchScriptNavAction: async () => null,
      handleWorkbenchScriptProjectModelAction: async () => { throw failure; },
      language: "en",
      navArgs: {},
      payload: {},
      projectModelArgs: {},
      recordSecurityAuditEvent: (entry: Record<string, unknown>) => audits.push(entry),
      source: "script",
    }),
    failure,
  );

  assert.deepEqual(logs.map((entry) => entry.status), ["started", "failed"]);
  assert.deepEqual(audits.map((entry) => entry.status), ["prompted", "failed"]);
});
