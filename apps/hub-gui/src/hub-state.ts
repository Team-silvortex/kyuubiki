import { detectDesktopPlatform } from "./shared/platform.js";
import {
  loadHubAssistantTrustedHosts,
  loadHubTrustedHosts,
} from "./hub-storage.js";
import {
  HUB_DENSITY_DEFAULTS,
  HUB_REMOTE_TRUSTED_HOSTS_KEY,
} from "./hub-app-config.js";

export type HubPanelGroup = "runtimes" | "deploy" | "observe" | "tools";
export type HubAssistantMode = "local" | "llm";

export type HubState = {
  hostPlatform: string;
  activeSection: string;
  projectsPage: string;
  panelPages: Record<HubPanelGroup, string>;
  assistantOpen: boolean;
  isBusy: boolean;
  historyFilter: string;
  workloadFilter: string;
  workloadFamilyFilter: string;
  workflowCatalog: unknown[];
  workflowCatalogBusy: boolean;
  assistantMode: HubAssistantMode;
  assistantPlan: Record<string, unknown> | null;
  hotLogRefreshInFlight: boolean;
  runtimeLogRefreshInFlight: boolean;
  density: Record<string, boolean>;
  releaseVersion: string;
  releaseCodename: string;
  language: string;
  directMeshRegressionSnapshot: unknown | null;
  regressionGateReport: unknown | null;
  assistantApiKey: string;
  assistantTrustedHosts: Set<string>;
  remoteTrustedHosts: Set<string>;
};

export function createHubState(): HubState {
  return {
    hostPlatform: detectDesktopPlatform(),
    activeSection: "projects",
    projectsPage: "start",
    panelPages: {
      runtimes: "local",
      deploy: "modes",
      observe: "health",
      tools: "packages",
    },
    assistantOpen: false,
    isBusy: false,
    historyFilter: "all",
    workloadFilter: "all",
    workloadFamilyFilter: "all",
    workflowCatalog: [],
    workflowCatalogBusy: false,
    assistantMode: "local",
    assistantPlan: null,
    hotLogRefreshInFlight: false,
    runtimeLogRefreshInFlight: false,
    density: { ...HUB_DENSITY_DEFAULTS },
    releaseVersion: "",
    releaseCodename: "",
    language: "en",
    directMeshRegressionSnapshot: null,
    regressionGateReport: null,
    assistantApiKey: "",
    assistantTrustedHosts: loadHubAssistantTrustedHosts(),
    remoteTrustedHosts: loadHubTrustedHosts(HUB_REMOTE_TRUSTED_HOSTS_KEY),
  };
}
