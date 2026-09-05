import { resolveWorkbenchUiStreamingState } from "@/components/workbench/workbench-ui-streaming";
import type { WorkbenchUiChunkId } from "@/components/workbench/workbench-ui-streaming";
import type { SidebarSection } from "@/components/workbench/workbench-types";

export const loadWorkbenchLibrarySectionMount = () =>
  import("@/components/workbench/workbench-library-section-mount");
export const loadWorkbenchModelSectionMount = () =>
  import("@/components/workbench/workbench-model-section-mount");
export const loadWorkbenchStudySectionMount = () =>
  import("@/components/workbench/workbench-study-section-mount");
export const loadWorkbenchStoreSectionMount = () =>
  import("@/components/workbench/workbench-store-section-mount");
export const loadWorkbenchSystemSidebarMount = () =>
  import("@/components/workbench/workbench-system-sidebar-mount");
export const loadWorkbenchWorkflowSectionMount = () =>
  import("@/components/workbench/workbench-workflow-section-mount");

type ChunkLoader = () => Promise<unknown>;

const CHUNK_LOADERS: Partial<Record<WorkbenchUiChunkId, ChunkLoader>> = {
  "section.library": loadWorkbenchLibrarySectionMount,
  "section.model": loadWorkbenchModelSectionMount,
  "section.study": loadWorkbenchStudySectionMount,
  "section.store": loadWorkbenchStoreSectionMount,
  "section.system": loadWorkbenchSystemSidebarMount,
  "section.workflow": loadWorkbenchWorkflowSectionMount,
};

const prefetchedChunks = new Set<WorkbenchUiChunkId>();
const inflightChunks = new Map<WorkbenchUiChunkId, Promise<unknown>>();

export function prefetchWorkbenchSidebarChunk(chunkId: WorkbenchUiChunkId) {
  if (prefetchedChunks.has(chunkId)) return Promise.resolve();
  const inflight = inflightChunks.get(chunkId);
  if (inflight) return inflight;
  const loader = CHUNK_LOADERS[chunkId];
  if (!loader) return Promise.resolve();

  const request = loader()
    .then(() => {
      prefetchedChunks.add(chunkId);
    })
    .finally(() => {
      inflightChunks.delete(chunkId);
    });
  inflightChunks.set(chunkId, request);
  return request;
}

export function scheduleWorkbenchSidebarChunkPrefetch(activeSection: SidebarSection) {
  if (typeof window === "undefined") return () => {};
  const queue = resolveWorkbenchUiStreamingState(activeSection).prefetchChunks.filter(
    (chunkId) => CHUNK_LOADERS[chunkId] && !prefetchedChunks.has(chunkId),
  );
  let cancelled = false;
  let frameHandle: number | null = null;
  let idleHandle: number | null = null;
  let timeoutHandle: number | null = null;

  const scheduleNext = () => {
    if (cancelled || queue.length === 0) return;
    if (typeof window.requestIdleCallback === "function") {
      idleHandle = window.requestIdleCallback(() => void loadNext(), { timeout: 900 });
      return;
    }
    timeoutHandle = window.setTimeout(() => void loadNext(), 240);
  };
  const loadNext = async () => {
    if (cancelled) return;
    const chunkId = queue.shift();
    if (!chunkId) return;
    try {
      await prefetchWorkbenchSidebarChunk(chunkId);
    } catch {
      // Active navigation can retry a failed speculative import through React.lazy.
    } finally {
      scheduleNext();
    }
  };

  // Warm one adjacent workspace after the active shell has painted. Remaining
  // candidates stay serialized behind idle callbacks to preserve the load budget.
  frameHandle = window.requestAnimationFrame(() => void loadNext());
  return () => {
    cancelled = true;
    if (frameHandle !== null) window.cancelAnimationFrame(frameHandle);
    if (idleHandle !== null) window.cancelIdleCallback(idleHandle);
    if (timeoutHandle !== null) window.clearTimeout(timeoutHandle);
  };
}
