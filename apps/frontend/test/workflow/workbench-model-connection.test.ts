import test from "node:test";
import assert from "node:assert/strict";
import { validateAssistantEndpoint, listAssistantModels, requestAssistantJson, AssistantConnectionError, sameAssistantEndpoint, filterAssistantModels } from "@/lib/assistant/model-connection";

test("model picker searches all results while bounding rendered choices", () => {
  const models = Array.from({ length:5000 }, (_, i) => `research-model-${i}`);
  assert.equal(filterAssistantModels(models, "").items.length, 40);
  assert.equal(filterAssistantModels(models, "").matched, 5000);
  assert.deepEqual(filterAssistantModels(models, " RESEARCH 4999 ").items, ["research-model-4999"]);
  assert.deepEqual(filterAssistantModels(models, "not-present"), { items:[], matched:0 });
  assert.equal(models.length, 5000);
});

test("model URLs accept base and full chat paths but reject embedded credentials and ambiguous URLs", () => {
  for (const value of ["https://model.example/v1/", " https://model.example/v1/chat/completions/ ", "https://model.example/v1/models"]) {
    assert.equal(validateAssistantEndpoint(value).normalized, "https://model.example/v1");
  }
  for (const value of ["https://user:secret@model.example/v1", "https://model.example/v1?key=secret", "https://model.example/v1#key", "http://model.example/v1", "file:///private/key"]) {
    assert.equal(validateAssistantEndpoint(value).ok, false);
  }
  assert.equal(validateAssistantEndpoint("http://[::1]:1234/v1").ok, true);
  assert.equal(sameAssistantEndpoint("https://model.example/v1", "https://model.example/v1/chat/completions"), true);
  assert.equal(sameAssistantEndpoint("https://model.example/v1", "https://other.example/v1"), false);
});

test("model discovery is explicit, bounded and does not send a study or Agent token", async () => {
  const original = globalThis.fetch;
  const calls: Array<{ url: string; options?: RequestInit }> = [];
  globalThis.fetch = async (url, options) => {
    calls.push({ url: String(url), options });
    return Response.json({ data: [{ id:"z-model" }, { id:"a-model" }, { id:"z-model" }] });
  };
  try {
    const input = { baseUrl:"https://model.example/v1/chat/completions", apiKey:"model-secret", trustedOrigins:new Set(["https://model.example"]) };
    assert.deepEqual(await listAssistantModels(input), ["a-model", "z-model"]);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].url, "https://model.example/v1/models");
    assert.equal(calls[0].options?.method, "GET");
    assert.equal(calls[0].options?.body, undefined);
    assert.equal(calls[0].options?.redirect, "error");
    assert.equal(calls[0].options?.credentials, "omit");
    assert.deepEqual(calls[0].options?.headers, { Accept:"application/json", Authorization:"Bearer model-secret" });
    await assert.rejects(listAssistantModels({ ...input, trustedOrigins:new Set() }), (error: unknown) => error instanceof AssistantConnectionError && error.code === "trust_required");
    assert.equal(calls.length, 1);
  } finally { globalThis.fetch = original; }
});

test("model connection errors never echo vendor response bodies or API keys", async () => {
  const original = globalThis.fetch;
  try {
    for (const status of [401, 403, 404, 429, 500]) {
      globalThis.fetch = async () => new Response("Authorization Bearer secret-user-key", { status });
      await assert.rejects(listAssistantModels({ baseUrl:"http://localhost:1234/v1", apiKey:"secret-user-key" }), (error: unknown) => {
        assert.ok(error instanceof AssistantConnectionError);
        assert.equal(error.status, status);
        assert.ok(!error.message.includes("secret-user-key"));
        return true;
      });
    }
    globalThis.fetch = async () => new Response("x".repeat(1024 * 1024 + 1));
    await assert.rejects(listAssistantModels({ baseUrl:"http://localhost:1234/v1" }), (error: unknown) => error instanceof AssistantConnectionError && error.code === "response_too_large");
  } finally { globalThis.fetch = original; }
});

test("model calls have a deadline, support cancellation, and never retry", async () => {
  const original = globalThis.fetch;
  let calls = 0;
  globalThis.fetch = async (_url, options) => {
    calls++;
    return new Promise((_resolve, reject) => options?.signal?.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError")), { once:true }));
  };
  try {
    await assert.rejects(requestAssistantJson({ baseUrl:"http://localhost:1234/v1", timeoutMs:10 }, "models"), (error: unknown) => error instanceof AssistantConnectionError && error.code === "timeout");
    const controller = new AbortController(); controller.abort();
    await assert.rejects(listAssistantModels({ baseUrl:"http://localhost:1234/v1", signal:controller.signal }), (error: unknown) => error instanceof AssistantConnectionError && error.code === "cancelled");
    assert.equal(calls, 1);
  } finally { globalThis.fetch = original; }
});
