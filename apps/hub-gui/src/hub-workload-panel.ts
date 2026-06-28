import { renderHubWorkloadLibraryList, renderWorkloadFilters as renderWorkloadFiltersModule } from "./hub-workload-list.js";
import {
  attachCurrentBundleToWorkload as attachCurrentBundleToWorkloadModule,
  clearHubWorkloadLibrary as clearHubWorkloadLibraryModule,
  currentWorkloadLibrarySearchQuery as currentWorkloadLibrarySearchQueryModule,
  downloadRemoteWorkloadBundle as downloadRemoteWorkloadBundleModule,
  exportHubWorkloadLibrary as exportHubWorkloadLibraryModule,
  importHubWorkloadLibrary as importHubWorkloadLibraryModule,
  matchesWorkloadFamilyFilter as matchesWorkloadFamilyFilterModule,
  matchesWorkloadFilter as matchesWorkloadFilterModule,
  matchesWorkloadSearchQuery as matchesWorkloadSearchQueryModule,
  openWorkloadInWorkbench as openWorkloadInWorkbenchModule,
  registerCurrentBundleAsWorkload as registerCurrentBundleAsWorkloadModule,
  saveHubWorkloadLibrary as saveHubWorkloadLibraryModule,
  syncLocalControlPlaneWorkloads as syncLocalControlPlaneWorkloadsModule,
  syncRemoteWorkloadCatalog as syncRemoteWorkloadCatalogModule,
} from "./hub-workload-runtime.js";
import type { HubCatalogValidation, HubWorkloadEntry } from "./hub-workload-library.js";
import type { HubI18nRegistry } from "./hub-i18n-types.js";

type HubWorkloadPanelElements = {
  projectBundlePath?: HTMLInputElement | null;
  workloadCatalogUrl?: HTMLInputElement | null;
  workloadLibrarySearch?: HTMLInputElement | null;
  workloadLabel?: HTMLInputElement | null;
  workloadLibraryList?: HTMLElement | null;
  workloadFilterButtons: HTMLElement[];
  workloadFamilyFilterButtons: HTMLElement[];
};

type HubWorkloadPanelState = {
  workloadFilter: string;
  workloadFamilyFilter: string;
};

type HubWorkloadPanelContext = {
  appendTextElement: (parent: HTMLElement, tagName: string, text: unknown, className?: string) => HTMLElement;
  downloadHubBlob: (filename: string, blob: Blob) => void;
  downloadHubJson: (filename: string, payload: unknown) => void;
  elements: HubWorkloadPanelElements;
  ensureDefaultWorkloadCatalogUrl: (persist?: boolean) => string;
  ensureRemoteHostTrust: (url: string | undefined, actionLabel: string) => boolean;
  formatHubOperatorError: (error: unknown, options?: Record<string, unknown>) => string;
  formatProjectActionTime: (value: string) => string;
  hubCopy: () => Record<string, any>;
  hubI18n: HubI18nRegistry;
  hubMessage: (template: string, values?: Record<string, unknown>) => string;
  inferDownloadFilename: (url: string | undefined) => string;
  invokeTauri: (command: string, args?: Record<string, unknown>) => Promise<string>;
  loadHubWorkloadLibrary: () => HubWorkloadEntry[];
  localizedWorkloadFamilyFilterLabel: (family: string) => string;
  localizedWorkloadFilterLabel: (filter: string) => string;
  mergeHubWorkloadLibrary: (existing: unknown[], incoming: unknown[]) => HubWorkloadEntry[];
  normalizeHubWorkloadEntry: (entry: unknown) => HubWorkloadEntry | null;
  normalizeRemoteWorkloadCatalogPayload: (payload: unknown, url: string) => HubWorkloadEntry[];
  persistHubWorkloadLibrary: (entries: HubWorkloadEntry[]) => void;
  projectSummaryFromInspectPayload: (raw: string) => Record<string, unknown>;
  renderAssistantContext: () => void;
  renderEmptyHistoryState: (container: HTMLElement, message: string) => void;
  renderHubAssistantLocalCards: () => void;
  runAction: (action: string) => Promise<unknown>;
  setWorkloadLibraryOutput: (value: string) => void;
  state: HubWorkloadPanelState;
  validateHubCatalogUrl: (value: unknown) => HubCatalogValidation;
  validateRemoteWorkloadCatalogPayload: (payload: unknown) => HubCatalogValidation;
  workloadDomainLabel: (domain: string) => string;
  workloadFamilyLabel: (family: string) => string;
  workloadIdentity: (entry: unknown) => string;
  workloadProvenanceLabel: (entry: HubWorkloadEntry) => string;
  workloadSourceBadge: (entry: HubWorkloadEntry) => [string, string];
};

type HubWorkloadPanelApi = {
  clearHubWorkloadLibrary: () => void;
  exportHubWorkloadLibrary: () => void;
  importHubWorkloadLibrary: (file?: File | null) => Promise<void>;
  registerCurrentBundleAsWorkload: () => Promise<void>;
  renderHubWorkloadLibrary: (entries?: HubWorkloadEntry[]) => void;
  saveHubWorkloadLibrary: (entries: HubWorkloadEntry[]) => void;
  syncLocalControlPlaneWorkloads: () => Promise<void>;
  syncRemoteWorkloadCatalog: (urlOverride?: string) => Promise<void>;
};

export function createHubWorkloadPanel(context: HubWorkloadPanelContext): HubWorkloadPanelApi {
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

  async function downloadRemoteWorkloadBundle(entry: HubWorkloadEntry): Promise<void> {
    return downloadRemoteWorkloadBundleModule(workloadRuntimeContext(), entry);
  }

  async function openWorkloadInWorkbench(entry: HubWorkloadEntry): Promise<void> {
    return openWorkloadInWorkbenchModule(workloadRuntimeContext(), entry);
  }

  async function attachCurrentBundleToWorkload(entry: HubWorkloadEntry): Promise<void> {
    return attachCurrentBundleToWorkloadModule(workloadRuntimeContext(), entry);
  }

  function saveHubWorkloadLibrary(entries: HubWorkloadEntry[]): void {
    return saveHubWorkloadLibraryModule(workloadRuntimeContext(), entries);
  }

  function matchesWorkloadFilter(entry: HubWorkloadEntry): boolean {
    return matchesWorkloadFilterModule(workloadRuntimeContext(), entry);
  }

  function matchesWorkloadFamilyFilter(entry: HubWorkloadEntry): boolean {
    return matchesWorkloadFamilyFilterModule(workloadRuntimeContext(), entry);
  }

  function currentWorkloadLibrarySearchQuery(): string {
    return currentWorkloadLibrarySearchQueryModule(workloadRuntimeContext());
  }

  function matchesWorkloadSearchQuery(entry: HubWorkloadEntry): boolean {
    return matchesWorkloadSearchQueryModule(workloadRuntimeContext(), entry);
  }

  function renderWorkloadFilters(): void {
    return renderWorkloadFiltersModule({
      elements: context.elements,
      state: context.state,
    });
  }

  function renderHubWorkloadLibrary(entries = context.loadHubWorkloadLibrary()): void {
    const domainLabel = context.localizedWorkloadFilterLabel(context.state.workloadFilter);
    const familyLabel = context.localizedWorkloadFamilyFilterLabel(context.state.workloadFamilyFilter);
    const query = currentWorkloadLibrarySearchQuery();

    return renderHubWorkloadLibraryList({
      elements: context.elements,
      entries,
      state: context.state,
      renderWorkloadFilters,
      renderEmptyHistoryState: context.renderEmptyHistoryState,
      emptyMessage:
        context.hubCopy().dynamic?.managedWorkloadsEmpty
        || context.hubI18n.en.dynamic?.managedWorkloadsEmpty
        || "No managed workloads yet. Register a current bundle or sync a remote catalog.",
      filterEmptyMessage: context.hubMessage(
        context.hubCopy().dynamic?.managedWorkloadsFilterEmpty
          || context.hubI18n.en.dynamic?.managedWorkloadsFilterEmpty
          || "No workloads match {domain} / {family} for \"{query}\".",
        {
          domain: domainLabel,
          family: familyLabel,
          query: query || "--",
        },
      ),
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
      restoredWorkloadContextMessage:
        context.hubCopy().dynamic?.restoredWorkloadContext
        || context.hubI18n.en.dynamic?.restoredWorkloadContext
        || "restored workload context for {label}",
      loadedWorkloadContextMessage:
        context.hubCopy().dynamic?.loadedWorkloadContext
        || context.hubI18n.en.dynamic?.loadedWorkloadContext
        || "loaded {label} into the bundle path",
      removedWorkloadMessage:
        context.hubCopy().dynamic?.removedWorkload
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

  async function registerCurrentBundleAsWorkload(): Promise<void> {
    return registerCurrentBundleAsWorkloadModule(workloadRuntimeContext());
  }

  async function syncRemoteWorkloadCatalog(urlOverride = ""): Promise<void> {
    return syncRemoteWorkloadCatalogModule(workloadRuntimeContext(), urlOverride);
  }

  async function syncLocalControlPlaneWorkloads(): Promise<void> {
    return syncLocalControlPlaneWorkloadsModule(workloadRuntimeContext());
  }

  function exportHubWorkloadLibrary(): void {
    return exportHubWorkloadLibraryModule(workloadRuntimeContext());
  }

  async function importHubWorkloadLibrary(file?: File | null): Promise<void> {
    return importHubWorkloadLibraryModule(workloadRuntimeContext(), file);
  }

  function clearHubWorkloadLibrary(): void {
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
