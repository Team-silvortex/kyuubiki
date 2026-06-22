import test, { after } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";

const FRONTEND_ROOT = path.resolve(import.meta.dirname, "../..");
const WEB_ROOT = path.resolve(FRONTEND_ROOT, "../web");
const CLI_PATH = path.join(FRONTEND_ROOT, "scripts/kyuubiki-cli.mjs");
const SERVER_SCRIPT_PATH = path.join(WEB_ROOT, "test/support/headless_live_server.exs");

const ELECTROSTATIC_PLANE_QUAD_INPUT_ARTIFACTS = {
  electrostatic_model: {
    nodes: [
      { id: "n0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
      { id: "n1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
      { id: "n2", x: 1, y: 1, fix_potential: true, potential: 0, charge_density: 0 },
      { id: "n3", x: 0, y: 1, fix_potential: true, potential: 10, charge_density: 0 },
    ],
    elements: [
      {
        id: "epq0",
        node_i: 0,
        node_j: 1,
        node_k: 2,
        node_l: 3,
        thickness: 0.05,
        permittivity: 2.0,
      },
    ],
  },
};

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
  return mkdtemp(path.join(tmpdir(), "kyuubiki-headless-live-"));
}

async function startLiveServer() {
  const child = spawn("mix", ["run", SERVER_SCRIPT_PATH], {
    cwd: WEB_ROOT,
    env: {
      ...process.env,
      MIX_ENV: "test",
      KYUUBIKI_STORAGE_BACKEND: "sqlite",
      KYUUBIKI_DEPLOYMENT_MODE: "local",
      KYUUBIKI_HEADLESS_LIVE_SCENARIO: "electrostatic_quad_summary",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });

  let stdout = "";
  let stderr = "";

  const ready = await new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error(`timed out waiting for live server\nstdout:\n${stdout}\nstderr:\n${stderr}`));
    }, 60_000);

    function handleChunk(chunk) {
      const text = chunk.toString();
      stdout += text;
      const match = text.match(/HEADLESS_LIVE_SERVER_READY\s+(\d+)/);
      if (match) {
        clearTimeout(timeout);
        resolve({ port: Number(match[1]) });
      }
    }

    child.stdout.on("data", handleChunk);
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("exit", (code) => {
      clearTimeout(timeout);
      reject(new Error(`live server exited before ready with code ${code}\nstdout:\n${stdout}\nstderr:\n${stderr}`));
    });
    child.on("error", (error) => {
      clearTimeout(timeout);
      reject(error);
    });
  });

  return {
    child,
    port: ready.port,
    getLogs() {
      return { stdout, stderr };
    },
  };
}

const liveServer = await startLiveServer();

after(async () => {
  liveServer.child.kill("SIGTERM");
  await new Promise((resolve) => liveServer.child.once("exit", resolve));
});

test("headless live service health reaches the temporary control plane", { timeout: 60_000 }, async () => {
  const healthWorkflow = {
    schema_version: "kyuubiki.headless-workflow/v1",
    exported_at: new Date().toISOString(),
    workflow: {
      id: "workflow.live.service-health",
      steps: [
        {
          action: "service_health",
          payload: {},
        },
      ],
    },
  };
  const tempPath = path.join(tmpdir(), `kyuubiki-headless-health-${Date.now()}.json`);
  await writeFile(tempPath, `${JSON.stringify(healthWorkflow, null, 2)}\n`);
  const result = runCli([
    "headless",
    "run",
    tempPath,
    "--execute",
    "--api-base-url",
    `http://127.0.0.1:${liveServer.port}`,
    "--json",
  ]);
  assert.equal(result.status, 0, `${result.stderr}\n${JSON.stringify(liveServer.getLogs())}`);
  const payload = JSON.parse(result.stdout);
  assert.equal(payload.report.status, "completed");
  assert.equal(payload.report.steps[0].result.status, "ok");
  assert.equal(payload.report.steps[0].result.service, "kyuubiki-orchestrator");
});

test("headless live workflow submit executes against the temporary control plane", { timeout: 60_000 }, async () => {
  const tempDir = await makeTempDir();
  const workflowPath = path.join(tempDir, "live-workflow.json");

  const workflow = {
    schema_version: "kyuubiki.headless-workflow/v1",
    exported_at: new Date().toISOString(),
    language: "en",
    workflow: {
      id: "workflow.live.electrostatic-plane-quad",
      steps: [
        {
          action: "workflow_submit_catalog",
          payload: {
            workflow_id: "workflow.electrostatic-plane-quad-2d",
            input_artifacts: ELECTROSTATIC_PLANE_QUAD_INPUT_ARTIFACTS,
          },
        },
        {
          action: "job_wait",
          payload: {
            job_id: "{{steps.1.result.job_id}}",
            interval_ms: 20,
            timeout_ms: 5000,
          },
        },
        {
          action: "result_fetch",
          payload: {
            job_id: "{{steps.1.result.job_id}}",
          },
        },
      ],
    },
  };

  await writeFile(workflowPath, `${JSON.stringify(workflow, null, 2)}\n`);

  const result = runCli([
    "headless",
    "run",
    workflowPath,
    "--json",
    "--execute",
    "--api-base-url",
    `http://127.0.0.1:${liveServer.port}`,
  ]);

  assert.equal(result.status, 0, `${result.stderr}\n${JSON.stringify(liveServer.getLogs())}`);
  const payload = JSON.parse(result.stdout);
  assert.equal(payload.report.dry_run, false);
  assert.equal(payload.report.status, "completed");
  assert.equal(payload.report.executed_step_count, 3);
  assert.equal(payload.report.steps[0].status, "completed");
  assert.equal(payload.report.steps[1].status, "completed");
  assert.equal(payload.report.steps[2].status, "completed");
  assert.equal(typeof payload.report.steps[0].result.job_id, "string");
  assert.ok(payload.report.steps[0].result.job_id.length > 0);
  assert.equal(payload.report.steps[2].result.result.workflow_id, "workflow.electrostatic-plane-quad-2d");
});
