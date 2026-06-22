import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const FRONTEND_ROOT = path.resolve(import.meta.dirname, "../..");
const CLI_PATH = path.join(FRONTEND_ROOT, "scripts/kyuubiki-cli.mjs");

function runCli(args) {
  const result = spawnSync("node", [CLI_PATH, ...args], {
    cwd: FRONTEND_ROOT,
    encoding: "utf8",
  });
  return {
    status: result.status ?? 1,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
  };
}

async function makeTempDir() {
  return mkdtemp(path.join(tmpdir(), "kyuubiki-headless-test-"));
}

test("headless templates filters service workflows and emits JSON", () => {
  const result = runCli(["headless", "templates", "--runtime", "service_only", "--json"]);
  assert.equal(result.status, 0, result.stderr);
  const payload = JSON.parse(result.stdout);
  assert.ok(payload.template_count >= 1);
  assert.ok(payload.templates.every((entry) => entry.runtime_style === "service_only"));
  assert.ok(payload.templates.some((entry) => entry.id === "workflow_submit_monitor"));
});

test("headless init writes a workflow contract that inspect can summarize", async () => {
  const tempDir = await makeTempDir();
  const workflowPath = path.join(tempDir, "workflow.json");

  const init = runCli([
    "headless",
    "init",
    "--template",
    "workflow_submit_monitor",
    "--workflow-id",
    "workflow.test.submit-monitor",
    "--out",
    workflowPath,
  ]);
  assert.equal(init.status, 0, init.stderr);

  const workflow = JSON.parse(await readFile(workflowPath, "utf8"));
  assert.equal(workflow.schema_version, "kyuubiki.headless-workflow/v1");
  assert.equal(workflow.workflow.id, "workflow.test.submit-monitor");
  assert.equal(workflow.workflow.steps.length, 3);

  const inspect = runCli(["headless", "inspect", workflowPath, "--json"]);
  assert.equal(inspect.status, 0, inspect.stderr);
  const summary = JSON.parse(inspect.stdout);
  assert.equal(summary.workflow_id, "workflow.test.submit-monitor");
  assert.equal(summary.step_count, 3);
  assert.deepEqual(summary.actions, ["workflow_submit_catalog", "job_wait", "result_fetch"]);
});

test("headless validate accepts a generated service workflow and reports service-only policy", async () => {
  const tempDir = await makeTempDir();
  const workflowPath = path.join(tempDir, "workflow.json");

  const init = runCli(["headless", "init", "--template", "solve_wait_result", "--out", workflowPath]);
  assert.equal(init.status, 0, init.stderr);

  const validate = runCli(["headless", "validate", workflowPath, "--json"]);
  assert.equal(validate.status, 0, validate.stderr);
  const report = JSON.parse(validate.stdout);
  assert.equal(report.ok, true);
  assert.equal(report.summary.workflow_id, "template.solve_wait_result");
  assert.equal(report.policy.recommended_runtime, "service_only");
  assert.equal(report.policy.safe_for_service_only, true);
});

test("headless plan reports service runtime and step bindings", async () => {
  const tempDir = await makeTempDir();
  const workflowPath = path.join(tempDir, "workflow.json");
  const planPath = path.join(tempDir, "plan.json");

  const init = runCli(["headless", "init", "--template", "workflow_submit_monitor", "--out", workflowPath]);
  assert.equal(init.status, 0, init.stderr);

  const plan = runCli(["headless", "plan", workflowPath, "--json", "--out", planPath]);
  assert.equal(plan.status, 0, plan.stderr);
  const payload = JSON.parse(plan.stdout);
  assert.equal(payload.schema_version, "kyuubiki.headless-plan/v1");
  assert.equal(payload.policy.recommended_runtime, "service_only");
  assert.equal(payload.compatibility.service_only.ok, true);
  assert.equal(payload.confirmation_count, 0);
  assert.deepEqual(payload.steps[1].bindings, [{ source_step: 1, output: "job_id" }]);

  const savedPlan = JSON.parse(await readFile(planPath, "utf8"));
  assert.equal(savedPlan.workflow_id, payload.workflow_id);
});

test("headless plan surfaces sensitive confirmation gates", async () => {
  const tempDir = await makeTempDir();
  const workflowPath = path.join(tempDir, "workflow.json");

  const init = runCli(["headless", "init", "--template", "browser_capture_review", "--out", workflowPath]);
  assert.equal(init.status, 0, init.stderr);

  const plan = runCli(["headless", "plan", workflowPath, "--json"]);
  assert.equal(plan.status, 0, plan.stderr);
  const payload = JSON.parse(plan.stdout);
  assert.equal(payload.policy.recommended_runtime, "browser_only");
  assert.equal(payload.compatibility.browser_session_required, true);
  assert.equal(payload.confirmation_count, 1);
  assert.equal(payload.confirmations[0].action, "snapshot");
  assert.equal(payload.confirmations[0].flag, "--allow-sensitive");
});

test("headless run simulates a workflow document and resolves prior-step bindings", async () => {
  const tempDir = await makeTempDir();
  const workflowPath = path.join(tempDir, "workflow.json");
  const reportPath = path.join(tempDir, "run-report.json");

  const init = runCli(["headless", "init", "--template", "workflow_submit_monitor", "--out", workflowPath]);
  assert.equal(init.status, 0, init.stderr);

  const run = runCli(["headless", "run", workflowPath, "--json", "--report-out", reportPath]);
  assert.equal(run.status, 0, run.stderr);
  const payload = JSON.parse(run.stdout);
  assert.equal(payload.report.status, "simulated");
  assert.equal(payload.report.executed_step_count, 3);
  assert.equal(payload.report.steps[0].result.job_id, "simulated-workflow_submit_catalog-1-job_id");
  assert.equal(payload.report.steps[1].payload.job_id, "simulated-workflow_submit_catalog-1-job_id");
  assert.equal(payload.report.steps[2].payload.job_id, "simulated-workflow_submit_catalog-1-job_id");

  const savedReport = JSON.parse(await readFile(reportPath, "utf8"));
  assert.equal(savedReport.report.workflow_id, payload.report.workflow_id);
});

test("headless run blocks sensitive browser capture in execute mode without confirmation", async () => {
  const tempDir = await makeTempDir();
  const workflowPath = path.join(tempDir, "workflow.json");

  const init = runCli(["headless", "init", "--template", "browser_capture_review", "--out", workflowPath]);
  assert.equal(init.status, 0, init.stderr);

  const run = runCli(["headless", "run", workflowPath, "--json", "--execute"]);
  assert.equal(run.status, 0, run.stderr);
  const payload = JSON.parse(run.stdout);
  assert.equal(payload.report.status, "blocked");
  assert.equal(payload.report.executed_step_count, 2);
  assert.equal(payload.report.blocked_by_confirmation?.action, "snapshot");
  assert.equal(payload.report.blocked_by_confirmation?.risk, "sensitive");
});
