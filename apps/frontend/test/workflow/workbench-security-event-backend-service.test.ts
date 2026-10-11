import test from "node:test";
import assert from "node:assert/strict";

import { createSecurityEventBackendService } from "../../src/lib/workbench/security-event-backend-service-core.ts";
import type {
  SecurityEventEnvelope,
  SecurityEventListPayload,
  SecurityEventRecord,
} from "../../src/lib/api/security-results-types.ts";

function securityEvent(eventId: string): SecurityEventRecord {
  return {
    action: "governance/runtime-drift",
    context: { runtime: "direct_mesh_gui" },
    event_id: eventId,
    event_type: "security_high_risk_action",
    occurred_at: "2026-06-29T00:00:00.000Z",
    risk: "sensitive",
    source: "governance",
    status: "completed",
  };
}

test("security event backend fetches events through transport", async () => {
  const calls: unknown[] = [];
  const service = createSecurityEventBackendService({
    createEvent: async (input) => ({ event: securityEvent(input.event_id) }) satisfies SecurityEventEnvelope,
    fetchEvents: async (filters) => {
      calls.push(filters);
      return { events: [securityEvent("event-a")] } satisfies SecurityEventListPayload;
    },
  });

  const payload = await service.fetchEvents({
    limit: 120,
    risk: "sensitive",
    source: "governance",
  });

  assert.equal(payload.events[0]?.event_id, "event-a");
  assert.deepEqual(calls, [{ limit: 120, risk: "sensitive", source: "governance" }]);
});

test("security event backend creates events through transport", async () => {
  const calls: unknown[] = [];
  const service = createSecurityEventBackendService({
    createEvent: async (input) => {
      calls.push(input);
      return { event: securityEvent(input.event_id) } satisfies SecurityEventEnvelope;
    },
    fetchEvents: async () => ({ events: [] }) satisfies SecurityEventListPayload,
  });

  const payload = await service.createEvent({
    action: "assistant/run-script",
    context: { project_id: "project-a" },
    event_id: "event-42",
    event_type: "security_high_risk_action",
    occurred_at: "2026-06-29T00:00:00.000Z",
    risk: "high",
    source: "assistant",
    status: "blocked",
  });

  assert.equal(payload.event.event_id, "event-42");
  assert.deepEqual(calls, [
    {
      action: "assistant/run-script",
      context: { project_id: "project-a" },
      event_id: "event-42",
      event_type: "security_high_risk_action",
      occurred_at: "2026-06-29T00:00:00.000Z",
      risk: "high",
      source: "assistant",
      status: "blocked",
    },
  ]);
});

const invalidHistories: Array<[string, unknown]> = [
  ["null envelope", null],
  ["missing events", {}],
  ["null events", { events: null }],
  ["object events", { events: {} }],
  ["null entry", { events: [null] }],
  ["array entry", { events: [[]] }],
  ["duplicate identity", { events: [securityEvent("duplicate"), securityEvent("duplicate")] }],
  ...["event_id", "event_type", "source", "action", "risk", "status"].map((field): [string, unknown] =>
    [`invalid ${field}`, { events: [{ ...securityEvent("event"), [field]: " " }] }]),
  ["invalid timestamp", { events: [{ ...securityEvent("event"), occurred_at: "not-a-date" }] }],
  ["null context", { events: [{ ...securityEvent("event"), context: null }] }],
  ["array context", { events: [{ ...securityEvent("event"), context: [] }] }],
  ["missing context", { events: [{ ...securityEvent("event"), context: undefined }] }],
  ["object note", { events: [{ ...securityEvent("event"), note: {} }] }],
  ["invalid optional timestamp", { events: [{ ...securityEvent("event"), inserted_at: "invalid" }] }],
];

for (const [name, payload] of invalidHistories) {
  test(`security event backend rejects ${name} before publishing history`, async () => {
    let reads = 0;
    let writes = 0;
    const service = createSecurityEventBackendService({
      createEvent: async () => { writes += 1; return { event: securityEvent("write") }; },
      fetchEvents: async () => { reads += 1; return payload as SecurityEventListPayload; },
    });
    await assert.rejects(service.fetchEvents(), /SECURITY_EVENT_HISTORY_INVALID/u);
    assert.equal(reads, 1);
    assert.equal(writes, 0);
  });
}

test("security event backend accepts authoritative empty history and extensible event fields without copying", async () => {
  const payload = { events: [{ ...securityEvent("event"), risk: "custom-risk", source: "external-agent",
    status: "custom-status", note: null, context: { nested: { safe: true } } }] };
  const service = createSecurityEventBackendService({
    createEvent: async () => ({ event: securityEvent("write") }),
    fetchEvents: async () => payload,
  });
  assert.equal(await service.fetchEvents(), payload);
  payload.events.length = 0;
  assert.equal(await service.fetchEvents(), payload);
});

test("security event backend preserves the original unavailable error without retrying", async () => {
  const failure = new Error("history unavailable");
  let reads = 0;
  const service = createSecurityEventBackendService({
    createEvent: async () => ({ event: securityEvent("write") }),
    fetchEvents: async () => { reads += 1; throw failure; },
  });
  await assert.rejects(service.fetchEvents(), (error) => error === failure);
  assert.equal(reads, 1);
});
