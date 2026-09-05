import test from "node:test";
import assert from "node:assert/strict";

import { requestJsonWithContext, requestTextWithContext, type WorkbenchApiRequestContext } from "@/lib/api/core";
import { WorkbenchRequestError } from "@/lib/api/request-errors";

function installFetch(response: Response, seen: { urls: string[]; tokens: string[] }) {
  const previousFetch = globalThis.fetch;
  globalThis.fetch = (async (url: RequestInfo | URL, init?: RequestInit) => {
    seen.urls.push(String(url));
    seen.tokens.push(new Headers(init?.headers).get("x-kyuubiki-token") ?? "");
    return response;
  }) as typeof fetch;
  return () => {
    globalThis.fetch = previousFetch;
  };
}

const context: WorkbenchApiRequestContext = {
  resolveUrl: (url) => `https://runtime.example${url}`,
  buildAuthHeaders: () => ({ "x-kyuubiki-token": "context-token" }),
};

test("requestJsonWithContext uses injected backend and auth providers", async () => {
  const seen = { urls: [] as string[], tokens: [] as string[] };
  const restoreFetch = installFetch(
    new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { "content-type": "application/json" },
    }),
    seen,
  );

  try {
    assert.deepEqual(await requestJsonWithContext<{ ok: boolean }>(context, "/api/health"), { ok: true });
    assert.deepEqual(seen, {
      urls: ["https://runtime.example/api/health"],
      tokens: ["context-token"],
    });
  } finally {
    restoreFetch();
  }
});

test("requestTextWithContext uses injected backend and auth providers", async () => {
  const seen = { urls: [] as string[], tokens: [] as string[] };
  const restoreFetch = installFetch(new Response("ok", { status: 200 }), seen);

  try {
    assert.equal(await requestTextWithContext(context, "/api/v1/export/security-events.csv"), "ok");
    assert.deepEqual(seen, {
      urls: ["https://runtime.example/api/v1/export/security-events.csv"],
      tokens: ["context-token"],
    });
  } finally {
    restoreFetch();
  }
});

test("requestJsonWithContext reports signal timeouts as retryable timeout failures", async () => {
  const previousFetch = globalThis.fetch;
  globalThis.fetch = ((_url: RequestInfo | URL, init?: RequestInit) => new Promise((_resolve, reject) => {
    init?.signal?.addEventListener("abort", () => reject(init.signal?.reason), { once: true });
  })) as typeof fetch;

  try {
    await assert.rejects(
      requestJsonWithContext(context, "/api/slow", undefined, 5),
      (error: unknown) => {
        assert.ok(error instanceof WorkbenchRequestError);
        assert.equal(error.kind, "timeout");
        assert.equal(error.retryable, true);
        return true;
      },
    );
  } finally {
    globalThis.fetch = previousFetch;
  }
});
