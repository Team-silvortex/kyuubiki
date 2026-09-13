type PendingCheckpoint = { requestId: string; inFlight?: Promise<unknown> };

export function createCheckpointRetry(options: { scope?: () => string; limit?: number } = {}) {
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
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(
      JSON.stringify([requestScope, operation, parentId, serialized]),
    ));
    if (scope() !== requestScope) throw new Error("checkpoint:context_changed");
    const key = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
    let request = pending.get(key);
    if (!request) {
      // Never silently evict an uncertain write and generate a new key for its retry.
      if (pending.size >= (options.limit ?? 64)) throw new Error("checkpoint:recovery_capacity_reached");
      request = { requestId: input.request_id ?? crypto.randomUUID() };
      pending.set(key, request);
    }
    if (request.inFlight) return request.inFlight as Promise<Result>;
    const attempt = request;
    attempt.inFlight = Promise.resolve().then(() => write({ ...JSON.parse(serialized), request_id: attempt.requestId }))
      .then((response) => {
        pending.delete(key);
        return response;
      }, (error: unknown) => {
        attempt.inFlight = undefined;
        const status = (error as { statusCode?: number } | null)?.statusCode;
        if (input.request_id || [400, 401, 403, 404, 405, 413, 415, 422].includes(status ?? 0)) pending.delete(key);
        throw error;
      });
    return attempt.inFlight as Promise<Result>;
  };
}
