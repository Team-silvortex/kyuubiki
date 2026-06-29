import test from "node:test";
import assert from "node:assert/strict";
import {
  persistWorkbenchApiBaseUrl,
  readPersistedWorkbenchApiBaseUrl,
  resolveWorkbenchApiUrl,
  sanitizeWorkbenchApiBaseUrl,
  WORKBENCH_API_BASE_URL_STORAGE_KEY,
} from "@/lib/api/backend-target";
import { requestJson } from "@/lib/api/core";

test("sanitizeWorkbenchApiBaseUrl accepts http targets and removes hashes/trailing slashes", () => {
  assert.equal(
    sanitizeWorkbenchApiBaseUrl(" https://orch.example.local:4000/runtime/#secret "),
    "https://orch.example.local:4000/runtime",
  );
});

test("sanitizeWorkbenchApiBaseUrl rejects non-http targets", () => {
  assert.equal(sanitizeWorkbenchApiBaseUrl("file:///tmp/control-plane.sock"), "");
  assert.equal(sanitizeWorkbenchApiBaseUrl("javascript:alert(1)"), "");
});

test("resolveWorkbenchApiUrl keeps GUI API clients backend-target agnostic", () => {
  const target = { baseUrl: "http://127.0.0.1:4000", source: "environment" as const };

  assert.equal(resolveWorkbenchApiUrl("/api/v1/jobs", target), "http://127.0.0.1:4000/api/v1/jobs");
  assert.equal(resolveWorkbenchApiUrl("https://mesh.example/api/health", target), "https://mesh.example/api/health");
  assert.equal(resolveWorkbenchApiUrl("relative/path", target), "relative/path");
});

test("persistWorkbenchApiBaseUrl stores sanitized targets and clears invalid values", () => {
  const storage = new Map<string, string>();
  const previousWindow = globalThis.window;
  globalThis.window = {
    localStorage: {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
      removeItem: (key: string) => storage.delete(key),
    },
  } as unknown as Window & typeof globalThis;

  try {
    assert.equal(persistWorkbenchApiBaseUrl("http://127.0.0.1:4000////"), "http://127.0.0.1:4000");
    assert.equal(readPersistedWorkbenchApiBaseUrl(), "http://127.0.0.1:4000");
    assert.equal(storage.get(WORKBENCH_API_BASE_URL_STORAGE_KEY), "http://127.0.0.1:4000");

    assert.equal(persistWorkbenchApiBaseUrl("file:///tmp/socket"), "");
    assert.equal(readPersistedWorkbenchApiBaseUrl(), "");
  } finally {
    globalThis.window = previousWindow;
  }
});

test("requestJson can run outside a browser window while honoring backend targets", async () => {
  const previousFetch = globalThis.fetch;
  const previousWindow = globalThis.window;
  const seenUrls: string[] = [];

  globalThis.fetch = (async (url: RequestInfo | URL) => {
    seenUrls.push(String(url));
    return new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }) as typeof fetch;

  globalThis.window = {
    localStorage: {
      getItem: (key: string) =>
        key === WORKBENCH_API_BASE_URL_STORAGE_KEY ? "http://127.0.0.1:4010" : null,
      setItem: () => undefined,
      removeItem: () => undefined,
    },
  } as unknown as Window & typeof globalThis;

  try {
    assert.deepEqual(await requestJson<{ ok: boolean }>("/api/health"), { ok: true });
    assert.deepEqual(seenUrls, ["http://127.0.0.1:4010/api/health"]);
  } finally {
    globalThis.fetch = previousFetch;
    globalThis.window = previousWindow;
  }
});
