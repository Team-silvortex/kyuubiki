function scheduleIdle(callback, timeout = 1200) {
    if (window.requestIdleCallback) {
        window.requestIdleCallback(callback, { timeout });
        return;
    }
    window.setTimeout(callback, Math.min(timeout, 1200));
}
function normalizeNodes(nodes) {
    const list = nodes instanceof NodeList ? Array.from(nodes) : Array.isArray(nodes) ? nodes : [nodes];
    return list.filter((node) => node instanceof HTMLElement);
}
function clearReleaseTimer(chunk) {
    if (chunk.releaseTimer !== null) {
        window.clearTimeout(chunk.releaseTimer);
        chunk.releaseTimer = null;
    }
}
export function createHubStreamingRuntime({ setEventMessage } = {}) {
    const chunks = new Map();
    const activeChunks = new Set();
    function registerChunk(id, options = {}) {
        if (!id) {
            return null;
        }
        const retainMs = typeof options.retainMs === "number" && Number.isFinite(options.retainMs)
            ? options.retainMs
            : 45000;
        const chunk = {
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
    function activateChunk(id) {
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
    function scheduleRelease(id) {
        const chunk = chunks.get(id);
        if (!chunk || activeChunks.has(id)) {
            return;
        }
        clearReleaseTimer(chunk);
        chunk.releaseTimer = window.setTimeout(() => {
            releaseChunk(id);
        }, chunk.retainMs);
    }
    function deactivateChunk(id) {
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
    function releaseChunk(id) {
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
    function activateOnly(ids, options = {}) {
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
    function snapshot() {
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
