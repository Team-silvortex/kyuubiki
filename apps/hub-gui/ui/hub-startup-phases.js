import { markHubUiPerf, measureHubUiPerf } from "./hub-ui-performance.js";
function afterFirstPaint(task) {
    const schedule = window.requestAnimationFrame
        ? (callback) => window.requestAnimationFrame(() => window.requestAnimationFrame(callback))
        : (callback) => window.setTimeout(callback, 0);
    return new Promise((resolve) => {
        schedule(() => resolve(task()));
    });
}
function duringIdle(task) {
    return new Promise((resolve) => {
        const run = () => resolve(task());
        if (window.requestIdleCallback) {
            window.requestIdleCallback(run, { timeout: 1200 });
            return;
        }
        window.setTimeout(run, 48);
    });
}
async function settleStartup(label, task) {
    markHubUiPerf(`startup:${label}:start`);
    try {
        await task();
    }
    catch (error) {
        console.warn(`Hub startup phase failed: ${label}`, error);
    }
    finally {
        measureHubUiPerf(`startup:${label}`, `startup:${label}:start`);
    }
}
export async function runHubStartupPhases(context) {
    const { elements, state, loadDesktopLanguagePreference, rerenderLocalizedHubShell, enhanceHubAccessibility, loadHubDensitySettings, loadHubHotLogSettings, loadHubRuntimeLogSettings, renderHotRuntimeLogServiceLabel, syncDesktopStates, renderHubDensityToggles, renderPanelPages, renderHubRecents, applyAssistantSettings, renderAssistantPanel, setEventMessage, setSection, setBusy, } = context;
    markHubUiPerf("startup:interactive:start");
    state.language = await loadDesktopLanguagePreference();
    rerenderLocalizedHubShell();
    enhanceHubAccessibility();
    state.density = loadHubDensitySettings();
    applyRuntimeLogSettings(elements, loadHubHotLogSettings(), loadHubRuntimeLogSettings());
    renderHotRuntimeLogServiceLabel();
    syncDesktopStates();
    renderHubDensityToggles();
    ["runtimes", "deploy", "observe", "tools"].forEach((group) => renderPanelPages(group));
    renderHubRecents();
    applyAssistantSettings();
    renderAssistantPanel();
    rerenderLocalizedHubShell();
    syncDesktopStates();
    setSection(state.activeSection);
    setBusy(false, "idle");
    setEventMessage?.("Hub listeners are mounted.", "startup:ready");
    measureHubUiPerf("startup:interactive", "startup:interactive:start");
    void afterFirstPaint(() => runDeferredStartup(context));
}
function applyRuntimeLogSettings(elements, hotLogSettings, runtimeLogSettings) {
    if (elements.hotRuntimeLogService)
        elements.hotRuntimeLogService.value = hotLogSettings.service;
    if (elements.hotRuntimeLogAuto)
        elements.hotRuntimeLogAuto.checked = hotLogSettings.autoRefresh;
    if (elements.hotRuntimeLogInterval)
        elements.hotRuntimeLogInterval.value = hotLogSettings.interval;
    if (elements.observeRuntimeLogService)
        elements.observeRuntimeLogService.value = runtimeLogSettings.service;
    if (elements.observeRuntimeLogAuto)
        elements.observeRuntimeLogAuto.checked = runtimeLogSettings.autoRefresh;
}
async function runDeferredStartup({ applyBrand, fetchWorkflowCatalog, loadDirectMeshRegressionSnapshot, loadEnvironment, loadRegressionGateReport, refreshDesktopStatusOutput, refreshHotRuntimeStatus, refreshRuntimeStatus, rerenderLocalizedHubShell, syncDesktopStates, }) {
    await Promise.all([
        settleStartup("brand", applyBrand),
        settleStartup("environment", loadEnvironment),
    ]);
    syncDesktopStates();
    rerenderLocalizedHubShell();
    await duringIdle(() => settleStartup("runtime-status", refreshRuntimeStatus));
    await duringIdle(() => settleStartup("hot-runtime-status", refreshHotRuntimeStatus));
    await duringIdle(() => settleStartup("desktop-status", refreshDesktopStatusOutput));
    await duringIdle(() => settleStartup("workflow-catalog", () => fetchWorkflowCatalog({ silent: true })));
    await duringIdle(() => settleStartup("direct-mesh-regression", loadDirectMeshRegressionSnapshot));
    await duringIdle(() => settleStartup("regression-gate", loadRegressionGateReport));
    rerenderLocalizedHubShell();
}
