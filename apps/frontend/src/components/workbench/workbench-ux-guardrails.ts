import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";

export type WorkbenchUxGuardrailTone = "ok" | "warn" | "block";

export type WorkbenchUxGuardrailInput = {
  frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
  healthStatus?: string;
  protocolOnline: boolean;
  watchdogOnline: boolean;
  controlPlaneApiToken: string;
  clusterApiToken: string;
  directMeshApiToken: string;
  directMeshEndpointsText: string;
  selectedProjectId: string | null;
  selectedVersionId: string | null;
  languagePackCount: number;
};

export type WorkbenchUxGuardrailItem = {
  id: string;
  tone: WorkbenchUxGuardrailTone;
  title: string;
  detail: string;
  nextAction: string;
};

export type WorkbenchUxGuardrailSummary = {
  tone: WorkbenchUxGuardrailTone;
  blockedActionCount: number;
  warningCount: number;
  items: WorkbenchUxGuardrailItem[];
  nextAction: string;
};

type WorkbenchUxGuardrailPresentationCopy = Pick<
  WorkbenchCopy,
  | "assistantConfigureDirectMesh"
  | "assistantConfigureDirectMeshHint"
  | "assistantEmpty"
  | "assistantNeedsProjectHint"
  | "controlPlaneProtocol"
  | "controlPlaneTokenHelp"
  | "controlPlaneTokenPlaceholder"
  | "createProject"
  | "directMeshEndpointsHelp"
  | "directMeshTokenHelp"
  | "directMeshTokenPlaceholder"
  | "initialFailed"
  | "languagePackImport"
  | "languagePacksEmptyLabel"
  | "languagePacksHint"
  | "modelWorkspaceHint"
  | "noVersions"
  | "offline"
  | "projectRequired"
  | "ready"
  | "refresh"
  | "save"
  | "saveAs"
  | "watchdog"
>;

const CONTROL_PLANE_TOKEN_MIN_LENGTH = 8;

export function buildWorkbenchUxGuardrailSummary(input: WorkbenchUxGuardrailInput): WorkbenchUxGuardrailSummary {
  const items = [
    ...runtimeGuardrails(input),
    ...workspaceGuardrails(input),
    ...experienceGuardrails(input),
  ];
  const blockedActionCount = items.filter((item) => item.tone === "block").length;
  const warningCount = items.filter((item) => item.tone === "warn").length;
  const tone: WorkbenchUxGuardrailTone = blockedActionCount > 0 ? "block" : warningCount > 0 ? "warn" : "ok";
  return {
    tone,
    blockedActionCount,
    warningCount,
    items: items.length > 0 ? items : [okItem()],
    nextAction: items.find((item) => item.tone === "block")?.nextAction ?? items[0]?.nextAction ?? "Keep working.",
  };
}

export function localizeWorkbenchUxGuardrailSummary(
  summary: WorkbenchUxGuardrailSummary,
  copy: WorkbenchUxGuardrailPresentationCopy,
): WorkbenchUxGuardrailSummary {
  const items = summary.items.map((guardrail) => ({
    ...guardrail,
    ...localizedGuardrailCopy(guardrail.id, copy),
  }));
  return {
    ...summary,
    items,
    nextAction: items.find((guardrail) => guardrail.tone === "block")?.nextAction
      ?? items[0]?.nextAction
      ?? copy.ready,
  };
}

function runtimeGuardrails(input: WorkbenchUxGuardrailInput): WorkbenchUxGuardrailItem[] {
  const items: WorkbenchUxGuardrailItem[] = [];
  if (input.frontendRuntimeMode === "orchestrated_gui") {
    if (input.healthStatus !== "ok") {
      items.push(item("backend-offline", "block", "Control plane is offline", "Workbench actions that need orchestration may not apply.", "Open System > Runtime and start or refresh the control plane."));
    }
    if (input.controlPlaneApiToken.trim().length > 0 && input.controlPlaneApiToken.trim().length < CONTROL_PLANE_TOKEN_MIN_LENGTH) {
      items.push(item("short-control-token", "warn", "Control token looks incomplete", "A short token is often a pasted fragment and can cause silent auth failures.", "Paste the full token or clear the field before retrying."));
    }
  }
  if (input.frontendRuntimeMode === "direct_mesh_gui") {
    if (input.directMeshEndpointsText.trim().length === 0) {
      items.push(item("missing-mesh-endpoints", "block", "No direct mesh endpoint", "Direct mesh mode needs at least one reachable agent endpoint.", "Add an agent endpoint or switch back to orchestrated mode."));
    }
    if (input.directMeshApiToken.trim().length === 0 && input.clusterApiToken.trim().length === 0) {
      items.push(item("missing-mesh-token", "warn", "Mesh token is not configured", "Some mesh agents reject unauthenticated requests.", "Configure a direct mesh or cluster token before running distributed work."));
    }
  }
  if (!input.protocolOnline) {
    items.push(item("protocol-offline", "warn", "Protocol probe is offline", "Agent and runtime descriptors may be stale.", "Refresh runtime status before trusting agent availability."));
  }
  if (!input.watchdogOnline) {
    items.push(item("watchdog-offline", "warn", "Watchdog is offline", "Long-running jobs may not report failure reasons cleanly.", "Start the watchdog or treat the next run as manual supervision."));
  }
  return items;
}

function workspaceGuardrails(input: WorkbenchUxGuardrailInput): WorkbenchUxGuardrailItem[] {
  if (!input.selectedProjectId) {
    return [item("no-project", "warn", "No active project", "Store installs, result retention, and workflow packages need a project context.", "Create or select a project before installing assets or saving results.")];
  }
  if (!input.selectedVersionId) {
    return [item("no-version", "warn", "No selected model version", "Reproducible runs are easier when the current model version is pinned.", "Save or select a model version before long runs.")];
  }
  return [];
}

function experienceGuardrails(input: WorkbenchUxGuardrailInput): WorkbenchUxGuardrailItem[] {
  if (input.languagePackCount === 0) {
    return [item("no-language-pack", "warn", "No installed language pack", "The UI can still run, but future localized copy audits have no installed pack to validate.", "Install or export a language pack from System > Config.")];
  }
  return [];
}

function item(id: string, tone: WorkbenchUxGuardrailTone, title: string, detail: string, nextAction: string): WorkbenchUxGuardrailItem {
  return { id, tone, title, detail, nextAction };
}

function okItem(): WorkbenchUxGuardrailItem {
  return item("ready", "ok", "Ready for guided operation", "No blocking UX guardrail is active.", "Continue with the current workflow.");
}

function localizedGuardrailCopy(
  id: string,
  copy: WorkbenchUxGuardrailPresentationCopy,
): Pick<WorkbenchUxGuardrailItem, "title" | "detail" | "nextAction"> | null {
  switch (id) {
    case "backend-offline":
    case "protocol-offline":
      return { title: `${copy.controlPlaneProtocol}: ${copy.offline}`, detail: copy.initialFailed, nextAction: copy.refresh };
    case "short-control-token":
      return { title: copy.controlPlaneTokenPlaceholder, detail: copy.controlPlaneTokenHelp, nextAction: copy.save };
    case "missing-mesh-endpoints":
      return {
        title: copy.assistantConfigureDirectMesh,
        detail: copy.assistantConfigureDirectMeshHint,
        nextAction: copy.directMeshEndpointsHelp,
      };
    case "missing-mesh-token":
      return { title: copy.directMeshTokenPlaceholder, detail: copy.directMeshTokenHelp, nextAction: copy.save };
    case "watchdog-offline":
      return { title: `${copy.watchdog}: ${copy.offline}`, detail: copy.initialFailed, nextAction: copy.refresh };
    case "no-project":
      return { title: copy.projectRequired, detail: copy.assistantNeedsProjectHint, nextAction: copy.createProject };
    case "no-version":
      return { title: copy.noVersions, detail: copy.modelWorkspaceHint, nextAction: copy.saveAs };
    case "no-language-pack":
      return { title: copy.languagePacksEmptyLabel, detail: copy.languagePacksHint, nextAction: copy.languagePackImport };
    case "ready":
      return { title: copy.ready, detail: copy.assistantEmpty, nextAction: copy.ready };
    default:
      return null;
  }
}
