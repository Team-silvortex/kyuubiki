#!/usr/bin/env node

import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const WEB_DIR = path.join(ROOT_DIR, "apps/web");
const FRONTEND_DIR = path.join(ROOT_DIR, "apps/frontend");
const RUST_DIR = path.join(ROOT_DIR, "workers/rust");
const RUN_DIR = path.join(ROOT_DIR, "tmp/run");
const HOT_RUN_DIR = path.join(RUN_DIR, "hot");
const ENV_FILE = path.join(ROOT_DIR, ".env.local");
const ENV_EXAMPLE_FILE = path.join(ROOT_DIR, ".env.example");
const RUNTIME_MODE_FILE = path.join(RUN_DIR, "runtime-mode.txt");
const SERVICE_PORTS = { orchestrator: 4000, frontend: 3000 };
const SERVICE_FILES = {
  orchestrator: { pid: path.join(RUN_DIR, "orchestrator.pid"), log: path.join(RUN_DIR, "orchestrator.log") },
  frontend: { pid: path.join(RUN_DIR, "frontend.pid"), log: path.join(RUN_DIR, "frontend.log") },
  hot: { pid: path.join(HOT_RUN_DIR, "stack.pid"), log: path.join(HOT_RUN_DIR, "stack.console.log") },
};
const DEFAULT_AGENT_ENDPOINTS = "127.0.0.1:5001,127.0.0.1:5002";

async function main() {
  const [command = "help", ...args] = process.argv.slice(2);

  switch (command) {
    case "help":
      printHelp();
      break;
    case "status":
      console.log(await renderServiceStatus());
      break;
    case "start":
      await startServices("default");
      break;
    case "start-local":
      await startServices("local");
      break;
    case "start-cloud":
      await startServices("cloud");
      break;
    case "start-distributed":
      await startServices("distributed");
      break;
    case "restart":
      await restartServices("default");
      break;
    case "restart-local":
      await restartServices("local");
      break;
    case "restart-cloud":
      await restartServices("cloud");
      break;
    case "restart-distributed":
      await restartServices("distributed");
      break;
    case "stop":
      await stopServices();
      break;
    case "export-db":
      process.stdout.write(await exportDb(args[0]));
      break;
    case "hot-status":
      console.log(await renderHotStatus());
      break;
    case "hot-start-local":
      await startHotStack("local");
      break;
    case "hot-start-cloud":
      await startHotStack("cloud");
      break;
    case "hot-start-distributed":
      await startHotStack("distributed");
      break;
    case "hot-stop":
      await stopHotStack();
      break;
    default:
      throw new Error(`unknown runtime command: ${command}`);
  }
}

function printHelp() {
  console.log(["kyuubiki runtime launcher", "", "Commands:", "  status", "  start | start-local | start-cloud | start-distributed", "  restart | restart-local | restart-cloud | restart-distributed", "  stop", "  export-db [url]", "  hot-status", "  hot-start-local | hot-start-cloud | hot-start-distributed", "  hot-stop"].join("\n"));
}

async function renderServiceStatus() {
  const agents = loadAgentPorts();
  const mode = readRuntimeMode();
  const lines = [];
  lines.push(`deployment-mode: ${mode}`);
  lines.push(`control-mode: ${controlModeLabel(mode)}`);
  lines.push(`authority-mode: ${authorityModeLabel(mode)}`);
  lines.push(await formatHttpStatus("orchestrator", SERVICE_FILES.orchestrator.pid, SERVICE_PORTS.orchestrator));
  lines.push(await formatHttpStatus("frontend", SERVICE_FILES.frontend.pid, SERVICE_PORTS.frontend));
  for (const port of agents) {
    lines.push(await formatAgentStatus(port));
  }
  return lines.join("\n");
}

async function renderHotStatus() {
  const agents = loadAgentPorts();
  const lines = [];
  const hotPid = readPid(SERVICE_FILES.hot.pid);
  lines.push(isPidAlive(hotPid) ? `hot-loop: running (pid ${hotPid})` : "hot-loop: stopped");
  lines.push(await formatListeningStatus("hot-web", "http://127.0.0.1:4000", SERVICE_PORTS.orchestrator));
  lines.push(await formatListeningStatus("hot-frontend", "http://127.0.0.1:3000", SERVICE_PORTS.frontend));
  for (const port of agents) {
    const label = `hot-agent[${port}]`;
    const url = `tcp://127.0.0.1:${port}`;
    lines.push((await isPortListening(port)) ? `${label}: listening on ${url}` : `${label}: stopped`);
  }
  if (fs.existsSync(HOT_RUN_DIR)) {
    lines.push(`hot-logs: ${HOT_RUN_DIR}`);
  }
  return lines.join("\n");
}

async function startServices(mode) {
  ensureRunDirs();
  const resolvedMode = resolveDeploymentMode(mode);
  if (mode !== "distributed") {
    for (const port of loadAgentPorts()) {
      await startAgent(port);
    }
  }
  await startOrchestrator(mode);
  await startFrontend();
  writeRuntimeMode(resolvedMode);
}

async function restartServices(mode) {
  await stopServices();
  await startServices(mode);
  console.log("restart complete");
}

async function stopServices() {
  const agents = loadAgentPorts().reverse();
  for (const port of agents) {
    await stopManagedProcess(agentFiles(port).pid, `agent[${port}]`, port);
  }
  await stopManagedProcess(SERVICE_FILES.frontend.pid, "frontend", SERVICE_PORTS.frontend);
  await stopManagedProcess(SERVICE_FILES.orchestrator.pid, "orchestrator", SERVICE_PORTS.orchestrator);
  removeRuntimeMode();
}

async function startOrchestrator(mode) {
  if (await isPortListening(SERVICE_PORTS.orchestrator)) {
    console.log(`orchestrator already running at http://127.0.0.1:${SERVICE_PORTS.orchestrator}`);
    return;
  }

  const env = buildModeEnv(mode);
  env.PORT = String(SERVICE_PORTS.orchestrator);
  env.KYUUBIKI_AGENT_ENDPOINTS = agentEndpointsValue();
  spawnManaged({
    pidPath: SERVICE_FILES.orchestrator.pid,
    logPath: SERVICE_FILES.orchestrator.log,
    cwd: WEB_DIR,
    command: platformCommand("mix"),
    args: ["run", "--no-halt"],
    env,
  });

  await waitForPortState(SERVICE_PORTS.orchestrator, true, 15_000);
  console.log(
    `started orchestrator API at http://127.0.0.1:${SERVICE_PORTS.orchestrator} (${storageModeLabel(mode)}, ${deploymentModeLabel(mode)})`,
  );
  console.log(`log: ${SERVICE_FILES.orchestrator.log}`);
}

async function startFrontend() {
  if (await isPortListening(SERVICE_PORTS.frontend)) {
    console.log(`frontend already running at http://127.0.0.1:${SERVICE_PORTS.frontend}`);
    return;
  }
  spawnManaged({
    pidPath: SERVICE_FILES.frontend.pid,
    logPath: SERVICE_FILES.frontend.log,
    cwd: FRONTEND_DIR,
    command: platformCommand("npm"),
    args: ["run", "dev"],
    env: process.env,
  });

  await waitForPortState(SERVICE_PORTS.frontend, true, 20_000);
  console.log(`started Next.js workbench at http://127.0.0.1:${SERVICE_PORTS.frontend}`);
  console.log(`log: ${SERVICE_FILES.frontend.log}`);
}

async function startAgent(port) {
  if (await isPortListening(port)) {
    console.log(`Rust FEM agent already running at tcp://127.0.0.1:${port}`);
    return;
  }
  const files = agentFiles(port);
  spawnManaged({
    pidPath: files.pid,
    logPath: files.log,
    cwd: RUST_DIR,
    command: platformCommand("cargo"),
    args: ["run", "-p", "kyuubiki-cli", "--", "agent", "--port", String(port)],
    env: process.env,
  });

  await waitForPortState(port, true, 20_000);
  console.log(`started Rust FEM agent at tcp://127.0.0.1:${port}`);
  console.log(`log: ${files.log}`);
}

async function startHotStack(mode) {
  ensureRunDirs();
  const pid = readPid(SERVICE_FILES.hot.pid);
  if (isPidAlive(pid)) {
    console.log(`managed hot-reload loop already running (pid ${pid})`);
    return;
  }

  const env = buildModeEnv(mode);
  env.KYUUBIKI_AGENT_ENDPOINTS = agentEndpointsValue();
  env.KYUUBIKI_HOT_LOG_DIR = HOT_RUN_DIR;
  spawnManaged({
    pidPath: SERVICE_FILES.hot.pid,
    logPath: SERVICE_FILES.hot.log,
    cwd: ROOT_DIR,
    command: platformCommand("node"),
    args: [
      "./scripts/hot-dev.mjs",
      "stack",
      "--mode",
      hotModeLabel(mode),
      "--orchestrator-port",
      "4000",
      "--frontend-port",
      "3000",
      "--agent-endpoints",
      agentEndpointsValue(),
    ],
    env,
  });

  console.log(`started managed hot-reload loop (${hotModeLabel(mode)})`);
  console.log(`logs: ${HOT_RUN_DIR}`);
}

async function stopHotStack() {
  await stopManagedProcess(SERVICE_FILES.hot.pid, "hot-loop");
}

async function exportDb(url = "http://127.0.0.1:4000/api/v1/export/database") {
  if (!(await isPortListening(SERVICE_PORTS.orchestrator))) {
    throw new Error(`orchestrator is not running at http://127.0.0.1:${SERVICE_PORTS.orchestrator}`);
  }
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`export-db failed with HTTP ${response.status}`);
  }

  return await response.text();
}

function ensureRunDirs() {
  fs.mkdirSync(RUN_DIR, { recursive: true });
  fs.mkdirSync(HOT_RUN_DIR, { recursive: true });
}

function readRuntimeMode() {
  try {
    const value = fs.readFileSync(RUNTIME_MODE_FILE, "utf8").trim();
    if (value === "local" || value === "cloud" || value === "distributed") {
      return value;
    }
  } catch {}

  return resolveDeploymentMode("default");
}

function writeRuntimeMode(mode) {
  fs.mkdirSync(RUN_DIR, { recursive: true });
  fs.writeFileSync(RUNTIME_MODE_FILE, `${resolveDeploymentMode(mode)}\n`, "utf8");
}

function removeRuntimeMode() {
  fs.rmSync(RUNTIME_MODE_FILE, { force: true });
}

function spawnManaged({ pidPath, logPath, cwd, command, args, env }) {
  fs.mkdirSync(path.dirname(pidPath), { recursive: true });
  fs.mkdirSync(path.dirname(logPath), { recursive: true });
  const output = fs.openSync(logPath, "a");
  const child = spawn(command, args, {
    cwd,
    env: { ...process.env, ...env },
    detached: true,
    stdio: ["ignore", output, output],
    windowsHide: true,
  });

  child.unref();
  fs.writeFileSync(pidPath, `${child.pid}\n`, "utf8");
}

async function stopManagedProcess(pidPath, label, port) {
  const pid = readPid(pidPath);
  if (isPidAlive(pid)) {
    await killProcessTree(pid);
    if (typeof port === "number") {
      await waitForPortState(port, false, 10_000);
    }
    safeUnlink(pidPath);
    console.log(`stopped ${label} (pid ${pid})`);
    return;
  }

  safeUnlink(pidPath);
  if (typeof port === "number" && (await isPortListening(port))) {
    console.log(`${label}: port ${port} is still busy (unmanaged process)`);
  } else {
    console.log(`${label}: stopped`);
  }
}

function readPid(pidPath) {
  try {
    const raw = fs.readFileSync(pidPath, "utf8").trim();
    const pid = Number.parseInt(raw, 10);
    return Number.isFinite(pid) ? pid : null;
  } catch {
    return null;
  }
}

function isPidAlive(pid) {
  if (!pid) {
    return false;
  }

  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function killProcessTree(pid) {
  if (process.platform === "win32") {
    await runCommand("taskkill", ["/PID", String(pid), "/T", "/F"]);
    return;
  }

  try {
    process.kill(-pid, "SIGTERM");
  } catch {
    try {
      process.kill(pid, "SIGTERM");
    } catch {
      return;
    }
  }

  await waitForProcessExit(pid, 5_000);
  if (isPidAlive(pid)) {
    try {
      process.kill(-pid, "SIGKILL");
    } catch {
      try {
        process.kill(pid, "SIGKILL");
      } catch {
        return;
      }
    }
  }
}

async function waitForProcessExit(pid, timeoutMs) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if (!isPidAlive(pid)) {
      return;
    }
    await sleep(200);
  }
}

async function waitForPortState(port, expectedListening, timeoutMs) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if ((await isPortListening(port)) === expectedListening) {
      return;
    }
    await sleep(200);
  }
}

async function isPortListening(port) {
  return await new Promise((resolve) => {
    const socket = net.createConnection({ host: "127.0.0.1", port });
    socket.once("connect", () => {
      socket.destroy();
      resolve(true);
    });
    socket.once("error", () => {
      socket.destroy();
      resolve(false);
    });
  });
}

async function formatHttpStatus(label, pidPath, port) {
  const pid = readPid(pidPath);
  const url = `http://127.0.0.1:${port}`;
  if (isPidAlive(pid) && (await isPortListening(port))) {
    return `${label}: running on ${url} (pid ${pid})`;
  }
  if (await isPortListening(port)) {
    return `${label}: running on ${url} (unmanaged pid)`;
  }
  return `${label}: stopped`;
}

async function formatAgentStatus(port) {
  const files = agentFiles(port);
  const pid = readPid(files.pid);
  const url = `tcp://127.0.0.1:${port}`;
  if (isPidAlive(pid) && (await isPortListening(port))) {
    return `agent[${port}]: running on ${url} (pid ${pid})`;
  }
  if (await isPortListening(port)) {
    return `agent[${port}]: running on ${url} (unmanaged pid)`;
  }
  return `agent[${port}]: stopped`;
}

async function formatListeningStatus(label, url, port) {
  return (await isPortListening(port)) ? `${label}: listening on ${url}` : `${label}: stopped`;
}

function loadAgentPorts() {
  const env = loadEnvValues();
  if ((env.KYUUBIKI_AGENT_DISCOVERY ?? "static") === "manifest") {
    return loadManifestAgentPorts(env);
  }
  return agentEndpointsValue()
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => Number.parseInt(entry.split(":").pop() ?? "", 10))
    .filter(Number.isFinite);
}

function loadManifestAgentPorts(env) {
  const manifestPath = resolveWorkspacePath(env.KYUUBIKI_AGENT_MANIFEST_PATH ?? "./deploy/agents.local.json");
  try {
    const payload = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    return Array.isArray(payload.agents)
      ? payload.agents.map((agent) => Number.parseInt(String(agent.port ?? ""), 10)).filter(Number.isFinite)
      : [];
  } catch {
    return [];
  }
}

function agentEndpointsValue() {
  const env = loadEnvValues();
  return env.KYUUBIKI_AGENT_ENDPOINTS ?? DEFAULT_AGENT_ENDPOINTS;
}

function buildModeEnv(mode) {
  const env = loadEnvValues();
  const merged = { ...process.env, ...env };
  if (mode === "local") {
    merged.KYUUBIKI_STORAGE_BACKEND = "sqlite";
    merged.SQLITE_DATABASE_PATH = env.SQLITE_DATABASE_PATH ?? "./tmp/data/kyuubiki_dev.sqlite3";
    merged.KYUUBIKI_DEPLOYMENT_MODE = "local";
  } else if (mode === "cloud") {
    requireEnv(merged.DATABASE_URL, "DATABASE_URL is required for cloud mode");
    merged.KYUUBIKI_STORAGE_BACKEND = "postgres";
    merged.KYUUBIKI_DEPLOYMENT_MODE = "cloud";
  } else if (mode === "distributed") {
    requireEnv(merged.DATABASE_URL, "DATABASE_URL is required for distributed mode");
    merged.KYUUBIKI_STORAGE_BACKEND = "postgres";
    merged.KYUUBIKI_DEPLOYMENT_MODE = "distributed";
  }
  return merged;
}

function storageModeLabel(mode) {
  if (mode === "local") {
    return "sqlite";
  }
  if (mode === "cloud" || mode === "distributed") {
    return "postgres";
  }
  return loadEnvValues().KYUUBIKI_STORAGE_BACKEND ?? "sqlite";
}

function deploymentModeLabel(mode) {
  return resolveDeploymentMode(mode);
}

function resolveDeploymentMode(mode) {
  if (mode === "local" || mode === "cloud" || mode === "distributed") {
    return mode;
  }
  return loadEnvValues().KYUUBIKI_DEPLOYMENT_MODE ?? "local";
}

function controlModeLabel(mode) {
  return resolveDeploymentMode(mode) === "local" ? "standalone" : "orch_managed";
}

function authorityModeLabel(mode) {
  return resolveDeploymentMode(mode) === "local" ? "self_directed" : "single_orchestrator";
}

function hotModeLabel(mode) {
  return mode === "cloud" || mode === "distributed" ? mode : "local";
}

function loadEnvValues() {
  if (loadEnvValues.cache) {
    return loadEnvValues.cache;
  }

  const values = {};
  for (const envPath of [ENV_EXAMPLE_FILE, ENV_FILE]) {
    if (!fs.existsSync(envPath)) {
      continue;
    }

    const lines = fs.readFileSync(envPath, "utf8").split(/\r?\n/u);
    for (const rawLine of lines) {
      const line = rawLine.trim();
      if (!line || line.startsWith("#")) {
        continue;
      }
      const separator = line.indexOf("=");
      if (separator <= 0) {
        continue;
      }
      const key = line.slice(0, separator).trim();
      const value = line.slice(separator + 1).trim();
      values[key] = value;
    }
  }

  loadEnvValues.cache = values;
  return values;
}

loadEnvValues.cache = null;

function resolveWorkspacePath(value) {
  return path.isAbsolute(value) ? value : path.join(ROOT_DIR, value);
}

function agentFiles(port) {
  return { pid: path.join(RUN_DIR, `agent-${port}.pid`), log: path.join(RUN_DIR, `agent-${port}.log`) };
}

function safeUnlink(target) {
  try {
    fs.unlinkSync(target);
  } catch {}
}

function platformCommand(name) {
  if (process.platform !== "win32") {
    return name;
  }
  if (name === "npm") {
    return "npm.cmd";
  }
  if (name === "mix") {
    return "mix.bat";
  }
  if (name === "cargo") {
    return "cargo.exe";
  }
  if (name === "node") {
    return "node.exe";
  }
  return name;
}

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: "ignore", windowsHide: true });
    child.once("error", reject);
    child.once("exit", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} exited with code ${code}`));
      }
    });
  });
}

function requireEnv(value, message) {
  if (!value) {
    throw new Error(message);
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
