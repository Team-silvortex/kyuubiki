const PERF_KEY = "__kyuubikiHubPerf";
function now() {
    return window.performance?.now?.() ?? Date.now();
}
function store() {
    const typedWindow = window;
    if (typedWindow[PERF_KEY]) {
        return typedWindow[PERF_KEY];
    }
    typedWindow[PERF_KEY] = {
        counters: {},
        marks: {},
        measures: [],
    };
    return typedWindow[PERF_KEY];
}
export function markHubUiPerf(label) {
    if (!label) {
        return 0;
    }
    const timestamp = now();
    store().marks[label] = timestamp;
    return timestamp;
}
export function measureHubUiPerf(label, fromLabel) {
    const perf = store();
    const startedAt = perf.marks[fromLabel];
    if (typeof startedAt !== "number") {
        return null;
    }
    const durationMs = Math.max(0, now() - startedAt);
    perf.measures.push({
        label,
        from: fromLabel,
        durationMs: Math.round(durationMs * 10) / 10,
        recordedAt: Date.now(),
    });
    if (perf.measures.length > 80) {
        perf.measures.splice(0, perf.measures.length - 80);
    }
    return durationMs;
}
export function countHubUiPerf(label, amount = 1) {
    if (!label) {
        return 0;
    }
    const counters = store().counters;
    counters[label] = (Number(counters[label]) || 0) + amount;
    return counters[label];
}
export function snapshotHubUiPerf() {
    const perf = store();
    return {
        counters: { ...perf.counters },
        marks: { ...perf.marks },
        measures: [...perf.measures],
    };
}
