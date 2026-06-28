export const HUB_RECENTS_KEY = "kyuubiki.hub.recents.v1";
export const HUB_WORKLOAD_LIBRARY_KEY = "kyuubiki.hub.workloads.v1";
export const HUB_ASSISTANT_SETTINGS_KEY = "kyuubiki.hub.assistant.settings.v1";
export const HUB_ASSISTANT_LEGACY_SECRETS_KEY = "kyuubiki.hub.assistant.secrets.v1";
export const HUB_ASSISTANT_AUDIT_KEY = "kyuubiki.hub.assistant.audit.v1";
export const HUB_ASSISTANT_TRUSTED_HOSTS_KEY = "kyuubiki.hub.assistant.trusted-hosts.v1";
export const HUB_REMOTE_TRUSTED_HOSTS_KEY = "kyuubiki.hub.remote-trusted-hosts.v1";
export const HUB_HOT_LOG_SETTINGS_KEY = "kyuubiki.hub.hot-log-settings.v1";
export const HUB_RUNTIME_LOG_SETTINGS_KEY = "kyuubiki.hub.runtime-log-settings.v1";
export const HUB_DENSITY_SETTINGS_KEY = "kyuubiki.hub.density-settings.v1";

export const HUB_RECENTS_LIMIT = 6;
export const HUB_ACTION_HISTORY_LIMIT = 8;
export const HUB_ASSISTANT_AUDIT_LIMIT = 16;
export const HUB_WORKLOAD_LIBRARY_LIMIT = 32;
export const HUB_HOT_LOG_POLL_MS = 4000;

export const HUB_ASSISTANT_MODEL_PRESETS = ["gpt-5", "gpt-5-mini", "gpt-4.1", "custom"] as const;

export type HubAssistantModelPreset = (typeof HUB_ASSISTANT_MODEL_PRESETS)[number];
export type HubActionRisk = "low" | "sensitive" | "high";

export type HubAssistantAction = {
  id: string;
  summary: string;
  payloadExample: Record<string, unknown>;
};

export const HUB_ASSISTANT_ACTION_RISK: Record<string, HubActionRisk> = {
  "hub/focusSection": "low",
  "hub/openWorkbench": "low",
  "hub/openInstaller": "sensitive",
  "hub/openDocsIndex": "low",
  "hub/openCurrentLineDoc": "low",
  "hub/openOperationsDoc": "low",
  "hub/openTroubleshootingDoc": "low",
  "hub/startLocal": "sensitive",
  "hub/validateEnv": "low",
  "hub/desktopStage": "sensitive",
  "hub/desktopBuildHost": "high",
  "hub/desktopVerify": "sensitive",
  "hub/setBundleContext": "low",
  "hub/projectInspect": "low",
  "hub/projectValidate": "low",
  "hub/projectNormalize": "sensitive",
  "hub/projectUnpack": "sensitive",
  "hub/projectPack": "high",
  "hub/projectDiff": "low",
};

export const PROJECT_ACTION_LABELS: Record<string, string> = {
  "project inspect": "project-inspect",
  "project validate": "project-validate",
  "project normalize": "project-normalize",
  "project unpack": "project-unpack",
  "project pack": "project-pack",
  "project diff": "project-diff",
};

export const HUB_DIRECT_ACTION_RISK: Record<string, HubActionRisk> = {
  "open-installer": "sensitive",
  "workload-clear-library": "sensitive",
  "start-local": "sensitive",
  "start-cloud": "sensitive",
  "start-distributed": "sensitive",
  "restart-local": "sensitive",
  "stop-stack": "sensitive",
  "hot-start-local": "sensitive",
  "hot-start-cloud": "sensitive",
  "hot-start-distributed": "sensitive",
  "hot-stop": "sensitive",
  "project-normalize": "sensitive",
  "project-unpack": "sensitive",
  "project-pack": "high",
};

export const HUB_ASSISTANT_ACTIONS: HubAssistantAction[] = [
  { id: "hub/focusSection", summary: "Focus a Hub section.", payloadExample: { section: "projects" } },
  { id: "hub/openWorkbench", summary: "Open the Workbench desktop shell.", payloadExample: {} },
  { id: "hub/openInstaller", summary: "Open the Installer desktop shell.", payloadExample: {} },
  { id: "hub/openDocsIndex", summary: "Open the Hub documentation index.", payloadExample: {} },
  { id: "hub/openCurrentLineDoc", summary: "Open the current-line document.", payloadExample: {} },
  { id: "hub/openOperationsDoc", summary: "Open the operations guide.", payloadExample: {} },
  { id: "hub/openTroubleshootingDoc", summary: "Open the troubleshooting guide.", payloadExample: {} },
  { id: "hub/startLocal", summary: "Start the local stack.", payloadExample: {} },
  { id: "hub/validateEnv", summary: "Validate the desktop environment.", payloadExample: {} },
  { id: "hub/desktopStage", summary: "Open Installer for desktop staging work.", payloadExample: {} },
  { id: "hub/desktopBuildHost", summary: "Open Installer for host-bundle build work.", payloadExample: {} },
  { id: "hub/desktopVerify", summary: "Open Installer for desktop verification work.", payloadExample: {} },
  { id: "hub/setBundleContext", summary: "Fill Hub bundle path inputs.", payloadExample: { path: "", comparePath: "", out: "" } },
  { id: "hub/projectInspect", summary: "Inspect the selected project bundle.", payloadExample: { path: "" } },
  { id: "hub/projectValidate", summary: "Validate the selected project bundle.", payloadExample: { path: "" } },
  { id: "hub/projectNormalize", summary: "Normalize the selected project bundle.", payloadExample: { path: "", out: "" } },
  { id: "hub/projectUnpack", summary: "Unpack the selected project bundle.", payloadExample: { path: "", out: "" } },
  { id: "hub/projectPack", summary: "Pack a project directory into a bundle.", payloadExample: { path: "", out: "" } },
  { id: "hub/projectDiff", summary: "Diff two project bundles.", payloadExample: { leftPath: "", rightPath: "" } },
];

export const HUB_DENSITY_DEFAULTS: Record<string, boolean> = {
  "projects-workflow": false,
  "runtimes-remote-targets": false,
  "deploy-suggested-flow": false,
  "tools-output": false,
  "side-current-mode": false,
};
