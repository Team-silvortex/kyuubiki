export type DirectMeshCachedResult = {
  studyKind: "truss_2d" | "truss_3d" | "plane_triangle_2d" | "axial_bar_1d";
  result: Record<string, unknown>;
  endpoint: string;
  storedAt: string;
};

type ChunkKind = "nodes" | "elements";

function resultCache() {
  const globalWithCache = globalThis as typeof globalThis & {
    __kyuubikiDirectMeshResults?: Map<string, DirectMeshCachedResult>;
  };

  if (!globalWithCache.__kyuubikiDirectMeshResults) {
    globalWithCache.__kyuubikiDirectMeshResults = new Map<string, DirectMeshCachedResult>();
  }

  return globalWithCache.__kyuubikiDirectMeshResults;
}

export function putDirectMeshResult(jobId: string, entry: DirectMeshCachedResult) {
  resultCache().set(jobId, entry);
}

export function getDirectMeshResult(jobId: string) {
  return resultCache().get(jobId) ?? null;
}

export function chunkDirectMeshResult(
  jobId: string,
  kind: ChunkKind,
  options: { offset?: number; limit?: number } = {},
) {
  const entry = getDirectMeshResult(jobId);
  if (!entry) {
    throw new Error(`no cached direct mesh result for ${jobId}`);
  }

  const collection = Array.isArray((entry.result as { [key: string]: unknown[] })[kind])
    ? (((entry.result as { [key: string]: unknown[] })[kind] as unknown[]) ?? [])
    : [];

  const safeOffset = Math.min(Math.max(0, options.offset ?? 0), collection.length);
  const safeLimit = Math.max(1, options.limit ?? 200);
  const items = collection.slice(safeOffset, safeOffset + safeLimit);

  return {
    job_id: jobId,
    kind,
    offset: safeOffset,
    limit: safeLimit,
    returned: items.length,
    total: collection.length,
    items,
    endpoint: entry.endpoint,
    stored_at: entry.storedAt,
  };
}
