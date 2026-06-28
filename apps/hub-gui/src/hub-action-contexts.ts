type UnknownRecord = Record<string, unknown>;

export type HubBusySetter = (busy: boolean, state?: string) => void;
export type HubOutputSetter = (value: unknown) => void;
export type HubSectionSetter = (section: string) => void;
export type HubProjectPageSetter = (page: string) => void;
export type HubTauriInvoker = (command: string, payload?: UnknownRecord) => Promise<unknown>;

export type HubProjectActionDeps = UnknownRecord & {
  invokeTauri: HubTauriInvoker;
  setOperationOutput: HubOutputSetter;
  setSection: HubSectionSetter;
  setProjectsPage: HubProjectPageSetter;
  setBusy: HubBusySetter;
  runProjectBundleAction: (options: UnknownRecord) => Promise<unknown>;
  currentProjectBundlePayload: () => UnknownRecord;
  currentProjectBundleOutputPayload: () => UnknownRecord;
  currentProjectBundleComparePayload: () => UnknownRecord;
  setProjectBundleOutput: HubOutputSetter;
};

export type HubRuntimeActionDeps = UnknownRecord & {
  invokeGuardedMutation: (action: string, payload?: UnknownRecord) => Promise<unknown>;
  setOperationOutput: HubOutputSetter;
  refreshRuntimeStatus: () => Promise<unknown>;
  refreshHotRuntimeStatus: () => Promise<unknown>;
  refreshHotRuntimeLog: () => Promise<unknown>;
  refreshObserveRuntimeLog: () => Promise<unknown>;
  copyHotRuntimeLogView: () => Promise<unknown>;
  copyObserveRuntimeLogView: () => Promise<unknown>;
  clearHotRuntimeLogView: () => void;
  currentHotRuntimeLogService: () => string;
  currentObserveRuntimeLogService: () => string;
  hubDynamic: (key: string, replacements?: UnknownRecord) => string;
  setBusy: HubBusySetter;
};

export type HubWorkloadActionDeps = UnknownRecord & {
  registerCurrentBundleAsWorkload: () => Promise<unknown>;
  syncLocalControlPlaneWorkloads: () => Promise<unknown>;
  syncRemoteWorkloadCatalog: () => Promise<unknown>;
  exportHubWorkloadLibrary: () => void;
  clearHubWorkloadLibrary: () => void;
  fetchWorkflowCatalog: () => Promise<unknown>;
  workloadImportInput?: HTMLElement | null;
  setBusy: HubBusySetter;
};

export type HubDesktopActionDeps = UnknownRecord & {
  invokeTauri: HubTauriInvoker;
  setOperationOutput: HubOutputSetter;
  setSection: HubSectionSetter;
  setBusy: HubBusySetter;
  refreshDesktopStatusOutput: () => Promise<unknown>;
  hubDynamic: (key: string, replacements?: UnknownRecord) => string;
};

export function buildHubProjectActionContext(deps: HubProjectActionDeps): HubProjectActionDeps {
  return {
    invokeTauri: deps.invokeTauri,
    setOperationOutput: deps.setOperationOutput,
    setSection: deps.setSection,
    setProjectsPage: deps.setProjectsPage,
    setBusy: deps.setBusy,
    runProjectBundleAction: deps.runProjectBundleAction,
    currentProjectBundlePayload: deps.currentProjectBundlePayload,
    currentProjectBundleOutputPayload: deps.currentProjectBundleOutputPayload,
    currentProjectBundleComparePayload: deps.currentProjectBundleComparePayload,
    setProjectBundleOutput: deps.setProjectBundleOutput,
  };
}

export function buildHubRuntimeActionContext(deps: HubRuntimeActionDeps): HubRuntimeActionDeps {
  return {
    invokeGuardedMutation: deps.invokeGuardedMutation,
    setOperationOutput: deps.setOperationOutput,
    refreshRuntimeStatus: deps.refreshRuntimeStatus,
    refreshHotRuntimeStatus: deps.refreshHotRuntimeStatus,
    refreshHotRuntimeLog: deps.refreshHotRuntimeLog,
    refreshObserveRuntimeLog: deps.refreshObserveRuntimeLog,
    copyHotRuntimeLogView: deps.copyHotRuntimeLogView,
    copyObserveRuntimeLogView: deps.copyObserveRuntimeLogView,
    clearHotRuntimeLogView: deps.clearHotRuntimeLogView,
    currentHotRuntimeLogService: deps.currentHotRuntimeLogService,
    currentObserveRuntimeLogService: deps.currentObserveRuntimeLogService,
    hubDynamic: deps.hubDynamic,
    setBusy: deps.setBusy,
  };
}

export function buildHubWorkloadActionContext(deps: HubWorkloadActionDeps): HubWorkloadActionDeps {
  return {
    registerCurrentBundleAsWorkload: deps.registerCurrentBundleAsWorkload,
    syncLocalControlPlaneWorkloads: deps.syncLocalControlPlaneWorkloads,
    syncRemoteWorkloadCatalog: deps.syncRemoteWorkloadCatalog,
    exportHubWorkloadLibrary: deps.exportHubWorkloadLibrary,
    clearHubWorkloadLibrary: deps.clearHubWorkloadLibrary,
    fetchWorkflowCatalog: deps.fetchWorkflowCatalog,
    workloadImportInput: deps.workloadImportInput,
    setBusy: deps.setBusy,
  };
}

export function buildHubDesktopActionContext(deps: HubDesktopActionDeps): HubDesktopActionDeps {
  return {
    invokeTauri: deps.invokeTauri,
    setOperationOutput: deps.setOperationOutput,
    setSection: deps.setSection,
    setBusy: deps.setBusy,
    refreshDesktopStatusOutput: deps.refreshDesktopStatusOutput,
    hubDynamic: deps.hubDynamic,
  };
}
