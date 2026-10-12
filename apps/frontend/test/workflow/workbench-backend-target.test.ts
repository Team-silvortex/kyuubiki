import test from "node:test";
import assert from "node:assert/strict";
import {
  persistWorkbenchApiBaseUrl,
  readPersistedWorkbenchApiBaseUrl,
  resolveWorkbenchApiUrl,
  resolveWorkbenchBackendTarget,
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

for (const failure of ["storage-access", "target-read", "settings-read"] as const) {
  test(`unreadable ${failure} is safe to display but blocks HTTP without selecting a fallback`, async () => {
    const previousWindow = globalThis.window;
    const previousFetch = globalThis.fetch;
    const previousEnvironment = process.env.NEXT_PUBLIC_KYUUBIKI_API_BASE_URL;
    process.env.NEXT_PUBLIC_KYUUBIKI_API_BASE_URL = "https://must-not-fall-back.invalid";
    let unavailable = true;
    const storage = {
      getItem: (key: string) => {
        if (unavailable && ((failure === "target-read" && key === WORKBENCH_API_BASE_URL_STORAGE_KEY)
          || (failure === "settings-read" && key === "kyuubiki-workbench-settings"))) throw new Error("private diagnostic must not escape");
        return null;
      },
    };
    globalThis.window = { location: { search: "" }, get localStorage() {
      if (unavailable && failure === "storage-access") throw new Error("private diagnostic must not escape");
      return storage;
    } } as unknown as Window & typeof globalThis;
    globalThis.fetch = (async () => { assert.fail("unresolved target must not reach fetch"); }) as typeof fetch;
    try {
      assert.deepEqual(resolveWorkbenchBackendTarget(), { baseUrl: "", source: "unavailable" });
      if (failure !== "settings-read") assert.equal(readPersistedWorkbenchApiBaseUrl(), "");
      assert.throws(() => resolveWorkbenchApiUrl("/api/v1/jobs"), /BACKEND_TARGET_UNAVAILABLE/u);
      await assert.rejects(requestJson("/api/v1/jobs", { method: "PATCH" }), (error: Error) => {
        assert.match(error.message, /BACKEND_TARGET_UNAVAILABLE/u);
        assert.doesNotMatch(error.message, /private diagnostic/u);
        return true;
      });
      unavailable = false;
      assert.equal(resolveWorkbenchBackendTarget().source, "environment");
    } finally {
      globalThis.fetch = previousFetch;
      globalThis.window = previousWindow;
      if (previousEnvironment === undefined) delete process.env.NEXT_PUBLIC_KYUUBIKI_API_BASE_URL;
      else process.env.NEXT_PUBLIC_KYUUBIKI_API_BASE_URL = previousEnvironment;
    }
  });
}

test("an explicit query target remains usable when lower-priority storage is unreadable", () => {
  const previousWindow = globalThis.window;
  globalThis.window = { location: { search: "?apiBaseUrl=https%3A%2F%2Fquery.invalid" },
    get localStorage() { throw new Error("unavailable storage"); } } as unknown as Window & typeof globalThis;
  try {
    assert.deepEqual(resolveWorkbenchBackendTarget(), { baseUrl: "https://query.invalid", source: "query" });
    assert.equal(resolveWorkbenchApiUrl("/api/v1/jobs"), "https://query.invalid/api/v1/jobs");
    assert.equal(readPersistedWorkbenchApiBaseUrl(), "");
  } finally { globalThis.window = previousWindow; }
});
