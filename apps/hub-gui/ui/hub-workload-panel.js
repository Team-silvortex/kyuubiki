import { renderHubWorkloadLibraryList, renderWorkloadFilters as renderWorkloadFiltersModule } from "./hub-workload-list.js";
import { attachCurrentBundleToWorkload as attachCurrentBundleToWorkloadModule, clearHubWorkloadLibrary as clearHubWorkloadLibraryModule, currentWorkloadLibrarySearchQuery as currentWorkloadLibrarySearchQueryModule, downloadRemoteWorkloadBundle as downloadRemoteWorkloadBundleModule, exportHubWorkloadLibrary as exportHubWorkloadLibraryModule, importHubWorkloadLibrary as importHubWorkloadLibraryModule, matchesWorkloadFamilyFilter as matchesWorkloadFamilyFilterModule, matchesWorkloadFilter as matchesWorkloadFilterModule, matchesWorkloadSearchQuery as matchesWorkloadSearchQueryModule, openWorkloadInWorkbench as openWorkloadInWorkbenchModule, registerCurrentBundleAsWorkload as registerCurrentBundleAsWorkloadModule, saveHubWorkloadLibrary as saveHubWorkloadLibraryModule, syncLocalControlPlaneWorkloads as syncLocalControlPlaneWorkloadsModule, syncRemoteWorkloadCatalog as syncRemoteWorkloadCatalogModule, } from "./hub-workload-runtime.js";
export function createHubWorkloadPanel(context) {
    const workloadRuntimeContext = () => ({
        downloadHubBlob: context.downloadHubBlob,
        downloadHubJson: context.downloadHubJson,
        elements: context.elements,
        ensureDefaultWorkloadCatalogUrl: context.ensureDefaultWorkloadCatalogUrl,
        ensureRemoteHostTrust: context.ensureRemoteHostTrust,
        inferDownloadFilename: context.inferDownloadFilename,
        invokeTauri: context.invokeTauri,
        loadHubWorkloadLibrary: context.loadHubWorkloadLibrary,
        mergeHubWorkloadLibrary: context.mergeHubWorkloadLibrary,
        normalizeHubWorkloadEntry: context.normalizeHubWorkloadEntry,
        persistHubWorkloadLibrary: context.persistHubWorkloadLibrary,
        projectSummaryFromInspectPayload: context.projectSummaryFromInspectPayload,
        renderAssistantContext: context.renderAssistantContext,
        renderHubAssistantLocalCards: context.renderHubAssistantLocalCards,
        renderHubWorkloadLibrary,
        runAction: context.runAction,
        saveHubWorkloadLibrary,
        setWorkloadLibraryOutput: context.setWorkloadLibraryOutput,
        state: context.state,
        validateHubCatalogUrl: context.validateHubCatalogUrl,
        validateRemoteWorkloadCatalogPayload: context.validateRemoteWorkloadCatalogPayload,
        normalizeRemoteWorkloadCatalogPayload: context.normalizeRemoteWorkloadCatalogPayload,
        workloadIdentity: context.workloadIdentity,
    });
    async function downloadRemoteWorkloadBundle(entry) {
        return downloadRemoteWorkloadBundleModule(workloadRuntimeContext(), entry);
    }
    async function openWorkloadInWorkbench(entry) {
        return openWorkloadInWorkbenchModule(workloadRuntimeContext(), entry);
    }
    async function attachCurrentBundleToWorkload(entry) {
        return attachCurrentBundleToWorkloadModule(workloadRuntimeContext(), entry);
    }
    function saveHubWorkloadLibrary(entries) {
        return saveHubWorkloadLibraryModule(workloadRuntimeContext(), entries);
    }
    function matchesWorkloadFilter(entry) {
        return matchesWorkloadFilterModule(workloadRuntimeContext(), entry);
    }
    function matchesWorkloadFamilyFilter(entry) {
        return matchesWorkloadFamilyFilterModule(workloadRuntimeContext(), entry);
    }
    function currentWorkloadLibrarySearchQuery() {
        return currentWorkloadLibrarySearchQueryModule(workloadRuntimeContext());
    }
    function matchesWorkloadSearchQuery(entry) {
        return matchesWorkloadSearchQueryModule(workloadRuntimeContext(), entry);
    }
    function renderWorkloadFilters() {
        return renderWorkloadFiltersModule({
            elements: context.elements,
            state: context.state,
        });
    }
    function renderHubWorkloadLibrary(entries = context.loadHubWorkloadLibrary()) {
        const domainLabel = context.localizedWorkloadFilterLabel(context.state.workloadFilter);
        const familyLabel = context.localizedWorkloadFamilyFilterLabel(context.state.workloadFamilyFilter);
        const query = currentWorkloadLibrarySearchQuery();
        return renderHubWorkloadLibraryList({
            elements: context.elements,
            entries,
            state: context.state,
            renderWorkloadFilters,
            renderEmptyHistoryState: context.renderEmptyHistoryState,
            emptyMessage: context.hubCopy().dynamic?.managedWorkloadsEmpty
                || context.hubI18n.en.dynamic?.managedWorkloadsEmpty
                || "No managed workloads yet. Register a current bundle or sync a remote catalog.",
            filterEmptyMessage: context.hubMessage(context.hubCopy().dynamic?.managedWorkloadsFilterEmpty
                || context.hubI18n.en.dynamic?.managedWorkloadsFilterEmpty
                || "No workloads match {domain} / {family} for \"{query}\".", {
                domain: domainLabel,
                family: familyLabel,
                query: query || "--",
            }),
            matchesWorkloadFilter,
            matchesWorkloadSearchQuery,
            workloadSourceBadge: context.workloadSourceBadge,
            formatProjectActionTime: context.formatProjectActionTime,
            appendTextElement: context.appendTextElement,
            workloadDomainLabel: context.workloadDomainLabel,
            workloadFamilyLabel: context.workloadFamilyLabel,
            workloadProvenanceLabel: context.workloadProvenanceLabel,
            currentSearchQuery: query,
            setWorkloadLibraryOutput: context.setWorkloadLibraryOutput,
            hubMessage: context.hubMessage,
            restoredWorkloadContextMessage: context.hubCopy().dynamic?.restoredWorkloadContext
                || context.hubI18n.en.dynamic?.restoredWorkloadContext
                || "restored workload context for {label}",
            loadedWorkloadContextMessage: context.hubCopy().dynamic?.loadedWorkloadContext
                || context.hubI18n.en.dynamic?.loadedWorkloadContext
                || "loaded {label} into the bundle path",
            removedWorkloadMessage: context.hubCopy().dynamic?.removedWorkload
                || context.hubI18n.en.dynamic?.removedWorkload
                || "removed {label} from the workload library",
            renderAssistantContext: context.renderAssistantContext,
            renderHubAssistantLocalCards: context.renderHubAssistantLocalCards,
            openWorkloadInWorkbench,
            formatHubOperatorError: context.formatHubOperatorError,
            runAction: context.runAction,
            downloadRemoteWorkloadBundle,
            attachCurrentBundleToWorkload,
            loadHubWorkloadLibrary: context.loadHubWorkloadLibrary,
            saveHubWorkloadLibrary,
            workloadIdentity: context.workloadIdentity,
            projectBundlePath: context.elements.projectBundlePath,
            workloadCatalogUrl: context.elements.workloadCatalogUrl,
            hubCopy: context.hubCopy,
            hubI18nEn: context.hubI18n.en,
        });
    }
    async function registerCurrentBundleAsWorkload() {
        return registerCurrentBundleAsWorkloadModule(workloadRuntimeContext());
    }
    async function syncRemoteWorkloadCatalog(urlOverride = "") {
        return syncRemoteWorkloadCatalogModule(workloadRuntimeContext(), urlOverride);
    }
    async function syncLocalControlPlaneWorkloads() {
        return syncLocalControlPlaneWorkloadsModule(workloadRuntimeContext());
    }
    function exportHubWorkloadLibrary() {
        return exportHubWorkloadLibraryModule(workloadRuntimeContext());
    }
    async function importHubWorkloadLibrary(file) {
        return importHubWorkloadLibraryModule(workloadRuntimeContext(), file);
    }
    function clearHubWorkloadLibrary() {
        return clearHubWorkloadLibraryModule(workloadRuntimeContext());
    }
    return {
        clearHubWorkloadLibrary,
        exportHubWorkloadLibrary,
        importHubWorkloadLibrary,
        registerCurrentBundleAsWorkload,
        renderHubWorkloadLibrary,
        saveHubWorkloadLibrary,
        syncLocalControlPlaneWorkloads,
        syncRemoteWorkloadCatalog,
    };
}
