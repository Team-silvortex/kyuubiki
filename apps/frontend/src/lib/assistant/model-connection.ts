export type AssistantEndpointValidation = { ok: boolean; normalized?: string; reason?: string; requiresTrust?: boolean; origin?: string };
export type AssistantConnectionCode = "endpoint" | "trust_required" | "credentials" | "unauthorized" | "forbidden" | "not_found" | "rate_limited" | "server" | "network" | "timeout" | "cancelled" | "response_too_large" | "invalid_response";
export class AssistantConnectionError extends Error {
  constructor(public readonly code: AssistantConnectionCode, public readonly status?: number) {
    super(`Model connection: ${code}${status ? ` (HTTP ${status})` : ""}`);
    this.name = "AssistantConnectionError";
  }
}
export type AssistantConnection = {
  baseUrl: string; apiKey?: string; trustedOrigins?: ReadonlySet<string>;
  signal?: AbortSignal; timeoutMs?: number; transport?: "browser" | "native";
};
const sessionTrust = new Set<string>();
const MAX_RESPONSE_BYTES = 1024 * 1024;

export function filterAssistantModels(models: readonly string[], query: string) {
  const terms = query.toLowerCase().trim().split(/\s+/u).filter(Boolean);
  const matched = models.filter(id => terms.every(term => id.toLowerCase().includes(term)));
  return { items: matched.slice(0, 40), matched: matched.length };
}

export function validateAssistantEndpoint(baseUrl: string): AssistantEndpointValidation {
  const reject = () => ({ ok:false, reason:"Use an HTTPS API base URL without credentials, query or fragment; HTTP is only allowed on loopback." });
  try {
    const parsed = new URL(baseUrl.trim());
    const local = ["localhost", "127.0.0.1", "[::1]"].includes(parsed.hostname.toLowerCase());
    if (!baseUrl.trim() || /[\u0000-\u001f\u007f]/u.test(baseUrl) || parsed.username || parsed.password || parsed.search || parsed.hash
      || (parsed.protocol !== "https:" && !(parsed.protocol === "http:" && local))) return reject();
    const path = parsed.pathname.replace(/\/+$/u, "").replace(/\/(?:chat\/completions|models)$/u, "");
    return { ok:true, normalized:parsed.origin + path, requiresTrust:!local, origin:parsed.origin.toLowerCase() };
  } catch { return reject(); }
}

export function sameAssistantEndpoint(left: string, right: string) {
  const a = validateAssistantEndpoint(left), b = validateAssistantEndpoint(right);
  return a.ok && b.ok && a.normalized === b.normalized;
}

export function assistantTrustedOrigins() { return new Set(sessionTrust); }
export function setAssistantEndpointTrusted(baseUrl: string, trusted: boolean) {
  const value = validateAssistantEndpoint(baseUrl);
  if (!value.ok || !value.origin) return;
  if (trusted) sessionTrust.add(value.origin); else sessionTrust.delete(value.origin);
}

export async function assistantTransport(): Promise<"browser" | "native"> {
  if (typeof window === "undefined") return "browser";
  // Capability discovery contains no model URL, key or research data.
  try {
    const response = await fetch("/api/assistant/capabilities", { credentials:"omit", cache:"no-store", redirect:"error", signal:AbortSignal.timeout(2000) });
    if (response.ok && (await response.json()).schema_version === "kyuubiki.model-gateway/v1") return "native";
  } catch { /* Older/web-only frontends retain the direct browser transport. */ }
  return "browser";
}

async function readBoundedJson(response: Response): Promise<unknown> {
  if (Number(response.headers.get("content-length")) > MAX_RESPONSE_BYTES) {
    await response.body?.cancel(); throw new AssistantConnectionError("response_too_large");
  }
  if (!response.body) throw new AssistantConnectionError("invalid_response");
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = []; let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > MAX_RESPONSE_BYTES) { await reader.cancel(); throw new AssistantConnectionError("response_too_large"); }
      chunks.push(value);
    }
  } finally { reader.releaseLock(); }
  const bytes = new Uint8Array(size); let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
  try { return JSON.parse(new TextDecoder("utf-8", { fatal:true }).decode(bytes)); }
  catch { throw new AssistantConnectionError("invalid_response"); }
}

export async function requestAssistantJson(input: AssistantConnection, operation: "models" | "plan", payload?: unknown): Promise<unknown> {
  const validation = validateAssistantEndpoint(input.baseUrl);
  if (!validation.ok || !validation.normalized) throw new AssistantConnectionError("endpoint");
  if (validation.requiresTrust && !input.trustedOrigins?.has(validation.origin!)) throw new AssistantConnectionError("trust_required");
  const key = input.apiKey?.trim() ?? "";
  if (key.length > 8192 || /[^\x21-\x7e]/u.test(key)) throw new AssistantConnectionError("credentials");
  const controller = new AbortController(); let timedOut = false;
  const cancel = () => controller.abort();
  if (input.signal?.aborted) throw new AssistantConnectionError("cancelled");
  input.signal?.addEventListener("abort", cancel, { once:true });
  const timeout = input.timeoutMs ?? (operation === "models" ? 15_000 : 60_000);
  if (!Number.isFinite(timeout) || timeout <= 0 || timeout > 60_000) {
    input.signal?.removeEventListener("abort", cancel); throw new AssistantConnectionError("timeout");
  }
  const timer = setTimeout(() => { timedOut = true; controller.abort(); }, timeout);
  try {
    const native = input.transport === "native";
    const headers: Record<string, string> = { Accept:"application/json" };
    if (native || operation === "plan") headers["Content-Type"] = "application/json";
    if (!native && key) headers.Authorization = `Bearer ${key}`;
    const response = await fetch(native ? "/api/assistant/request" : `${validation.normalized}/${operation === "models" ? "models" : "chat/completions"}`, {
      method:native || operation === "plan" ? "POST" : "GET", headers,
      credentials:"omit", cache:"no-store", redirect:"error", referrerPolicy:"no-referrer", signal:controller.signal,
      body:native ? JSON.stringify({ base_url:validation.normalized, api_key:key, operation, ...(payload ? { payload } : {}) })
        : operation === "plan" ? JSON.stringify(payload) : undefined,
    });
    if (!response.ok) {
      await response.body?.cancel();
      const status = response.status;
      throw new AssistantConnectionError(status === 401 ? "unauthorized" : status === 403 ? "forbidden" : status === 404 || status === 405 ? "not_found" : status === 429 ? "rate_limited" : "server", status);
    }
    return await readBoundedJson(response);
  } catch (error) {
    if (timedOut) throw new AssistantConnectionError("timeout");
    if (input.signal?.aborted) throw new AssistantConnectionError("cancelled");
    if (error instanceof AssistantConnectionError) throw error;
    throw new AssistantConnectionError("network");
  } finally { clearTimeout(timer); input.signal?.removeEventListener("abort", cancel); }
}

export async function listAssistantModels(input: AssistantConnection): Promise<string[]> {
  const payload = await requestAssistantJson(input, "models") as { data?: unknown } | null;
  if (!payload || !Array.isArray(payload.data) || payload.data.length > 5000) throw new AssistantConnectionError("invalid_response");
  const models = new Set<string>();
  for (const item of payload.data) {
    if (!item || typeof item.id !== "string" || !item.id.trim() || item.id.length > 256 || /[\u0000-\u001f\u007f]/u.test(item.id)) throw new AssistantConnectionError("invalid_response");
    models.add(item.id.trim());
  }
  return [...models].sort();
}
