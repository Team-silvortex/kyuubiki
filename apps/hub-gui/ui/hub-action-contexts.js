export function buildHubProjectActionContext(deps) {
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
export function buildHubRuntimeActionContext(deps) {
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
export function buildHubWorkloadActionContext(deps) {
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
export function buildHubDesktopActionContext(deps) {
    return {
        invokeTauri: deps.invokeTauri,
        setOperationOutput: deps.setOperationOutput,
        setSection: deps.setSection,
        setBusy: deps.setBusy,
        refreshDesktopStatusOutput: deps.refreshDesktopStatusOutput,
        hubDynamic: deps.hubDynamic,
    };
}
