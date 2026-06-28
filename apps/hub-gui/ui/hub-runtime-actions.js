export async function runHubRuntimeAction(action, options) {
    switch (action) {
        case "start-local":
            options.setOperationOutput(await options.invokeGuardedMutation("service_start", { mode: "local" }));
            await options.refreshRuntimeStatus();
            options.setBusy(false, "ready");
            return true;
        case "start-cloud":
            options.setOperationOutput(await options.invokeGuardedMutation("service_start", { mode: "cloud" }));
            await options.refreshRuntimeStatus();
            options.setBusy(false, "ready");
            return true;
        case "start-distributed":
            options.setOperationOutput(await options.invokeGuardedMutation("service_start", { mode: "distributed" }));
            await options.refreshRuntimeStatus();
            options.setBusy(false, "ready");
            return true;
        case "restart-local":
            options.setOperationOutput(await options.invokeGuardedMutation("service_restart", { mode: "local" }));
            await options.refreshRuntimeStatus();
            options.setBusy(false, "ready");
            return true;
        case "stop-stack":
            options.setOperationOutput(await options.invokeGuardedMutation("service_stop"));
            await options.refreshRuntimeStatus();
            options.setBusy(false, "idle");
            return true;
        case "validate-env":
            options.setOperationOutput(await options.invokeGuardedMutation("validate_env"));
            options.setBusy(false, "ready");
            return true;
        case "hot-start-local":
            options.setOperationOutput(await options.invokeGuardedMutation("hot_service_start", { mode: "local" }));
            await options.refreshHotRuntimeStatus();
            options.setBusy(false, "ready");
            return true;
        case "hot-start-cloud":
            options.setOperationOutput(await options.invokeGuardedMutation("hot_service_start", { mode: "cloud" }));
            await options.refreshHotRuntimeStatus();
            options.setBusy(false, "ready");
            return true;
        case "hot-start-distributed":
            options.setOperationOutput(await options.invokeGuardedMutation("hot_service_start", { mode: "distributed" }));
            await options.refreshHotRuntimeStatus();
            options.setBusy(false, "ready");
            return true;
        case "hot-stop":
            options.setOperationOutput(await options.invokeGuardedMutation("hot_service_stop"));
            await options.refreshHotRuntimeStatus();
            options.setBusy(false, "idle");
            return true;
        case "hot-refresh-status":
            await options.refreshHotRuntimeStatus();
            options.setOperationOutput(options.hubDynamic("hotStatusRefreshed"));
            options.setBusy(false, "ready");
            return true;
        case "hot-refresh-log":
            await options.refreshHotRuntimeLog();
            options.setOperationOutput(options.hubDynamic("hotLogRefreshed", {
                service: options.currentHotRuntimeLogService(),
            }));
            options.setBusy(false, "ready");
            return true;
        case "hot-copy-log-view":
            await options.copyHotRuntimeLogView();
            options.setOperationOutput(options.hubDynamic("hotLogCopied", {
                service: options.currentHotRuntimeLogService(),
            }));
            options.setBusy(false, "ready");
            return true;
        case "hot-clear-log-view":
            options.clearHotRuntimeLogView();
            options.setOperationOutput(options.hubDynamic("hotLogCleared", {
                service: options.currentHotRuntimeLogService(),
            }));
            options.setBusy(false, "idle");
            return true;
        case "observe-refresh-runtime-log":
            await options.refreshObserveRuntimeLog();
            options.setOperationOutput(options.hubDynamic("runtimeLogRefreshed", {
                service: options.currentObserveRuntimeLogService(),
            }));
            options.setBusy(false, "ready");
            return true;
        case "observe-copy-runtime-log":
            await options.copyObserveRuntimeLogView();
            options.setOperationOutput(options.hubDynamic("runtimeLogCopied", {
                service: options.currentObserveRuntimeLogService(),
            }));
            options.setBusy(false, "ready");
            return true;
        default:
            return false;
    }
}
