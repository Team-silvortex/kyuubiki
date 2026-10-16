import { checkpointDigest, validateCheckpointReservation, type CheckpointJournal } from "./checkpoint-journal.ts";
import { captureCheckpointAuthority, type CheckpointAuthorityOptions } from "./checkpoint-authority.ts";

type PendingCheckpoint = { requestId: string; inFlight?: Promise<unknown> };

export type CheckpointRetryOptions = CheckpointAuthorityOptions & { limit?: number; journal?: CheckpointJournal };
export function createCheckpointRetry(options: CheckpointRetryOptions = {}) {
  const pending = new Map<string, PendingCheckpoint>();

  return async function checkpoint<Input extends { request_id?: string }, Result>(
    operation: "model" | "version", parentId: string, input: Input,
    write: (snapshot: Input) => Promise<Result>,
  ): Promise<Result> {
    if (!globalThis.crypto?.subtle) throw new Error("checkpoint:secure_context_required");
    const { scope: requestScope, check } = captureCheckpointAuthority(options);
    const serialized = JSON.stringify(input, (_key, value) => {
      if (value && typeof value === "object" && !Array.isArray(value)) {
        return Object.fromEntries(Object.entries(value).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0));
      }
      return value;
    });
    // Identity and error handling must use the same snapshot as the fingerprint.
    const snapshot = JSON.parse(serialized) as Input;
    const explicitRequestId = snapshot.request_id;
    const key = await checkpointDigest(
      JSON.stringify([requestScope, operation, parentId, serialized]),
    );
    check();
    let request = pending.get(key);
    const isNew = !request;
    if (!request) {
      // Never silently evict an uncertain write and generate a new key for its retry.
      if (pending.size >= (options.limit ?? 64)) throw new Error("checkpoint:recovery_capacity_reached");
      request = { requestId: explicitRequestId ?? crypto.randomUUID() };
      pending.set(key, request);
    }
    if (request.inFlight) {
      const response = await request.inFlight as Result;
      check(); return response;
    }
    const attempt = request;
    attempt.inFlight = Promise.resolve().then(async () => {
      let dispatched = false;
      try {
        if (options.journal) {
          const scopeHash = await checkpointDigest(requestScope);
          check();
          const expected = {
            key, scope: scopeHash, operation, parentId,
            requestId: attempt.requestId, createdAt: Date.now(),
          };
          const reserved = await options.journal.reserve({ ...expected });
          check();
          const durable = validateCheckpointReservation(reserved, expected);
          attempt.requestId = durable.intent.requestId;
        }
        check();
        dispatched = true;
        const response = await write({ ...snapshot, request_id: attempt.requestId });
        check();
        // Storage-only cleanup failure does not turn a valid commit into a failed save.
        // Ownership loss still rejects delivery and must preserve recovery metadata.
        await options.journal?.remove(key, attempt.requestId, check).catch(() => undefined);
        check();
        pending.delete(key);
        return response;
      } catch (error) {
        attempt.inFlight = undefined;
        const status = (error as { statusCode?: number } | null)?.statusCode;
        // With persistence, memory only coalesces in-flight calls. A later
        // rejection cannot prove that an earlier attempt never committed.
        if (options.journal) pending.delete(key);
        else if (!dispatched && isNew) pending.delete(key);
        else if (explicitRequestId || [400, 401, 403, 404, 405, 413, 415, 422].includes(status ?? 0)) {
          try { check(); pending.delete(key); } catch { /* Retired replies cannot release uncertain identities. */ }
        }
        throw error;
      }
    });
    return attempt.inFlight as Promise<Result>;
  };
}
