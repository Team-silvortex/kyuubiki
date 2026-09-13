import { checkpointDigest, type CheckpointJournal } from "./checkpoint-journal.ts";

type PendingCheckpoint = { requestId: string; inFlight?: Promise<unknown> };

export function createCheckpointRetry(options: { scope?: () => string; limit?: number; journal?: CheckpointJournal } = {}) {
  const pending = new Map<string, PendingCheckpoint>();
  const scope = options.scope ?? (() => "");

  return async function checkpoint<Input extends { request_id?: string }, Result>(
    operation: "model" | "version", parentId: string, input: Input,
    write: (snapshot: Input) => Promise<Result>,
  ): Promise<Result> {
    if (!globalThis.crypto?.subtle) throw new Error("checkpoint:secure_context_required");
    const requestScope = scope();
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
    if (scope() !== requestScope) throw new Error("checkpoint:context_changed");
    let request = pending.get(key);
    const isNew = !request;
    if (!request) {
      // Never silently evict an uncertain write and generate a new key for its retry.
      if (pending.size >= (options.limit ?? 64)) throw new Error("checkpoint:recovery_capacity_reached");
      request = { requestId: explicitRequestId ?? crypto.randomUUID() };
      pending.set(key, request);
    }
    if (request.inFlight) return request.inFlight as Promise<Result>;
    const attempt = request;
    const remove = async () => {
      pending.delete(key);
      // Failure to clear a confirmed receipt must not turn a commit into a
      // reported save failure. Leave it visible for read-only reconciliation.
      await options.journal?.remove(key, attempt.requestId).catch(() => undefined);
    };
    attempt.inFlight = Promise.resolve().then(async () => {
      if (options.journal) {
        const durable = await options.journal.reserve({
          key, scope: await checkpointDigest(requestScope), operation, parentId,
          requestId: attempt.requestId, createdAt: Date.now(),
        });
        attempt.requestId = durable.intent.requestId;
      }
      if (scope() !== requestScope) {
        // A durable identity may already be in use by another window. Never
        // erase it merely because this window cancelled before dispatch.
        if (isNew && !options.journal) await remove();
        throw new Error("checkpoint:context_changed");
      }
      return write({ ...snapshot, request_id: attempt.requestId });
    })
      .then(async (response) => {
        await remove();
        return response;
      }, async (error: unknown) => {
        attempt.inFlight = undefined;
        const status = (error as { statusCode?: number } | null)?.statusCode;
        // With persistence, memory only coalesces in-flight calls. A later
        // rejection cannot prove that an earlier attempt never committed.
        if (options.journal) pending.delete(key);
        else if (explicitRequestId || [400, 401, 403, 404, 405, 413, 415, 422].includes(status ?? 0)) await remove();
        throw error;
      });
    return attempt.inFlight as Promise<Result>;
  };
}
