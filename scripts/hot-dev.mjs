#!/usr/bin/env node

import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, "..");

const APPS = {
  web: path.join(ROOT_DIR, "apps", "web"),
  frontend: path.join(ROOT_DIR, "apps", "frontend"),
  hub: path.join(ROOT_DIR, "apps", "hub-gui"),
  installer: path.join(ROOT_DIR, "apps", "installer-gui"),
  workbench: path.join(ROOT_DIR, "apps", "workbench-gui"),
  rust: path.join(ROOT_DIR, "workers", "rust"),
};

const WATCH_IGNORE_DIRS = new Set([".git", "node_modules", ".next", "target", "_build", "deps", "dist", "tmp"]);

function printUsage() {
  console.log(`Unified hot-reload launcher for local kyuubiki development.

Usage:
  node ./scripts/hot-dev.mjs stack [--mode local|cloud|distributed] [--orchestrator-port 4000] [--frontend-port 3000] [--agent-endpoints 127.0.0.1:5001,127.0.0.1:5002]
  node ./scripts/hot-dev.mjs web [--port 4000]
  node ./scripts/hot-dev.mjs frontend
  node ./scripts/hot-dev.mjs agent [--port 5001]
  node ./scripts/hot-dev.mjs hub-gui
  node ./scripts/hot-dev.mjs installer-gui
  node ./scripts/hot-dev.mjs workbench-gui

Notes:
  - Next.js and Tauri already provide their own HMR/live-reload loops.
  - This launcher mainly adds restart-on-change behavior for the Elixir control plane
    and Rust solver agents so the whole repo can iterate under one dev command.
  - Environment variables are inherited from the caller.
`);
}

function parseArgs(argv) {
  const [command = "help", ...rest] = argv;
  const options = {};

  for (let index = 0; index < rest.length; index += 1) {
    const token = rest[index];
    if (!token.startsWith("--")) continue;
    const key = token.slice(2);
    const value = rest[index + 1];
    if (!value || value.startsWith("--")) {
      options[key] = "true";
      continue;
    }
    options[key] = value;
    index += 1;
  }

  return { command, options };
}

function timestamp() {
  return new Date().toISOString().slice(11, 19);
}

function log(label, message) {
  process.stdout.write(`[${timestamp()}] [${label}] ${message}\n`);
  writeLogLine(label, message);
}

function logFileFor(label) {
  const logDir = process.env.KYUUBIKI_HOT_LOG_DIR;
  if (!logDir) return null;
  const safeLabel = label.replace(/[^a-zA-Z0-9._-]+/g, "-");
  return path.join(logDir, `${safeLabel}.log`);
}

function writeLogLine(label, message) {
  const target = logFileFor(label);
  if (!target) return;
  try {
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.appendFileSync(target, `[${timestamp()}] [${label}] ${message}\n`);
  } catch {
    // Ignore log sink failures during dev orchestration.
  }
}

function formatArgs(command, args) {
  return [command, ...args].join(" ");
}

function createLinePrefixer(label, targetStream) {
  let buffer = "";
  return (chunk) => {
    buffer += chunk.toString();
    const lines = buffer.split(/\r?\n/);
    buffer = lines.pop() ?? "";
    for (const line of lines) {
      if (!line) continue;
      targetStream.write(`[${timestamp()}] [${label}] ${line}\n`);
      writeLogLine(label, line);
    }
  };
}

function flushLinePrefixer(label, targetStream, remaining) {
  if (remaining) {
    targetStream.write(`[${timestamp()}] [${label}] ${remaining}\n`);
    writeLogLine(label, remaining);
  }
}

function buildFingerprint(watchRoots, allowedExtensions) {
  const entries = [];

  function walk(currentPath) {
    let stat;
    try {
      stat = fs.statSync(currentPath);
    } catch {
      return;
    }

    const base = path.basename(currentPath);

    if (stat.isDirectory()) {
      if (WATCH_IGNORE_DIRS.has(base)) return;
      let children = [];
      try {
        children = fs.readdirSync(currentPath);
      } catch {
        return;
      }
      for (const child of children) {
        walk(path.join(currentPath, child));
      }
      return;
    }

    if (!stat.isFile()) return;
    if (allowedExtensions.size > 0 && !allowedExtensions.has(path.extname(currentPath))) {
      return;
    }

    entries.push(`${path.relative(ROOT_DIR, currentPath)}:${stat.mtimeMs}`);
  }

  for (const root of watchRoots) {
    walk(root);
  }

  return entries.sort().join("|");
}

async function runOneShot({ cwd, command, args, env, label }) {
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      env,
      stdio: "pipe",
    });
    const stdoutPrefix = createLinePrefixer(label, process.stdout);
    const stderrPrefix = createLinePrefixer(label, process.stderr);
    let stdoutRemainder = "";
    let stderrRemainder = "";

    child.stdout.on("data", (chunk) => {
      stdoutRemainder += chunk.toString();
      const lines = stdoutRemainder.split(/\r?\n/);
      stdoutRemainder = lines.pop() ?? "";
      for (const line of lines) {
        if (!line) continue;
        process.stdout.write(`[${timestamp()}] [${label}] ${line}\n`);
        writeLogLine(label, line);
      }
    });
    child.stderr.on("data", (chunk) => {
      stderrRemainder += chunk.toString();
      const lines = stderrRemainder.split(/\r?\n/);
      stderrRemainder = lines.pop() ?? "";
      for (const line of lines) {
        if (!line) continue;
        process.stderr.write(`[${timestamp()}] [${label}] ${line}\n`);
        writeLogLine(label, line);
      }
    });

    child.once("error", reject);
    child.once("exit", (code) => {
      flushLinePrefixer(label, process.stdout, stdoutRemainder);
      flushLinePrefixer(label, process.stderr, stderrRemainder);
      if (code === 0) resolve();
      else reject(new Error(`${label} exited with code ${code ?? "unknown"}`));
    });
  });
}

class HotTask {
  constructor(manager, config) {
    this.manager = manager;
    this.config = config;
    this.child = null;
    this.watcher = null;
    this.currentFingerprint = null;
    this.restartPending = false;
    this.stopping = false;
    this.awaitingChange = false;
    this.stdoutRemainder = "";
    this.stderrRemainder = "";
  }

  async start(reason = "initial start") {
    if (this.child || this.stopping) return;
    const { name, cwd, command, args = [], env = process.env, preStart } = this.config;

    if (preStart) {
      log(name, `preparing dev shell (${reason})`);
      try {
        await runOneShot({ ...preStart, env: { ...process.env, ...env }, label: `${name}:prep` });
      } catch (error) {
        log(name, `prep failed: ${error instanceof Error ? error.message : String(error)}`);
        this.awaitingChange = Boolean(this.config.watch);
        return;
      }
    }

    log(name, `starting ${formatArgs(command, args)} (${reason})`);
    this.awaitingChange = false;
    this.stdoutRemainder = "";
    this.stderrRemainder = "";
    this.child = spawn(command, args, {
      cwd,
      env,
      stdio: "pipe",
    });

    this.child.stdout.on("data", (chunk) => {
      this.stdoutRemainder += chunk.toString();
      const lines = this.stdoutRemainder.split(/\r?\n/);
      this.stdoutRemainder = lines.pop() ?? "";
      for (const line of lines) {
        if (!line) continue;
        process.stdout.write(`[${timestamp()}] [${name}] ${line}\n`);
      }
    });

    this.child.stderr.on("data", (chunk) => {
      this.stderrRemainder += chunk.toString();
      const lines = this.stderrRemainder.split(/\r?\n/);
      this.stderrRemainder = lines.pop() ?? "";
      for (const line of lines) {
        if (!line) continue;
        process.stderr.write(`[${timestamp()}] [${name}] ${line}\n`);
      }
    });

    this.child.once("error", (error) => {
      log(name, `failed to spawn: ${error.message}`);
      this.child = null;
      if (!this.stopping && this.config.watch) {
        this.awaitingChange = true;
      } else if (!this.stopping) {
        this.manager.shutdown(1);
      }
    });

    this.child.once("exit", (code, signal) => {
      flushLinePrefixer(name, process.stdout, this.stdoutRemainder);
      flushLinePrefixer(name, process.stderr, this.stderrRemainder);
      this.child = null;

      if (this.stopping) return;

      if (this.config.watch) {
        this.awaitingChange = true;
        log(name, `stopped (${signal ?? code ?? "unknown"}); waiting for source changes`);
        return;
      }

      log(name, `stopped (${signal ?? code ?? "unknown"})`);
      this.manager.shutdown(typeof code === "number" ? code : 1);
    });

    if (this.config.watch && this.currentFingerprint === null) {
      this.currentFingerprint = buildFingerprint(this.config.watch.roots, this.config.watch.extensions);
      this.startWatcher();
    }
  }

  startWatcher() {
    if (!this.config.watch || this.watcher) return;

    this.watcher = setInterval(() => {
      const nextFingerprint = buildFingerprint(this.config.watch.roots, this.config.watch.extensions);
      if (this.currentFingerprint === null) {
        this.currentFingerprint = nextFingerprint;
        return;
      }
      if (nextFingerprint === this.currentFingerprint) return;
      this.currentFingerprint = nextFingerprint;
      void this.scheduleRestart("source change");
    }, this.config.watch.pollMs ?? 900);
  }

  async scheduleRestart(reason) {
    if (this.restartPending || this.stopping) return;
    this.restartPending = true;
    try {
      if (this.child) {
        await this.stop("SIGTERM");
      }
      await this.start(reason);
    } finally {
      this.restartPending = false;
    }
  }

  async stop(signal = "SIGTERM") {
    if (this.watcher) {
      clearInterval(this.watcher);
      this.watcher = null;
    }
    if (!this.child) return;
    this.stopping = true;
    const child = this.child;
    await new Promise((resolve) => {
      const timer = setTimeout(() => {
        child.kill("SIGKILL");
      }, 2500);
      child.once("exit", () => {
        clearTimeout(timer);
        resolve();
      });
      child.kill(signal);
    });
    this.stopping = false;
  }
}

class HotManager {
  constructor(tasks) {
    this.tasks = tasks.map((config) => new HotTask(this, config));
    this.shuttingDown = false;
  }

  async start() {
    for (const task of this.tasks) {
      // eslint-disable-next-line no-await-in-loop
      await task.start();
    }
  }

  async shutdown(code = 0) {
    if (this.shuttingDown) return;
    this.shuttingDown = true;
    log("hot-dev", "shutting down watched processes");
    for (const task of this.tasks) {
      // eslint-disable-next-line no-await-in-loop
      await task.stop();
    }
    process.exit(code);
  }
}

function envWith(overrides = {}) {
  return { ...process.env, ...overrides };
}

function createWebTask({ port = 4000, mode = "local" } = {}) {
  return {
    name: `web:${port}`,
    cwd: APPS.web,
    command: "mix",
    args: ["run", "--no-halt"],
    env: envWith({
      PORT: String(port),
      KYUUBIKI_HOT_RELOAD: "true",
      KYUUBIKI_HOT_RELOAD_MODE: mode,
    }),
    watch: {
      roots: [
        path.join(APPS.web, "lib"),
        path.join(APPS.web, "config"),
        path.join(APPS.web, "test"),
        path.join(APPS.web, "mix.exs"),
      ],
      extensions: new Set([".ex", ".exs"]),
      pollMs: 900,
    },
  };
}

function createFrontendTask({ port = 3000 } = {}) {
  return {
    name: `frontend:${port}`,
    cwd: APPS.frontend,
    command: "npm",
    args: ["run", "dev", "--", "--port", String(port)],
    env: envWith({ PORT: String(port) }),
  };
}

function createAgentTask({ port }) {
  return {
    name: `agent:${port}`,
    cwd: APPS.rust,
    command: "cargo",
    args: ["run", "-p", "kyuubiki-cli", "--", "agent", "--port", String(port)],
    env: envWith({ KYUUBIKI_HOT_RELOAD: "true" }),
    watch: {
      roots: [
        path.join(APPS.rust, "crates"),
        path.join(APPS.rust, "Cargo.toml"),
        path.join(APPS.rust, "Cargo.lock"),
      ],
      extensions: new Set([".rs", ".toml", ".lock"]),
      pollMs: 1000,
    },
  };
}

function createDesktopTask({ name, cwd }) {
  return {
    name,
    cwd,
    command: "npm",
    args: ["run", "tauri:dev"],
    preStart: {
      cwd,
      command: "npm",
      args: ["run", "sync:shared"],
    },
  };
}

function parseAgentPorts(rawValue) {
  return rawValue
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => Number(entry.split(":").pop()))
    .filter((value) => Number.isFinite(value));
}

async function main() {
  const { command, options } = parseArgs(process.argv.slice(2));

  if (command === "help" || command === "--help" || command === "-h") {
    printUsage();
    return;
  }

  let tasks = [];

  if (command === "stack") {
    const mode = options.mode ?? "local";
    const orchestratorPort = Number(options["orchestrator-port"] ?? 4000);
    const frontendPort = Number(options["frontend-port"] ?? 3000);
    const agentEndpoints = options["agent-endpoints"] ?? process.env.KYUUBIKI_AGENT_ENDPOINTS ?? "127.0.0.1:5001,127.0.0.1:5002";

    tasks.push(createWebTask({ port: orchestratorPort, mode }));
    if (mode !== "distributed") {
      tasks.push(createFrontendTask({ port: frontendPort }));
      for (const port of parseAgentPorts(agentEndpoints)) {
        tasks.push(createAgentTask({ port }));
      }
    }
  } else if (command === "web") {
    tasks = [createWebTask({ port: Number(options.port ?? 4000), mode: options.mode ?? "local" })];
  } else if (command === "frontend") {
    tasks = [createFrontendTask({ port: Number(options.port ?? 3000) })];
  } else if (command === "agent") {
    tasks = [createAgentTask({ port: Number(options.port ?? 5001) })];
  } else if (command === "hub-gui") {
    tasks = [createDesktopTask({ name: "hub-gui", cwd: APPS.hub })];
  } else if (command === "installer-gui") {
    tasks = [createDesktopTask({ name: "installer-gui", cwd: APPS.installer })];
  } else if (command === "workbench-gui") {
    tasks = [createDesktopTask({ name: "workbench-gui", cwd: APPS.workbench })];
  } else {
    printUsage();
    process.exitCode = 1;
    return;
  }

  const manager = new HotManager(tasks);
  process.on("SIGINT", () => {
    void manager.shutdown(0);
  });
  process.on("SIGTERM", () => {
    void manager.shutdown(0);
  });

  await manager.start();
}

main().catch((error) => {
  process.stderr.write(`[${timestamp()}] [hot-dev] ${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
});
