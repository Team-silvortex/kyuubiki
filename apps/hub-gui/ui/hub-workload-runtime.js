function asEntryList(entries) {
    return entries.filter((entry) => Boolean(entry));
}
function asRecord(value) {
    return Boolean(value) && typeof value === "object" && !Array.isArray(value) ? value : {};
}
function markHubWorkloadDownloaded(context, entry) {
    const next = context.loadHubWorkloadLibrary().map((candidate) => {
        if (context.workloadIdentity(candidate) !== context.workloadIdentity(entry)) {
            return candidate;
        }
        return {
            ...candidate,
            downloadedAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
        };
    });
    context.saveHubWorkloadLibrary(next);
}
export function updateHubWorkloadEntry(context, entry, updater) {
    const next = context.loadHubWorkloadLibrary()
        .map((candidate) => {
        if (context.workloadIdentity(candidate) !== context.workloadIdentity(entry)) {
            return candidate;
        }
        return context.normalizeHubWorkloadEntry(updater({
            ...candidate,
        }));
    })
        .filter((candidate) => Boolean(candidate));
    context.saveHubWorkloadLibrary(next);
}
export async function downloadRemoteWorkloadBundle(context, entry) {
    const validation = context.validateHubCatalogUrl(entry.downloadUrl || "");
    if (!validation.ok) {
        throw new Error(validation.reason);
    }
    const normalizedUrl = validation.normalized || "";
    if (!context.ensureRemoteHostTrust(normalizedUrl, "This workload download")) {
        throw new Error("workload download cancelled before contacting the remote host");
    }
    const response = await fetch(normalizedUrl);
    if (!response.ok) {
        throw new Error(`bundle download failed (${response.status})`);
    }
    const blob = await response.blob();
    const filename = context.inferDownloadFilename(normalizedUrl);
    context.downloadHubBlob(filename, blob);
    markHubWorkloadDownloaded(context, entry);
    context.setWorkloadLibraryOutput(`downloaded ${entry.label} as ${filename}`);
}
export async function openWorkloadInWorkbench(context, entry) {
    if (!entry.bundlePath) {
        throw new Error("This workload does not have a local bundle path yet.");
    }
    const bundleInput = context.elements.projectBundlePath;
    if (bundleInput) {
        bundleInput.value = entry.bundlePath;
    }
    context.renderAssistantContext();
    context.renderHubAssistantLocalCards();
    context.setWorkloadLibraryOutput(`loaded ${entry.label} into the bundle path and opening Workbench`);
    await context.runAction("open-workbench");
}
export async function attachCurrentBundleToWorkload(context, entry) {
    const bundlePath = String(context.elements.projectBundlePath?.value || "").trim();
    if (!bundlePath) {
        throw new Error("Fill in the current bundle path before attaching it to this workload.");
    }
    const inspectRaw = await context.invokeTauri("project_bundle_inspect", { payload: { path: bundlePath } });
    const summary = context.projectSummaryFromInspectPayload(inspectRaw);
    updateHubWorkloadEntry(context, entry, (candidate) => ({
        ...candidate,
        bundlePath,
        projectId: summary.projectId || candidate.projectId,
        projectName: summary.projectName || candidate.projectName,
        schema: summary.schema || candidate.schema,
        layout: summary.layout || candidate.layout,
        modelCount: summary.modelCount,
        versionCount: summary.versionCount,
        jobCount: summary.jobCount,
        resultCount: summary.resultCount,
        analysisDomains: summary.analysisDomains,
        analysisFamilies: summary.analysisFamilies,
        thermalIntents: summary.thermalIntents,
        attachedAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
    }));
    context.setWorkloadLibraryOutput(`attached local bundle ${bundlePath} to ${entry.label}`);
}
export function saveHubWorkloadLibrary(context, entries) {
    context.persistHubWorkloadLibrary(entries);
    context.renderHubWorkloadLibrary(entries);
}
export function matchesWorkloadFamilyFilter(context, entry) {
    if (context.state.workloadFamilyFilter === "all") {
        return true;
    }
    return entry.analysisFamilies.includes(context.state.workloadFamilyFilter);
}
export function matchesWorkloadFilter(context, entry) {
    if (context.state.workloadFilter === "all") {
        return matchesWorkloadFamilyFilter(context, entry);
    }
    return entry.analysisDomains.includes(context.state.workloadFilter) && matchesWorkloadFamilyFilter(context, entry);
}
export function currentWorkloadLibrarySearchQuery(context) {
    return String(context.elements.workloadLibrarySearch?.value || "").trim().toLowerCase();
}
export function matchesWorkloadSearchQuery(context, entry) {
    const query = currentWorkloadLibrarySearchQuery(context);
    if (!query) {
        return true;
    }
    const haystack = [
        entry.label,
        entry.note,
        entry.bundlePath,
        entry.downloadUrl,
        entry.projectId,
        entry.projectName,
        entry.schema,
        entry.layout,
        ...(Array.isArray(entry.analysisDomains) ? entry.analysisDomains : []),
        ...(Array.isArray(entry.analysisFamilies) ? entry.analysisFamilies : []),
        ...(Array.isArray(entry.thermalIntents) ? entry.thermalIntents : []),
    ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
    return query
        .split(/\s+/u)
        .filter(Boolean)
        .every((token) => haystack.includes(token));
}
export async function registerCurrentBundleAsWorkload(context) {
    const bundlePath = String(context.elements.projectBundlePath?.value || "").trim();
    if (!bundlePath) {
        throw new Error("Fill in a bundle path before registering a workload.");
    }
    const inspectRaw = await context.invokeTauri("project_bundle_inspect", { payload: { path: bundlePath } });
    const summary = context.projectSummaryFromInspectPayload(inspectRaw);
    const note = String(context.elements.workloadLabel?.value || "").trim();
    const entry = context.normalizeHubWorkloadEntry({
        label: note || summary.projectName || summary.projectId || bundlePath,
        note: note || `Registered from local bundle ${bundlePath}`,
        sourceKind: "local-bundle",
        sourceLabel: "Hub local registration",
        bundlePath,
        projectId: summary.projectId,
        projectName: summary.projectName,
        schema: summary.schema,
        layout: summary.layout,
        modelCount: summary.modelCount,
        versionCount: summary.versionCount,
        jobCount: summary.jobCount,
        resultCount: summary.resultCount,
        analysisDomains: summary.analysisDomains,
        analysisFamilies: summary.analysisFamilies,
        thermalIntents: summary.thermalIntents,
    });
    if (!entry) {
        throw new Error("Current bundle could not be normalized into a workload entry.");
    }
    const next = context.mergeHubWorkloadLibrary(context.loadHubWorkloadLibrary(), [entry]);
    context.saveHubWorkloadLibrary(next);
    context.setWorkloadLibraryOutput(`registered ${entry.label} in the workload library`);
}
export async function syncRemoteWorkloadCatalog(context, urlOverride = "") {
    const selectedUrl = String(urlOverride || "").trim() || String(context.elements.workloadCatalogUrl?.value || "").trim();
    const validation = context.validateHubCatalogUrl(selectedUrl);
    if (!validation.ok) {
        throw new Error(validation.reason);
    }
    const normalizedUrl = validation.normalized || "";
    if (context.elements.workloadCatalogUrl) {
        context.elements.workloadCatalogUrl.value = normalizedUrl;
    }
    if (!context.ensureRemoteHostTrust(normalizedUrl, "This remote catalog sync")) {
        throw new Error("remote catalog sync cancelled before contacting the remote host");
    }
    const response = await fetch(normalizedUrl);
    if (!response.ok) {
        throw new Error(`catalog sync failed (${response.status})`);
    }
    const payload = await response.json();
    const payloadValidation = context.validateRemoteWorkloadCatalogPayload(payload);
    if (!payloadValidation.ok) {
        throw new Error(payloadValidation.reason);
    }
    const normalized = context.normalizeRemoteWorkloadCatalogPayload(payload, normalizedUrl);
    const next = context.mergeHubWorkloadLibrary(context.loadHubWorkloadLibrary(), normalized);
    context.saveHubWorkloadLibrary(next);
    context.setWorkloadLibraryOutput(`synced ${normalized.length} workload entries from remote catalog`);
}
export async function syncLocalControlPlaneWorkloads(context) {
    const catalogUrl = context.ensureDefaultWorkloadCatalogUrl(true);
    await syncRemoteWorkloadCatalog(context, catalogUrl);
}
export function exportHubWorkloadLibrary(context) {
    const payload = {
        exportedAt: new Date().toISOString(),
        workloadCount: context.loadHubWorkloadLibrary().length,
        workloads: context.loadHubWorkloadLibrary(),
    };
    context.downloadHubJson("kyuubiki-hub-workloads.json", payload);
    context.setWorkloadLibraryOutput(`exported ${payload.workloadCount} workload entries as JSON`);
}
export async function importHubWorkloadLibrary(context, file) {
    if (!file) {
        return;
    }
    const raw = await file.text();
    const parsed = asRecord(JSON.parse(raw));
    const imported = Array.isArray(parsed.workloads) ? parsed.workloads : [];
    const normalized = imported
        .map((entry) => {
        const candidate = asRecord(entry);
        return context.normalizeHubWorkloadEntry({
            ...candidate,
            sourceKind: candidate.sourceKind || "imported-library",
        });
    })
        .filter((entry) => Boolean(entry));
    const next = context.mergeHubWorkloadLibrary(context.loadHubWorkloadLibrary(), normalized);
    context.saveHubWorkloadLibrary(next);
    context.setWorkloadLibraryOutput(`imported ${normalized.length} workload entries into the Hub library`);
}
export function clearHubWorkloadLibrary(context) {
    context.saveHubWorkloadLibrary([]);
    context.setWorkloadLibraryOutput("cleared the Hub workload library");
}
