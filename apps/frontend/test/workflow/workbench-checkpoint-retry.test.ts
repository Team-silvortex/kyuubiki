import test from "node:test";
import assert from "node:assert/strict";
import { createCheckpointRetry } from "../../src/lib/workbench/checkpoint-retry.ts";

const input = { name: "Truss", payload: { h: 1.5 }, request_id: undefined as string | undefined };

test("lost response retains the request key; a confirmed save starts a new checkpoint next time", async () => {
  const retry = createCheckpointRetry();
  const keys: string[] = [];
  const saved = new Map<string, number>();
  let loseResponse = true;
  const write = async (body: typeof input) => {
    keys.push(body.request_id!);
    if (!saved.has(body.request_id!)) saved.set(body.request_id!, saved.size + 1);
    if (loseResponse) { loseResponse = false; throw new Error("response lost after commit"); }
    return saved.get(body.request_id!);
  };
  await assert.rejects(retry("version", "model", input, write), /response lost/);
  assert.equal(await retry("version", "model", input, write), 1);
  assert.equal(saved.size, 1);
  assert.equal(keys[0], keys[1]);
  assert.equal(await retry("version", "model", input, write), 2);
  assert.notEqual(keys[1], keys[2]);
});

test("concurrent identical saves share one in-flight request", async (context) => {
  context.mock.method(crypto.subtle, "digest", async () => new Uint8Array(32).buffer);
  const retry = createCheckpointRetry();
  let calls = 0;
  let release!: () => void;
  let started!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  const writing = new Promise<void>((resolve) => { started = resolve; });
  const requests = Array.from({ length: 8 }, () => retry("version", "model", input, async () => {
    calls += 1;
    started();
    await held;
    return "version";
  }));
  await writing;
  release();
  assert.deepEqual(await Promise.all(requests), Array(8).fill("version"));
  assert.equal(calls, 1);
});

test("retry fingerprint scopes backend, parent, operation and content without retaining geometry", async () => {
  let scope = "server-a";
  const retry = createCheckpointRetry({ scope: () => scope });
  const keys: string[] = [];
  const fail = async (body: typeof input) => { keys.push(body.request_id!); throw new Error("lost"); };
  await assert.rejects(retry("version", "a", input, fail));
  await assert.rejects(retry("version", "a", { payload: { h: 1.5 }, name: "Truss", request_id: undefined }, fail));
  assert.equal(keys[0], keys[1]);
  await assert.rejects(retry("version", "b", input, fail));
  await assert.rejects(retry("model", "a", input, fail));
  await assert.rejects(retry("version", "a", { ...input, payload: { h: 2 } }, fail));
  scope = "server-b";
  await assert.rejects(retry("version", "a", input, fail));
  assert.equal(new Set(keys).size, 5);
});

test("bounded retry storage fails closed instead of forgetting an uncertain write", async () => {
  const retry = createCheckpointRetry({ limit: 1 });
  let original = "";
  await assert.rejects(retry("model", "project", input, async (body) => {
    original = body.request_id!;
    throw new Error("lost");
  }));
  await assert.rejects(retry("model", "other", input, async () => "must not write"), /recovery_capacity_reached/);
  assert.equal(await retry("model", "project", input, async (body) => body.request_id), original);
  assert.equal(await retry("model", "other", input, async () => "recovered"), "recovered");
});

test("explicit request ids survive a new frontend retry manager", async () => {
  const supplied = { ...input, request_id: "durable-explicit-request-0001" };
  await assert.rejects(createCheckpointRetry()("model", "p", supplied, async () => { throw new Error("lost"); }));
  const result = await createCheckpointRetry()("model", "p", supplied, async (body) => body.request_id);
  assert.equal(result, supplied.request_id);
});

test("payload mutation while hashing cannot change the request associated with its fingerprint", async () => {
  const retry = createCheckpointRetry();
  const mutable = structuredClone(input);
  const request = retry("model", "project", mutable, async (body) => body);
  mutable.payload.h = 9;
  assert.equal((await request).payload.h, 1.5);
});

test("backend changes while preparing the request prevent sending it to a different authority", async () => {
  let scope = "a";
  const retry = createCheckpointRetry({ scope: () => scope });
  let writes = 0;
  const request = retry("model", "project", input, async () => { writes += 1; });
  scope = "b";
  await assert.rejects(request, /context_changed/);
  assert.equal(writes, 0);
});

test("a confirmed validation rejection releases capacity rather than leaking pending requests", async () => {
  const retry = createCheckpointRetry({ limit: 1 });
  await assert.rejects(retry("model", "project", input, async () => {
    throw Object.assign(new Error("invalid"), { statusCode: 422 });
  }));
  assert.equal(await retry("model", "other", input, async () => "saved"), "saved");
});
