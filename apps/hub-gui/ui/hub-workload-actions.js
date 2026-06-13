export async function runHubWorkloadAction(action, options) {
  switch (action) {
    case "workload-register-local":
      await options.registerCurrentBundleAsWorkload();
      options.setBusy(false, "ready");
      return true;
    case "workload-sync-local":
      await options.syncLocalControlPlaneWorkloads();
      options.setBusy(false, "ready");
      return true;
    case "workload-sync-remote":
      await options.syncRemoteWorkloadCatalog();
      options.setBusy(false, "ready");
      return true;
    case "workload-export-library":
      options.exportHubWorkloadLibrary();
      options.setBusy(false, "ready");
      return true;
    case "workload-import-library":
      options.workloadImportInput?.click();
      options.setBusy(false, "idle");
      return true;
    case "workload-clear-library":
      options.clearHubWorkloadLibrary();
      options.setBusy(false, "ready");
      return true;
    case "workflow-catalog-refresh":
      await options.fetchWorkflowCatalog();
      options.setBusy(false, "ready");
      return true;
    default:
      return false;
  }
}
