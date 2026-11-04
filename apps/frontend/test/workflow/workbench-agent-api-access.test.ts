import test from "node:test";
import assert from "node:assert/strict";
import { configureAgentApiAccess, clearAgentApiAccess, agentApiAccessSummary, agentApiHeaders } from "@/lib/api/agent-api-access";

const config = { endpoint: "127.0.0.1:5001", serverName: "agent.example", caPem: "-----BEGIN CERTIFICATE-----\nZmFrZQ==\n-----END CERTIFICATE-----", token: "test-token-012345678901234567890123" };

test("Agent credentials are pinned to the gateway and never appear in summaries", () => {
  try {
    configureAgentApiAccess(config, "http://127.0.0.1:3000");
    const headers = agentApiHeaders("/api/direct-mesh/agents", "http://127.0.0.1:3000");
    assert.equal(headers["x-kyuubiki-agent-token"], config.token);
    assert.equal(headers["x-kyuubiki-agent-endpoint"], config.endpoint);
    assert.ok(!JSON.stringify(agentApiAccessSummary()).includes(config.token));
    assert.throws(() => agentApiHeaders("/api/direct-mesh/solve", "https://different.example"), /changed/);
    for (const route of ["/api/v1/jobs", "/api/direct-mesh-evil/agents", "https://evil.example/api/direct-mesh/agents", "/api/direct-mesh/results/123"]) {
      assert.deepEqual(agentApiHeaders(route, "http://127.0.0.1:3000"), {});
    }
  } finally { clearAgentApiAccess(); }
  assert.deepEqual(agentApiHeaders("/api/direct-mesh/agents", "http://127.0.0.1:3000"), {});
});

test("Agent credentials reject cleartext remote gateways and incomplete config", () => {
  for (const gateway of ["http://192.168.1.20:3000", "http://127.0.0.1.evil:3000", "ftp://localhost", "https://user:pass@example.com", "https://example.com/?target=x"]) {
    assert.throws(() => configureAgentApiAccess(config, gateway));
  }
  for (const patch of [{ token:"short" }, { caPem:"no PEM" }, { serverName:"" }, { endpoint:"one:5001,two:5002" }]) {
    assert.throws(() => configureAgentApiAccess({ ...config, ...patch }, "https://trusted.example"));
  }
});
