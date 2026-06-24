import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
export const ENTRYPOINT = `${ROOT}/scripts/kyuubiki-runtime.mjs`;
export const ORCHESTRATOR_URL = "http://127.0.0.1:4000";

export function runKyuubiki(...args) {
  return execFileSync("node", [ENTRYPOINT, ...args], {
    cwd: ROOT,
    stdio: "pipe",
    encoding: "utf8",
  });
}

export function loadSampleModel(filename) {
  return JSON.parse(readFileSync(`${ROOT}/apps/frontend/public/models/${filename}`, "utf8"));
}

export async function waitFor(url, predicate, timeoutMs = 30_000, intervalMs = 500) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        const payload = await response.json();
        if (predicate(payload)) {
          return payload;
        }
      }
    } catch {
      // wait for service boot
    }

    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`timed out waiting for ${url}`);
}
