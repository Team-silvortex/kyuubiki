import test from "node:test";
import assert from "node:assert/strict";
import { requestWorkbenchAssistantPlan } from "@/lib/assistant/openai-compatible";
import { AssistantConnectionError } from "@/lib/assistant/model-connection";

test("assistant plans use the normalized endpoint without forcing optional provider features", async () => {
  const original = globalThis.fetch;
  const input = { baseUrl:"http://localhost:1234/v1/chat/completions", apiKey:"model-secret", model:"custom-model", prompt:"help", snapshot:{} as never, localHints:[] };
  globalThis.fetch = async (url, options) => {
    assert.equal(url, "http://localhost:1234/v1/chat/completions");
    const body = JSON.parse(String(options?.body));
    assert.equal(body.model, "custom-model");
    assert.equal(body.response_format, undefined);
    assert.equal(body.temperature, undefined);
    assert.ok(!String(options?.body).includes(input.apiKey));
    return Response.json({ choices:[{ message:{ content:JSON.stringify({ summary:"ready", rationale:"review first", suggested_actions:[] }) } }] });
  };
  try {
    assert.equal((await requestWorkbenchAssistantPlan(input)).summary, "ready");
    for (const content of ["not JSON", "null", '{"summary":"x","rationale":"y","suggested_actions":[null]}', '{"summary":"x","rationale":"y","suggested_actions":[{"action":"unknown","reason":"x"}]}']) {
      globalThis.fetch = async () => Response.json({ choices:[{ message:{ content } }] });
      await assert.rejects(requestWorkbenchAssistantPlan(input), (error: unknown) => error instanceof AssistantConnectionError && error.code === "invalid_response");
    }
  } finally { globalThis.fetch = original; }
});
