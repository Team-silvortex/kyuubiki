import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const frontendDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const baseUrl = process.env.WORKFLOW_BENCHMARK_URL || "http://127.0.0.1:3000/workflow-benchmark";

async function isFrontendReady() {
  try {
    const response = await fetch(baseUrl);
    return response.ok;
  } catch {
    return false;
  }
}

async function waitForFrontend() {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    if (await isFrontendReady()) return;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`timed out waiting for workflow frontend at ${baseUrl}`);
}

function startFrontend() {
  const npm = process.platform === "win32" ? "npm.cmd" : "npm";
  return spawn(npm, ["run", "dev"], {
    cwd: frontendDir,
    env: process.env,
    stdio: "inherit",
  });
}

function runCheck(script) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [script], {
      cwd: frontendDir,
      env: process.env,
      stdio: "inherit",
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) return resolve();
      reject(new Error(`${script} exited with ${signal ?? `code ${code}`}`));
    });
  });
}

async function main() {
  const reusedFrontend = await isFrontendReady();
  const frontend = reusedFrontend ? null : startFrontend();
  try {
    await waitForFrontend();
    await runCheck("./scripts/check-workflow-topology-regression.mjs");
    await runCheck("./scripts/check-workflow-search-layout.mjs");
    console.log(`Workflow browser preflight passed (${reusedFrontend ? "reused" : "temporary"} frontend).`);
  } finally {
    if (frontend && !frontend.killed) frontend.kill("SIGTERM");
  }
}

main().catch((error) => {
  console.error(`Workflow browser preflight failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
});
