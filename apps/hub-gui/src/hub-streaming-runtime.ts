type HubStreamingChunkOptions = {
  nodes?: Element | Element[] | NodeListOf<Element> | null;
  retainMs?: number;
  onHydrate?: () => void;
  onRelease?: () => void;
  group?: string;
};

type HubStreamingChunk = {
  id: string;
  nodes: HTMLElement[];
  retainMs: number;
  hydrated: boolean;
  released: boolean;
  releaseTimer: number | null;
  onHydrate?: () => void;
  onRelease?: () => void;
  group: string;
};

type HubStreamingRuntimeOptions = {
  setEventMessage?: (message: string, kind?: string) => void;
};

type HubStreamingActivateOptions = {
  group?: string;
};

type HubStreamingSnapshotEntry = {
  id: string;
  active: boolean;
  hydrated: boolean;
  released: boolean;
  nodeCount: number;
};

export type HubStreamingRuntime = {
  activateChunk: (id: string) => void;
  activateOnly: (ids: string | string[], options?: HubStreamingActivateOptions) => void;
  deactivateChunk: (id: string) => void;
  registerChunk: (id: string, options?: HubStreamingChunkOptions) => HubStreamingChunk | null;
  snapshot: () => HubStreamingSnapshotEntry[];
};

function scheduleIdle(callback: () => void, timeout = 1200): void {
  if (window.requestIdleCallback) {
    window.requestIdleCallback(callback, { timeout });
    return;
  }
  window.setTimeout(callback, Math.min(timeout, 1200));
}

function normalizeNodes(nodes: HubStreamingChunkOptions["nodes"]): HTMLElement[] {
  const list = nodes instanceof NodeList ? Array.from(nodes) : Array.isArray(nodes) ? nodes : [nodes];
  return list.filter((node): node is HTMLElement => node instanceof HTMLElement);
}

function clearReleaseTimer(chunk: HubStreamingChunk): void {
  if (chunk.releaseTimer !== null) {
    window.clearTimeout(chunk.releaseTimer);
    chunk.releaseTimer = null;
  }
}

export function createHubStreamingRuntime({ setEventMessage }: HubStreamingRuntimeOptions = {}): HubStreamingRuntime {
  const chunks = new Map<string, HubStreamingChunk>();
  const activeChunks = new Set<string>();

  function registerChunk(id: string, options: HubStreamingChunkOptions = {}): HubStreamingChunk | null {
    if (!id) {
      return null;
    }

    const retainMs = typeof options.retainMs === "number" && Number.isFinite(options.retainMs)
      ? options.retainMs
      : 45000;
    const chunk: HubStreamingChunk = {
      id,
      nodes: normalizeNodes(options.nodes),
      retainMs,
      hydrated: false,
      released: false,
      releaseTimer: null,
      onHydrate: options.onHydrate,
      onRelease: options.onRelease,
      group: options.group || "main",
    };

    chunk.nodes.forEach((node) => {
      node.dataset.streamChunk = id;
      node.dataset.streamState = "registered";
    });
    chunks.set(id, chunk);
    return chunk;
  }

  function activateChunk(id: string): void {
    const chunk = chunks.get(id);
    if (!chunk) {
      return;
    }

    clearReleaseTimer(chunk);
    activeChunks.add(id);
    chunk.nodes.forEach((node) => {
      node.dataset.streamState = "active";
      node.removeAttribute("inert");
    });

    if (!chunk.hydrated || chunk.released) {
      chunk.hydrated = true;
      chunk.released = false;
      chunk.onHydrate?.();
      setEventMessage?.(`stream chunk active: ${id}`, "stream:hydrate");
    }

    for (const otherId of chunks.keys()) {
      if (otherId !== id) {
        scheduleRelease(otherId);
      }
    }
  }

  function scheduleRelease(id: string): void {
    const chunk = chunks.get(id);
    if (!chunk || activeChunks.has(id)) {
      return;
    }

    clearReleaseTimer(chunk);
    chunk.releaseTimer = window.setTimeout(() => {
      releaseChunk(id);
    }, chunk.retainMs);
  }

  function deactivateChunk(id: string): void {
    if (!activeChunks.delete(id)) {
      return;
    }
    const chunk = chunks.get(id);
    chunk?.nodes.forEach((node) => {
      if (node.dataset.streamState === "active") {
        node.dataset.streamState = "warm";
      }
    });
    scheduleRelease(id);
  }

  function releaseChunk(id: string): void {
    const chunk = chunks.get(id);
    if (!chunk || activeChunks.has(id) || chunk.released) {
      return;
    }

    scheduleIdle(() => {
      if (activeChunks.has(id) || chunk.released) {
        return;
      }

      chunk.released = true;
      chunk.nodes.forEach((node) => {
        node.dataset.streamState = "dormant";
        node.setAttribute("inert", "");
      });
      chunk.onRelease?.();
      setEventMessage?.(`stream chunk dormant: ${id}`, "stream:release");
    });
  }

  function activateOnly(ids: string | string[], options: HubStreamingActivateOptions = {}): void {
    const next = new Set((Array.isArray(ids) ? ids : [ids]).filter(Boolean));
    for (const id of activeChunks) {
      const chunk = chunks.get(id);
      const sameGroup = !options.group || chunk?.group === options.group;
      if (sameGroup && !next.has(id)) {
        deactivateChunk(id);
      }
    }
    for (const id of next) {
      activateChunk(id);
    }
  }

  function snapshot(): HubStreamingSnapshotEntry[] {
    return Array.from(chunks.values()).map((chunk) => ({
      id: chunk.id,
      active: activeChunks.has(chunk.id),
      hydrated: chunk.hydrated,
      released: chunk.released,
      nodeCount: chunk.nodes.length,
    }));
  }

  return {
    activateChunk,
    activateOnly,
    deactivateChunk,
    registerChunk,
    snapshot,
  };
}
