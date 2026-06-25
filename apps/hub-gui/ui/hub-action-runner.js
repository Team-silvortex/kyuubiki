import { runHubDesktopAction } from "./hub-desktop-actions.js";
import { runHubProjectAction } from "./hub-project-actions.js";
import { runHubRuntimeAction } from "./hub-runtime-actions.js";
import { runHubWorkloadAction } from "./hub-workload-actions.js";
import {
  buildHubDesktopActionContext,
  buildHubProjectActionContext,
  buildHubRuntimeActionContext,
  buildHubWorkloadActionContext,
} from "./hub-action-contexts.js";
import {
  currentProjectBundleComparePayload,
  currentProjectBundleOutputPayload,
  currentProjectBundlePayload,
} from "./hub-project-bundles.js";

export function createHubActionRunner(context) {
  async function invokeGuardedMutation(action, payload = {}) {
    return context.invokeTauri("guarded_mutation_action", {
      payload: {
        action,
        ...payload,
      },
    });
  }

  async function runAction(action) {
    return runActionWithOptions(action, {});
  }

  function confirmHubDesktopAction(action) {
    const risk = context.directActionRisk[action] || "low";
    if (risk === "low") {
      return true;
    }

    const message =
      risk === "high"
        ? `High-risk desktop action: ${action}\n\nThis can build packages or rewrite bundle outputs.\n\nContinue?`
        : `Sensitive desktop action: ${action}\n\nPlease confirm before the Hub continues.\n\nContinue?`;
    return window.confirm(message);
  }

  async function runActionWithOptions(action, options = {}) {
    context.setEventMessage?.(`action received: ${action}`, "action:received");
    if (context.state.isBusy) {
      context.setOperationOutput(`Hub is still finishing the current action. Try again after the activity state returns to idle. Requested action: ${action}`);
      context.applyDesktopState(context.elements.actionState, "busy", { kind: "activity" });
      context.setEventMessage?.(`busy: ignored ${action}`, "action:busy");
      return;
    }

    if (!options.skipConfirmation && !confirmHubDesktopAction(action)) {
      context.setOperationOutput(`cancelled desktop action: ${action}`);
      context.applyDesktopState(context.elements.actionState, "cancelled", { kind: "activity" });
      return;
    }

    context.setBusy(true, "running");
    window.__kyuubikiHubActionStartedAt = Date.now();
    window.__kyuubikiHubLastAction = action;
    context.setEventMessage?.(`running: ${action}`, "action:running");

    try {
      let handled = false;
      if (await runHubProjectAction(action, buildHubProjectActionContext({
        invokeTauri: context.invokeTauri,
        setOperationOutput: context.setOperationOutput,
        setSection: context.setSection,
        setProjectsPage: context.setProjectsPage,
        setBusy: context.setBusy,
        runProjectBundleAction: context.runProjectBundleAction,
        currentProjectBundlePayload,
        currentProjectBundleOutputPayload,
        currentProjectBundleComparePayload,
        setProjectBundleOutput: context.setProjectBundleOutput,
      }))) {
        handled = true;
      }

      if (!handled && await runHubRuntimeAction(action, buildHubRuntimeActionContext({
        invokeGuardedMutation,
        setOperationOutput: context.setOperationOutput,
        refreshRuntimeStatus: context.refreshRuntimeStatus,
        refreshHotRuntimeStatus: context.refreshHotRuntimeStatus,
        refreshHotRuntimeLog: context.refreshHotRuntimeLog,
        refreshObserveRuntimeLog: context.refreshObserveRuntimeLog,
        copyHotRuntimeLogView: context.copyHotRuntimeLogView,
        copyObserveRuntimeLogView: context.copyObserveRuntimeLogView,
        clearHotRuntimeLogView: context.clearHotRuntimeLogView,
        currentHotRuntimeLogService: context.currentHotRuntimeLogService,
        currentObserveRuntimeLogService: context.currentObserveRuntimeLogService,
        hubDynamic: context.hubDynamic,
        setBusy: context.setBusy,
      }))) {
        handled = true;
      }

      if (!handled && await runHubWorkloadAction(action, buildHubWorkloadActionContext({
        registerCurrentBundleAsWorkload: context.registerCurrentBundleAsWorkload,
        syncLocalControlPlaneWorkloads: context.syncLocalControlPlaneWorkloads,
        syncRemoteWorkloadCatalog: context.syncRemoteWorkloadCatalog,
        exportHubWorkloadLibrary: context.exportHubWorkloadLibrary,
        clearHubWorkloadLibrary: context.clearHubWorkloadLibrary,
        fetchWorkflowCatalog: context.fetchWorkflowCatalog,
        workloadImportInput: context.elements.workloadImportInput,
        setBusy: context.setBusy,
      }))) {
        handled = true;
      }

      if (!handled && await runHubDesktopAction(action, buildHubDesktopActionContext({
        invokeTauri: context.invokeTauri,
        setOperationOutput: context.setOperationOutput,
        setSection: context.setSection,
        setBusy: context.setBusy,
        refreshDesktopStatusOutput: context.refreshDesktopStatusOutput,
        hubDynamic: context.hubDynamic,
      }))) {
        handled = true;
      }

      if (!handled) {
        context.setOperationOutput(`No Hub action handler is registered for: ${action}`);
        context.setEventMessage?.(`unhandled action: ${action}`, "action:missing");
      } else {
        window.__kyuubikiHubActionCompletedAt = Date.now();
        window.__kyuubikiHubLastCompletedAction = action;
        context.setEventMessage?.(`completed: ${action}`, "action:complete");
      }
    } catch (error) {
      context.setOperationOutput(context.formatHubOperatorError(error, {
        actionLabel: "This desktop action",
      }));
      context.setEventMessage?.(`failed: ${action}`, "action:failed");
      context.setBusy(false, "failed");
      return;
    } finally {
      if (context.state.isBusy) {
        context.setBusy(false, "idle");
      }
    }
  }

  return {
    invokeGuardedMutation,
    runAction,
    runActionWithOptions,
  };
}
