import { closeSync, existsSync, mkdirSync, openSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const LOCK_PATH = path.join(ROOT, "tmp", "run", "integration-runtime.lock");
const LOCK_TIMEOUT_MS = 180_000;
const LOCK_RETRY_MS = 100;

let lockDescriptor = null;

function sleep(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}

function lockOwnerIsAlive() {
  try {
    const owner = JSON.parse(readFileSync(LOCK_PATH, "utf8"));
    if (!Number.isInteger(owner.pid) || owner.pid <= 0) return false;
    process.kill(owner.pid, 0);
    return true;
  } catch {
    return false;
  }
}

export function acquireLocalRuntimeLock() {
  if (lockDescriptor !== null) return;
  mkdirSync(path.dirname(LOCK_PATH), { recursive: true });
  const deadline = Date.now() + LOCK_TIMEOUT_MS;

  while (Date.now() < deadline) {
    try {
      lockDescriptor = openSync(LOCK_PATH, "wx");
      writeFileSync(lockDescriptor, JSON.stringify({ pid: process.pid, acquired_at: new Date().toISOString() }));
      return;
    } catch (error) {
      if (error?.code !== "EEXIST") throw error;
      if (!lockOwnerIsAlive() && existsSync(LOCK_PATH)) {
        rmSync(LOCK_PATH, { force: true });
        continue;
      }
      sleep(LOCK_RETRY_MS);
    }
  }

  throw new Error(`timed out waiting for the local integration runtime lock at ${LOCK_PATH}`);
}

export function releaseLocalRuntimeLock() {
  if (lockDescriptor === null) return;
  closeSync(lockDescriptor);
  lockDescriptor = null;
  rmSync(LOCK_PATH, { force: true });
}

export function assertNoUnmanagedLocalRuntime(status) {
  const unmanaged = String(status)
    .split("\n")
    .filter((line) => line.includes("(unmanaged pid)"));
  if (unmanaged.length === 0) return;
  throw new Error(
    `integration smoke requires an isolated local runtime; found unmanaged service(s): ${unmanaged.join("; ")}`,
  );
}
