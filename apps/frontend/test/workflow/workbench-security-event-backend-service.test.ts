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
