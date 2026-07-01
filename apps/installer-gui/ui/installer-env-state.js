import { applyDesktopState } from "./shared/tauri-bridge.js";

export const DEFAULT_PRESET = {
  agentManifestPath: "./deploy/agents.local.example.json",
  distributedAgentManifestPath: "./deploy/agents.distributed.example.json",
  sqliteDatabasePath: "./tmp/data/kyuubiki_dev.sqlite3",
};

export const SENSITIVE_ENV_FIELD_IDS = [
  "database-url",
  "api-token",
  "cluster-api-token",
  "direct-mesh-token",
];

export function createInstallerEnvState({ ids, ui, applyPreset }) {
  const setSensitiveFieldState = (input, configured, placeholders) => {
    if (!input) return;
    input.value = "";
    input.dataset.configured = configured ? "true" : "false";
    input.placeholder = configured ? placeholders.configured : placeholders.empty;
  };

  const currentMode = () => ids("deployment-mode").value || "local";

  const setModeCard = (mode) => {
    document.querySelectorAll("[data-mode-card]").forEach((card) => {
      card.classList.toggle("mode-card--active", card.dataset.modeCard === mode);
    });
    applyDesktopState(ui.currentModeLabel, mode, { kind: "activity" });
    applyDesktopState(ui.serviceModePill, `${mode} profile`, { kind: "activity" });
  };

  const currentEnvPayload = () => {
    const databaseUrlInput = ids("database-url");
    const apiTokenInput = ids("api-token");
    const clusterApiTokenInput = ids("cluster-api-token");
    const directMeshTokenInput = ids("direct-mesh-token");
    return {
      deploymentMode: ids("deployment-mode").value,
      agentDiscovery: ids("agent-discovery").value,
      agentManifestPath: ids("agent-manifest-path").value.trim(),
      storageBackend: ids("storage-mode").value,
      sqliteDatabasePath: ids("sqlite-path").value.trim(),
      databaseUrl: databaseUrlInput.value.trim(),
      databaseUrlConfigured: databaseUrlInput.dataset.configured === "true",
      agentEndpoints: ids("agent-endpoints").value.trim(),
      kyuubikiApiToken: apiTokenInput.value.trim(),
      kyuubikiApiTokenConfigured: apiTokenInput.dataset.configured === "true",
      kyuubikiClusterApiToken: clusterApiTokenInput.value.trim(),
      kyuubikiClusterApiTokenConfigured: clusterApiTokenInput.dataset.configured === "true",
      kyuubikiClusterAllowedAgentIds: ids("cluster-allowed-agent-ids").value.trim(),
      kyuubikiClusterAllowedClusterIds: ids("cluster-allowed-cluster-ids").value.trim(),
      kyuubikiClusterRequireFingerprint: ids("cluster-require-fingerprint").value === "true",
      kyuubikiClusterTimestampWindowMs: ids("cluster-timestamp-window").value.trim() || "30000",
      kyuubikiProtectReads: ids("protect-reads").value === "true",
      kyuubikiDirectMeshEnabled: ids("direct-mesh-enabled").value === "true",
      kyuubikiDirectMeshToken: directMeshTokenInput.value.trim(),
      kyuubikiDirectMeshTokenConfigured: directMeshTokenInput.dataset.configured === "true",
    };
  };

  const hydrateEnv = (form) => {
    if (!form) return;
    ids("deployment-mode").value = form.deployment_mode || "local";
    ids("agent-discovery").value = form.agent_discovery || "static";
    ids("agent-manifest-path").value = form.agent_manifest_path || DEFAULT_PRESET.agentManifestPath;
    ids("storage-mode").value = form.storage_backend || "sqlite";
    ids("sqlite-path").value = form.sqlite_database_path || DEFAULT_PRESET.sqliteDatabasePath;
    setSensitiveFieldState(ids("database-url"), form.database_url_configured === true, {
      configured: "configured; leave blank to keep current value",
      empty: "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev",
    });
    ids("agent-endpoints").value = form.agent_endpoints || "127.0.0.1:5001,127.0.0.1:5002";
    setSensitiveFieldState(ids("api-token"), form.kyuubiki_api_token_configured === true, {
      configured: "configured; leave blank to keep current token",
      empty: "optional shared token",
    });
    setSensitiveFieldState(ids("cluster-api-token"), form.kyuubiki_cluster_api_token_configured === true, {
      configured: "configured; leave blank to keep current token",
      empty: "optional cluster-only token",
    });
    ids("cluster-allowed-agent-ids").value = form.kyuubiki_cluster_allowed_agent_ids || "";
    ids("cluster-allowed-cluster-ids").value = form.kyuubiki_cluster_allowed_cluster_ids || "";
    ids("cluster-require-fingerprint").value = form.kyuubiki_cluster_require_fingerprint ? "true" : "false";
    ids("cluster-timestamp-window").value = form.kyuubiki_cluster_timestamp_window_ms || "30000";
    ids("protect-reads").value = form.kyuubiki_protect_reads ? "true" : "false";
    ids("direct-mesh-enabled").value = form.kyuubiki_direct_mesh_enabled === false ? "false" : "true";
    setSensitiveFieldState(ids("direct-mesh-token"), form.kyuubiki_direct_mesh_token_configured === true, {
      configured: "configured; leave blank to keep current token",
      empty: "optional direct-mesh token",
    });
    setModeCard(form.deployment_mode || "local");
  };

  return {
    currentEnvPayload,
    currentMode,
    hydrateEnv,
    setModeCard,
  };
}
