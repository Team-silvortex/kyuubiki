import type { ResultChunkPayload } from "@/lib/api";

export const RESULT_WINDOW_THRESHOLD = 400;
export const RESULT_WINDOW_BASE_SIZE = 240;
export const RESULT_WINDOW_CACHE_LIMIT = 24;

export function computeResultWindowSize(totalItems: number, viewportWidth = 980) {
  const safeTotalItems = Number.isFinite(totalItems) ? Math.max(0, Math.floor(totalItems)) : 0;
  const safeViewportWidth = Number.isFinite(viewportWidth) && viewportWidth > 0 ? viewportWidth : 980;
  const base =
    safeTotalItems >= 20_000
      ? 720
      : safeTotalItems >= 15_000
        ? 600
        : safeTotalItems >= 10_000
          ? 480
          : safeTotalItems >= 4_000
            ? 360
            : RESULT_WINDOW_BASE_SIZE;

  const widthFactor = Math.min(1.8, Math.max(0.85, safeViewportWidth / 980));
  const scaled = Math.round((base * widthFactor) / 60) * 60;
  return Math.max(RESULT_WINDOW_BASE_SIZE, scaled);
}

export function clampChunkOffset(offset: number, totalItems: number, limit: number) {
  if (!Number.isFinite(limit) || limit <= 0) return 0;
  const safeLimit = Math.max(1, Math.floor(limit));
  const safeTotalItems = Number.isFinite(totalItems) ? Math.max(0, Math.floor(totalItems)) : 0;
  const safeOffset = Number.isFinite(offset) ? Math.max(0, offset) : 0;
  const maxOffset = Math.max(0, safeTotalItems - safeLimit);
  const snapped = Math.round(safeOffset / safeLimit) * safeLimit;
  return Math.min(maxOffset, snapped);
}

export function computeVisibleResultWindowOffset(
  totalItems: number,
  limit: number,
  viewportWidth: number,
  scrollLeft: number,
  scrollWidth: number,
) {
  if (!Number.isFinite(limit) || limit <= 0) return 0;
  if (totalItems <= limit || scrollWidth <= viewportWidth + 1) {
    return 0;
  }

  const maxTravel = Math.max(1, scrollWidth - viewportWidth);
  const visibleStartRatio = Math.min(1, Math.max(0, scrollLeft / maxTravel));
  const visibleSpanRatio = Math.min(1, Math.max(0.08, viewportWidth / Math.max(scrollWidth, viewportWidth)));
  const visibleSpan = Math.max(1, Math.round(totalItems * visibleSpanRatio));
  const maxVisibleStart = Math.max(0, totalItems - visibleSpan);
  const visibleStart = visibleStartRatio * maxVisibleStart;
  const overscan = Math.max(60, Math.round(limit * 0.25));
  return clampChunkOffset(visibleStart - overscan, totalItems, limit);
}

export function chunkCacheKey(
  backendId: string,
  jobId: string,
  kind: "nodes" | "elements",
  offset: number,
  limit: number,
) {
  return JSON.stringify([backendId, jobId, kind, offset, limit]);
}

export function readChunkCache(
  cache: Map<string, ResultChunkPayload<Record<string, unknown>>>,
  key: string,
) {
  const cached = cache.get(key);
  if (!cached) return null;
  cache.delete(key);
  cache.set(key, cached);
  return cached;
}

export function writeChunkCache(
  cache: Map<string, ResultChunkPayload<Record<string, unknown>>>,
  key: string,
  value: ResultChunkPayload<Record<string, unknown>>,
) {
  if (cache.has(key)) {
    cache.delete(key);
  }

  cache.set(key, value);

  while (cache.size > RESULT_WINDOW_CACHE_LIMIT) {
    const oldest = cache.keys().next();
    if (oldest.done) break;
    cache.delete(oldest.value);
  }
}
