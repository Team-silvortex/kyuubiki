import type { HubDesktopActionDeps } from "./hub-action-contexts.js";

export async function runHubDesktopAction(
  action: string,
  options: HubDesktopActionDeps,
): Promise<boolean> {
  switch (action) {
    case "run-doctor": {
      const payload = await options.invokeTauri("doctor_report");
      options.setOperationOutput((payload as { rendered?: unknown }).rendered);
      options.setBusy(false, "ready");
      return true;
    }
    case "desktop-stage":
      options.setOperationOutput(await options.invokeTauri("launch_installer_gui"));
      options.setSection("deploy");
      options.setBusy(false, "ready");
      return true;
    case "desktop-status":
      await options.refreshDesktopStatusOutput();
      options.setOperationOutput(options.hubDynamic("packagingRefreshed"));
      options.setBusy(false, "ready");
      return true;
    case "desktop-verify":
      options.setOperationOutput(await options.invokeTauri("launch_installer_gui"));
      options.setSection("deploy");
      options.setBusy(false, "ready");
      return true;
    case "desktop-build-host":
      options.setOperationOutput(await options.invokeTauri("launch_installer_gui"));
      options.setSection("deploy");
      options.setBusy(false, "ready");
      return true;
    default:
      return false;
  }
}
