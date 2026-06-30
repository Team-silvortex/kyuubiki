export type DirectMeshCachedResult = {
  studyKind:
    | "truss_2d"
    | "truss_3d"
    | "plane_triangle_2d"
    | "thermal_plane_triangle_2d"
    | "plane_quad_2d"
    | "thermal_plane_quad_2d"
    | "axial_bar_1d"
    | "thermal_bar_1d"
    | "heat_bar_1d"
    | "electrostatic_plane_triangle_2d"
    | "electrostatic_plane_quad_2d"
    | "heat_plane_triangle_2d"
    | "heat_plane_quad_2d"
    | "thermal_truss_2d"
    | "thermal_truss_3d"
    | "spring_1d"
    | "spring_2d"
    | "spring_3d"
    | "beam_1d"
    | "thermal_beam_1d"
    | "thermal_frame_2d"
    | "torsion_1d"
    | "frame_2d";
  result: Record<string, unknown>;
  endpoint: string;
  storedAt: string;
};

type ChunkKind = "nodes" | "elements";
type DirectMeshCachedEntry = DirectMeshCachedResult & { storedAtUnixMs: number };

const DIRECT_MESH_RESULT_TTL_MS = 10 * 60 * 1000;
const DIRECT_MESH_RESULT_CACHE_MAX_ENTRIES = 24;
const DIRECT_MESH_RESULT_CHUNK_MAX_LIMIT = 1_000;

function resultCache() {
  const globalWithCache = globalThis as typeof globalThis & {
    __kyuubikiDirectMeshResults?: Map<string, DirectMeshCachedEntry>;
  };

  if (!globalWithCache.__kyuubikiDirectMeshResults) {
    globalWithCache.__kyuubikiDirectMeshResults = new Map<string, DirectMeshCachedEntry>();
  }

  return globalWithCache.__kyuubikiDirectMeshResults as Map<string, DirectMeshCachedEntry>;
}

function pruneExpiredResults(now = Date.now()) {
  const cache = resultCache();
  for (const [jobId, entry] of cache.entries()) {
    if (now - entry.storedAtUnixMs > DIRECT_MESH_RESULT_TTL_MS) {
      cache.delete(jobId);
    }
  }
}

export function putDirectMeshResult(jobId: string, entry: DirectMeshCachedResult) {
  const cache = resultCache();
  pruneExpiredResults();
  cache.set(jobId, { ...entry, storedAtUnixMs: Date.now() });

  while (cache.size > DIRECT_MESH_RESULT_CACHE_MAX_ENTRIES) {
    const oldestKey = cache.keys().next().value;
    if (typeof oldestKey !== "string") break;
    cache.delete(oldestKey);
  }
}

export function getDirectMeshResult(jobId: string) {
  pruneExpiredResults();
  const entry = resultCache().get(jobId);
  if (!entry) return null;
  return entry;
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

  const rawCollection = (entry.result as Record<string, unknown>)[kind];
  const collection = Array.isArray(rawCollection) ? rawCollection : [];

  const safeOffset = Math.min(Math.max(0, options.offset ?? 0), collection.length);
  const safeLimit = Math.min(
    DIRECT_MESH_RESULT_CHUNK_MAX_LIMIT,
    Math.max(1, options.limit ?? 200),
  );
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
