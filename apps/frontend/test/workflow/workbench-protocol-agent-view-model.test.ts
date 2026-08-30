import assert from "node:assert/strict";
import test from "node:test";

import { buildProtocolAgentCards } from "../../src/lib/workbench/view-model-protocol-agents.ts";

const labels = new Proxy({}, { get: (_target, key) => String(key) }) as any;

test("protocol agent cards deduplicate capability tags and expose a bounded preview", () => {
  const [card] = buildProtocolAgentCards({
    agents: [
      {
        id: "agent-a",
        host: "127.0.0.1",
        port: 5001,
        descriptor: {
          runtime: { health_score: 100 },
          capabilities: [
            { id: "heat", tags: ["thermal", "bar", "cpu"] },
            { id: "mesh", tags: ["thermal", "bar", "mesh"] },
          ],
        },
      },
    ] as any,
    labels,
    clusterHealthTone: () => "quiet",
    peerStatusLabel: (status) => status ?? "--",
  });

  assert.deepEqual(card?.chips.map((chip) => chip.label), ["thermal", "bar", "cpu", "mesh"]);
  assert.equal(card?.chipPreviewLimit, 10);
  assert.equal(card?.showMoreLabel, "showMore");
  assert.equal(card?.showLessLabel, "showLess");
  assert.equal(card?.metrics.find((metric) => metric.label === "clusterHealth")?.value, "100%");
});
