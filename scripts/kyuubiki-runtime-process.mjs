import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";

export function spawnManaged({ pidPath, logPath, cwd, command, args, env }) {
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

export async function stopManagedProcess(pidPath, label, port) {
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

export function readPid(pidPath) {
  try {
    const raw = fs.readFileSync(pidPath, "utf8").trim();
    const pid = Number.parseInt(raw, 10);
    return Number.isFinite(pid) ? pid : null;
  } catch {
    return null;
  }
}

export function isPidAlive(pid) {
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

export async function waitForPortState(port, expectedListening, timeoutMs) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if ((await isPortListening(port)) === expectedListening) {
      return;
    }
    await sleep(200);
  }
}

export async function isPortListening(port) {
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

export function platformCommand(name) {
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

function safeUnlink(target) {
  try {
    fs.unlinkSync(target);
  } catch {}
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

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
