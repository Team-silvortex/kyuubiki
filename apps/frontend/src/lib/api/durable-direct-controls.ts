import { DURABLE_DIRECT_SCHEMA, validateDurableDirectHandle, validateDurableStoreIdentity, type DurableDirectHandle } from "./durable-direct-client.ts";

export type DurableDirectIndexEntry = {
  schema_version: typeof DURABLE_DIRECT_SCHEMA;
  execution_id: string;
  request_id: string;
  process_instance_id: string;
  request_digest: string;
  method: string;
};
export type DurableDirectIndex = {
  store_id?: string;
  schema_version: "kyuubiki.agent-durable-execution-index/v1";
  executions: DurableDirectIndexEntry[];
  next_cursor: string | null;
  automatic_replay_authorized: false;
};
export type DurableDirectCancelTarget = DurableDirectHandle & Pick<DurableDirectIndexEntry, "process_instance_id" | "request_digest">;
export type DurableDirectCancellation = {
  store_id?: string;
  schema_version: "kyuubiki.agent-durable-cancellation/v1";
  execution_id: string;
  process_instance_id: string;
  request_digest: string;
  status: "requested" | "not_running" | "owner_not_observed";
  cancel_registered: boolean;
  execution_terminal_confirmed: false;
  automatic_replay_authorized: false;
};
type Envelope<T> = { recovery: T; endpoint: string; gateway_runtime: "rust-native" };
type Request = <T>(url: string, init?: RequestInit, timeoutMs?: number) => Promise<T>;
const hex = (value: unknown, length: number) => typeof value === "string" && value.length === length && /^[0-9a-f]+$/u.test(value);
const text = (value: unknown) => typeof value === "string" && value.length > 0 && new TextEncoder().encode(value).length <= 256;
const record = (value: unknown): Record<string, unknown> => value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};

export function validateDurableDirectIndex(value: unknown, endpoint: string, limit: number, after: string | null, storeId?: string): Envelope<DurableDirectIndex> {
  const envelope = record(value), index = record(envelope.recovery);
  validateDurableStoreIdentity(index, storeId);
  const rows = index.executions;
  if (envelope.gateway_runtime !== "rust-native" || envelope.endpoint !== endpoint
    || index.schema_version !== "kyuubiki.agent-durable-execution-index/v1" || index.automatic_replay_authorized !== false
    || !Array.isArray(rows) || rows.length > limit) throw new Error("Invalid durable execution index.");
  let previous = after ?? "";
  for (const entry of rows) {
    const row = record(entry);
    if (Object.keys(row).length !== 6 || row.schema_version !== DURABLE_DIRECT_SCHEMA
      || !hex(row.execution_id, 32) || !hex(row.request_digest, 64)
      || !text(row.request_id) || !text(row.process_instance_id) || !text(row.method)
      || (row.execution_id as string) <= previous) throw new Error("Invalid durable execution index entry.");
    previous = row.execution_id as string;
  }
  if (index.next_cursor !== null && (rows.length !== limit || !rows.length || index.next_cursor !== previous)) {
    throw new Error("Invalid durable execution index cursor.");
  }
  return value as Envelope<DurableDirectIndex>;
}

export function validateDurableDirectCancellation(value: unknown, target: DurableDirectCancelTarget): Envelope<DurableDirectCancellation> {
  const envelope = record(value), receipt = record(envelope.recovery);
  validateDurableStoreIdentity(receipt, target.store_id);
  if (envelope.gateway_runtime !== "rust-native" || envelope.endpoint !== target.endpoint
    || receipt.schema_version !== "kyuubiki.agent-durable-cancellation/v1"
    || receipt.execution_id !== target.execution_id || receipt.process_instance_id !== target.process_instance_id
    || receipt.request_digest !== target.request_digest || receipt.automatic_replay_authorized !== false
    || receipt.execution_terminal_confirmed !== false
    || !["requested", "not_running", "owner_not_observed"].includes(receipt.status as string)
    || receipt.cancel_registered !== (receipt.status === "requested")) {
    throw new Error("Invalid durable cancellation receipt; inspect the original execution.");
  }
  return value as Envelope<DurableDirectCancellation>;
}

export function createDurableDirectControlClient(request: Request) {
  async function exchange(path: string, body: Record<string, unknown>) {
    return request<unknown>(path, { method: "POST", cache: "no-store", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
  }
  return {
    async listDurableDirectExecutions(endpoint: string, limit = 24, after: string | null = null, storeId?: string) {
      validateDurableDirectHandle({ execution_id: "0".repeat(32), endpoint, store_id: storeId });
      if (!Number.isInteger(limit) || limit < 1 || limit > 64 || (after !== null && !hex(after, 32))) throw new Error("Invalid durable execution page.");
      return validateDurableDirectIndex(await exchange("/api/direct-mesh/tasks/list", { endpoints: [endpoint], limit, after, expected_store_id: storeId }), endpoint, limit, after, storeId);
    },
    async cancelDurableDirectExecution(target: DurableDirectCancelTarget) {
      const pinned = { ...target };
      validateDurableDirectHandle(pinned);
      if (!text(pinned.process_instance_id) || !hex(pinned.request_digest, 64)) throw new Error("Inspect the execution identity before requesting cancellation.");
      return validateDurableDirectCancellation(await exchange("/api/direct-mesh/tasks/cancel", {
        endpoints: [pinned.endpoint], execution_id: pinned.execution_id,
        process_instance_id: pinned.process_instance_id, request_digest: pinned.request_digest,
        expected_store_id: pinned.store_id,
      }), pinned);
    },
  };
}
