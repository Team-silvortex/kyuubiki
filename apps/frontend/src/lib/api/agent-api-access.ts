import { advanceWorkbenchAuthorityRevision } from "./authority-revisions";

type AgentApiAccess = { endpoint: string; serverName: string; caPem: string; token: string };
let access: (AgentApiAccess & { gateway: string }) | null = null;
const routes = new Set(["/api/direct-mesh/agents", "/api/direct-mesh/solve", "/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect", "/api/direct-mesh/tasks/list", "/api/direct-mesh/tasks/cancel"]);

function safeGateway(value: string): string {
  const url = new URL(value);
  const loopback = url.hostname === "localhost" || url.hostname === "127.0.0.1" || url.hostname === "[::1]";
  if ((url.protocol !== "https:" && !(url.protocol === "http:" && loopback)) || url.username || url.password || url.search || url.hash) {
    throw new Error("Agent credentials require HTTPS or a loopback gateway.");
  }
  return url.toString().replace(/\/+$/u, "");
}

export function configureAgentApiAccess(input: AgentApiAccess, gateway: string) {
  if (!/^[A-Za-z0-9_-]{32,256}$/u.test(input.token)
    || !/^[A-Za-z0-9.-]+:[0-9]{1,5}$/u.test(input.endpoint) || Number(input.endpoint.split(":")[1]) < 1 || Number(input.endpoint.split(":")[1]) > 65535
    || !/^[A-Za-z0-9.:-]{1,253}$/u.test(input.serverName)
    || input.caPem.length > 32 * 1024 || !input.caPem.includes("-----BEGIN CERTIFICATE-----")
    || !input.caPem.includes("-----END CERTIFICATE-----") || /[^\x09\x0a\x0d\x20-\x7e]/u.test(input.caPem)) {
    throw new Error("A pinned Agent address, certificate name, PEM trust certificate and valid API token are required.");
  }
  const next = { ...input, gateway: safeGateway(gateway) };
  access = next;
  advanceWorkbenchAuthorityRevision("agentApiToken");
}

export function clearAgentApiAccess() {
  access = null;
  advanceWorkbenchAuthorityRevision("agentApiToken");
}

export function agentApiAccessSummary() {
  return access ? { configured: true, endpoint: access.endpoint, serverName: access.serverName, gateway: access.gateway } : { configured: false };
}

export function agentApiHeaders(route: string, gateway: string): Record<string, string> {
  if (!access || !routes.has(route)) return {};
  if (safeGateway(gateway) !== access.gateway) throw new Error("Agent gateway changed; configure its credentials again before dispatch.");
  return {
    "x-kyuubiki-agent-token": access.token,
    "x-kyuubiki-agent-endpoint": access.endpoint,
    "x-kyuubiki-agent-server-name": access.serverName,
    "x-kyuubiki-agent-ca": btoa(access.caPem),
  };
}
