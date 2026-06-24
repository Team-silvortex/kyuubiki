import { detectDesktopPlatform } from "./shared/platform.js";
import {
  loadHubAssistantTrustedHosts,
  loadHubTrustedHosts,
} from "./hub-storage.js";
import {
  HUB_DENSITY_DEFAULTS,
  HUB_REMOTE_TRUSTED_HOSTS_KEY,
} from "./hub-app-config.js";

export function createHubState() {
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
