import test from "node:test";
import assert from "node:assert/strict";
import { requestJsonWithContext, requestTextWithContext, type WorkbenchApiRequestContext } from "@/lib/api/core";
import { WorkbenchRequestError } from "@/lib/api/request-errors";
import { createCheckpointRetry } from "../../src/lib/workbench/checkpoint-retry.ts";

const context: WorkbenchApiRequestContext = {
  resolveUrl: (url) => `https://runtime.example${url}`,
  buildAuthHeaders: () => ({}),
};

// A bounded fallback makes the old header-only timeout fail instead of hanging tests.
function delayedBody(init: RequestInit | undefined, status: number, json: boolean) {
  let release: ReturnType<typeof setTimeout>;
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      const abort = () => {
        clearTimeout(release);
        controller.error(init?.signal?.reason);
      };
      if (init?.signal?.aborted) { abort(); return; }
      init?.signal?.addEventListener("abort", abort, { once: true });
      controller.enqueue(new TextEncoder().encode(json ? '{"ok":' : "partial "));
      release = setTimeout(() => {
        init?.signal?.removeEventListener("abort", abort);
        controller.enqueue(new TextEncoder().encode(json ? "true}" : "complete"));
        controller.close();
      }, 60);
    },
  });
  return new Response(stream, { status, headers: { "content-type": json ? "application/json" : "text/plain" } });
}

for (const json of [true, false]) {
  for (const status of [201, 503]) {
    test(`response body remains inside the timeout budget; json=${json}, status=${status}`, async (t) => {
      let signal: AbortSignal | undefined | null;
      t.mock.method(globalThis, "fetch", async (_url: RequestInfo | URL, init?: RequestInit) => {
        signal = init?.signal;
        return delayedBody(init, status, json);
      });
      const read = json ? requestJsonWithContext : requestTextWithContext;
      await assert.rejects(read(context, "/api/v1/models/model/versions", undefined, 5), (error: unknown) => {
        assert.ok(error instanceof WorkbenchRequestError);
        assert.equal(error.kind, "timeout");
        assert.equal(error.retryable, true);
        assert.equal(error.statusCode, undefined, "an unread body is not a confirmed HTTP rejection");
        return true;
      });
      assert.equal(signal?.aborted, true);
    });
  }

  test(`external cancellation remains connected while reading the body; json=${json}`, async (t) => {
    const caller = new AbortController();
    let bodyStarted!: () => void;
    const started = new Promise<void>((resolve) => { bodyStarted = resolve; });
    t.mock.method(globalThis, "fetch", async (_url: RequestInfo | URL, init?: RequestInit) => {
      const response = delayedBody(init, 200, json);
      bodyStarted();
      return response;
    });
    const read = json ? requestJsonWithContext : requestTextWithContext;
    const request = read(context, "/api/v1/projects", { signal: caller.signal }, 1000);
    await started;
    // Let fetch settle so this cancels body consumption, not response headers.
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
    caller.abort(new DOMException("caller cancelled body", "AbortError"));
    await assert.rejects(request, (error: unknown) => {
      assert.ok(error instanceof WorkbenchRequestError);
      assert.equal(error.kind, "unknown");
      assert.match(error.message, /caller cancelled body/);
      assert.equal(error.retryable, false);
      return true;
    });
  });

  test(`transport failures in the body are not successful empty responses; json=${json}`, async (t) => {
    t.mock.method(globalThis, "fetch", async () => new Response(new ReadableStream({
      start(controller) { controller.error(new TypeError("network error reading response")); },
    }), { status: 201, headers: { "content-type": json ? "application/json" : "text/plain" } }));
    const read = json ? requestJsonWithContext : requestTextWithContext;
    await assert.rejects(read(context, "/api/v1/models/model/versions"), (error: unknown) => {
      assert.ok(error instanceof WorkbenchRequestError);
      assert.equal(error.kind, "offline");
      assert.equal(error.retryable, true);
      return true;
    });
  });
}

test("a timed-out committed response retains the automatic checkpoint key for an explicit retry", async (t) => {
  const retry = createCheckpointRetry();
  const keys: string[] = [];
  const committed = new Map<string, { version: string }>();
  t.mock.method(globalThis, "fetch", async (_url: RequestInfo | URL, init?: RequestInit) => {
    const body = JSON.parse(String(init?.body));
    keys.push(body.request_id);
    if (!committed.has(body.request_id)) committed.set(body.request_id, { version: `v${committed.size + 1}` });
    if (keys.length === 1) return delayedBody(init, 201, true);
    return Response.json(committed.get(body.request_id), { status: 201 });
  });
  const input = { payload: { h: 1.5 }, request_id: undefined as string | undefined };
  const save = () => retry("version", "model", input, (snapshot) =>
    requestJsonWithContext(context, "/api/v1/models/model/versions", {
      method: "POST", body: JSON.stringify(snapshot),
    }, 5));
  await assert.rejects(save(), (error: unknown) => error instanceof WorkbenchRequestError && error.kind === "timeout");
  assert.equal(keys.length, 1, "timeout never triggers an automatic write");
  assert.deepEqual(await save(), { version: "v1" });
  assert.equal(keys[1], keys[0]);
  assert.equal(committed.size, 1);
});

test("completed JSON parsing keeps empty and malformed payload compatibility", async (t) => {
  for (const body of ["", "not-json", '{"incomplete":']) {
    t.mock.method(globalThis, "fetch", async () => new Response(body, { headers: { "content-type": "application/json" } }));
    assert.deepEqual(await requestJsonWithContext(context, "/api/v1/projects"), {});
    t.mock.restoreAll();
  }
});
