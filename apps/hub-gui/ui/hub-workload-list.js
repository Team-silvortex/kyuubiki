import { countHubUiPerf, markHubUiPerf, measureHubUiPerf } from "./hub-ui-performance.js";
export function renderWorkloadFilters(params) {
    const { elements, state } = params;
    elements.workloadFilterButtons.forEach((button) => {
        const matches = button.dataset.workloadFilter === state.workloadFilter;
        button.classList.toggle("desktop-shell-button-primary", matches);
        button.classList.toggle("desktop-shell-button-ghost", !matches);
    });
    elements.workloadFamilyFilterButtons.forEach((button) => {
        const matches = button.dataset.workloadFamilyFilter === state.workloadFamilyFilter;
        button.classList.toggle("desktop-shell-button-primary", matches);
        button.classList.toggle("desktop-shell-button-ghost", !matches);
    });
}
function searchTokens(query) {
    return String(query || "")
        .trim()
        .toLowerCase()
        .split(/\s+/u)
        .filter(Boolean);
}
function appendHighlightedText(parent, tagName, text, tokens, className = "") {
    const element = document.createElement(tagName);
    if (className) {
        element.className = className;
    }
    const source = String(text || "");
    if (!source) {
        element.textContent = "";
        parent.appendChild(element);
        return element;
    }
    const normalized = source.toLowerCase();
    const ranges = [];
    tokens.forEach((token) => {
        let from = 0;
        while (from < normalized.length) {
            const index = normalized.indexOf(token, from);
            if (index === -1)
                break;
            ranges.push([index, index + token.length]);
            from = index + token.length;
        }
    });
    if (!ranges.length) {
        element.textContent = source;
        parent.appendChild(element);
        return element;
    }
    ranges.sort((left, right) => left[0] - right[0] || left[1] - right[1]);
    const merged = [];
    ranges.forEach((range) => {
        const last = merged[merged.length - 1];
        if (!last || range[0] > last[1]) {
            merged.push([...range]);
            return;
        }
        last[1] = Math.max(last[1], range[1]);
    });
    let cursor = 0;
    merged.forEach(([start, end]) => {
        if (cursor < start) {
            element.appendChild(document.createTextNode(source.slice(cursor, start)));
        }
        const mark = document.createElement("mark");
        mark.className = "hub-search-mark";
        mark.textContent = source.slice(start, end);
        element.appendChild(mark);
        cursor = end;
    });
    if (cursor < source.length) {
        element.appendChild(document.createTextNode(source.slice(cursor)));
    }
    parent.appendChild(element);
    return element;
}
export function renderHubWorkloadLibraryList(params) {
    const { elements, entries, state, renderWorkloadFilters, renderEmptyHistoryState, emptyMessage, filterEmptyMessage, matchesWorkloadFilter, matchesWorkloadSearchQuery, workloadSourceBadge, formatProjectActionTime, appendTextElement, workloadDomainLabel, workloadFamilyLabel, workloadProvenanceLabel, setWorkloadLibraryOutput, hubMessage, restoredWorkloadContextMessage, loadedWorkloadContextMessage, removedWorkloadMessage, renderAssistantContext, renderHubAssistantLocalCards, openWorkloadInWorkbench, formatHubOperatorError, runAction, downloadRemoteWorkloadBundle, attachCurrentBundleToWorkload, loadHubWorkloadLibrary, saveHubWorkloadLibrary, workloadIdentity, projectBundlePath, workloadCatalogUrl, hubCopy, hubI18nEn, currentSearchQuery, visibleLimit = 24, } = params;
    const workloadLibraryList = elements.workloadLibraryList;
    if (!workloadLibraryList) {
        return;
    }
    markHubUiPerf("workload-library-render-start");
    renderWorkloadFilters({ elements, state });
    workloadLibraryList.innerHTML = "";
    if (!entries.length) {
        renderEmptyHistoryState(workloadLibraryList, emptyMessage);
        return;
    }
    const filteredEntries = entries.filter((entry) => matchesWorkloadFilter(entry) && matchesWorkloadSearchQuery(entry));
    if (!filteredEntries.length) {
        renderEmptyHistoryState(workloadLibraryList, filterEmptyMessage);
        return;
    }
    const tokens = searchTokens(currentSearchQuery);
    const visibleEntries = filteredEntries.slice(0, visibleLimit);
    const fragment = document.createDocumentFragment();
    visibleEntries.forEach((entry) => {
        const shell = document.createElement("div");
        shell.className = "hub-history-item";
        const summary = document.createElement("button");
        summary.type = "button";
        summary.className = "hub-history-item__summary desktop-shell-button-ghost";
        const [sourceLabel, sourceClass] = workloadSourceBadge(entry);
        const metaBits = [
            entry.projectId ? `project ${entry.projectId}` : "",
            entry.schema || "",
            entry.layout || "",
            entry.attachedAt ? `attached ${formatProjectActionTime(entry.attachedAt)}` : "",
            entry.downloadedAt ? `downloaded ${formatProjectActionTime(entry.downloadedAt)}` : "",
        ].filter(Boolean);
        const heading = document.createElement("div");
        heading.className = "hub-history-item__heading";
        appendHighlightedText(heading, "strong", entry.label, tokens);
        const meta = document.createElement("div");
        meta.className = "hub-history-item__meta";
        appendHighlightedText(meta, "span", sourceLabel, tokens, sourceClass);
        entry.analysisDomains.forEach((domain) => {
            appendHighlightedText(meta, "span", workloadDomainLabel(domain), tokens, "desktop-shell-chip");
        });
        entry.analysisFamilies.forEach((family) => {
            appendHighlightedText(meta, "span", workloadFamilyLabel(family), tokens, "desktop-shell-chip");
        });
        heading.appendChild(meta);
        summary.appendChild(heading);
        appendHighlightedText(summary, "span", metaBits.join(" · ") || "workload entry", tokens, "hub-history-item__alias");
        appendHighlightedText(summary, "span", entry.note || entry.bundlePath || entry.downloadUrl || "--", tokens);
        appendHighlightedText(summary, "span", workloadProvenanceLabel(entry), tokens, "hub-history-item__provenance");
        if (entry.thermalIntents.length) {
            appendHighlightedText(summary, "span", `thermal: ${entry.thermalIntents.join(", ")}`, tokens, "desktop-shell-note");
        }
        summary.addEventListener("click", () => {
            if (entry.bundlePath) {
                if (projectBundlePath) {
                    projectBundlePath.value = entry.bundlePath;
                }
            }
            if (entry.downloadUrl && workloadCatalogUrl) {
                workloadCatalogUrl.value = entry.downloadUrl;
            }
            setWorkloadLibraryOutput(hubMessage(restoredWorkloadContextMessage, { label: entry.label }));
            renderAssistantContext();
            renderHubAssistantLocalCards();
        });
        const controls = document.createElement("div");
        controls.className = "hub-history-item__controls";
        const useButton = document.createElement("button");
        useButton.type = "button";
        useButton.className = "desktop-shell-button-ghost";
        useButton.textContent = hubCopy().dynamic?.workloadUse || hubI18nEn.dynamic?.workloadUse || "Use";
        useButton.addEventListener("click", () => {
            if (entry.bundlePath) {
                if (projectBundlePath) {
                    projectBundlePath.value = entry.bundlePath;
                }
            }
            setWorkloadLibraryOutput(hubMessage(loadedWorkloadContextMessage, { label: entry.label }));
            renderAssistantContext();
            renderHubAssistantLocalCards();
        });
        const workbenchButton = document.createElement("button");
        workbenchButton.type = "button";
        workbenchButton.className = "desktop-shell-button-ghost";
        workbenchButton.textContent = hubCopy().dynamic?.workloadOpenWorkbench || hubI18nEn.dynamic?.workloadOpenWorkbench || "Open in Workbench";
        workbenchButton.disabled = !entry.bundlePath;
        workbenchButton.addEventListener("click", () => {
            void openWorkloadInWorkbench(entry).catch((error) => {
                setWorkloadLibraryOutput(formatHubOperatorError(error, {
                    actionLabel: "Opening this workload in Workbench",
                }));
            });
        });
        const inspectButton = document.createElement("button");
        inspectButton.type = "button";
        inspectButton.className = "desktop-shell-button-ghost";
        inspectButton.textContent = hubCopy().dynamic?.workloadInspect || hubI18nEn.dynamic?.workloadInspect || "Inspect";
        inspectButton.disabled = !entry.bundlePath;
        inspectButton.addEventListener("click", () => {
            if (entry.bundlePath) {
                if (projectBundlePath) {
                    projectBundlePath.value = entry.bundlePath;
                }
                void runAction("project-inspect");
            }
        });
        const validateButton = document.createElement("button");
        validateButton.type = "button";
        validateButton.className = "desktop-shell-button-ghost";
        validateButton.textContent = hubCopy().dynamic?.workloadValidate || hubI18nEn.dynamic?.workloadValidate || "Validate";
        validateButton.disabled = !entry.bundlePath;
        validateButton.addEventListener("click", () => {
            if (entry.bundlePath) {
                if (projectBundlePath) {
                    projectBundlePath.value = entry.bundlePath;
                }
                void runAction("project-validate");
            }
        });
        const downloadButton = document.createElement("button");
        downloadButton.type = "button";
        downloadButton.className = "desktop-shell-button-ghost";
        downloadButton.textContent = hubCopy().dynamic?.workloadDownload || hubI18nEn.dynamic?.workloadDownload || "Download";
        downloadButton.disabled = !entry.downloadUrl;
        downloadButton.addEventListener("click", () => {
            void downloadRemoteWorkloadBundle(entry).catch((error) => {
                setWorkloadLibraryOutput(formatHubOperatorError(error, {
                    actionLabel: "Downloading this workload",
                }));
            });
        });
        const attachButton = document.createElement("button");
        attachButton.type = "button";
        attachButton.className = "desktop-shell-button-ghost";
        attachButton.textContent = entry.bundlePath
            ? hubCopy().dynamic?.workloadReattach || hubI18nEn.dynamic?.workloadReattach || "Reattach bundle"
            : hubCopy().dynamic?.workloadAttach || hubI18nEn.dynamic?.workloadAttach || "Attach current bundle";
        attachButton.addEventListener("click", () => {
            void attachCurrentBundleToWorkload(entry).catch((error) => {
                setWorkloadLibraryOutput(formatHubOperatorError(error, {
                    actionLabel: "Attaching the current bundle",
                }));
            });
        });
        const removeButton = document.createElement("button");
        removeButton.type = "button";
        removeButton.className = "desktop-shell-button-ghost";
        removeButton.textContent = hubCopy().dynamic?.workloadRemove || hubI18nEn.dynamic?.workloadRemove || "Remove";
        removeButton.addEventListener("click", () => {
            const next = loadHubWorkloadLibrary().filter((candidate) => workloadIdentity(candidate) !== workloadIdentity(entry));
            saveHubWorkloadLibrary(next);
            setWorkloadLibraryOutput(hubMessage(removedWorkloadMessage, { label: entry.label }));
        });
        controls.append(useButton, workbenchButton, inspectButton, validateButton, downloadButton, attachButton, removeButton);
        shell.append(summary, controls);
        fragment.appendChild(shell);
    });
    if (filteredEntries.length > visibleEntries.length) {
        countHubUiPerf("workload-library-overflow-renders");
        countHubUiPerf("workload-library-hidden-items", filteredEntries.length - visibleEntries.length);
        const overflow = document.createElement("div");
        overflow.className = "desktop-shell-note";
        overflow.textContent = `Showing ${visibleEntries.length} of ${filteredEntries.length} matching workloads. Narrow the search to reveal more.`;
        fragment.appendChild(overflow);
    }
    workloadLibraryList.appendChild(fragment);
    measureHubUiPerf("workload-library-render", "workload-library-render-start");
}
