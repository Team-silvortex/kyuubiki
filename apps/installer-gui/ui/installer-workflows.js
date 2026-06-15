import { applyDesktopState } from "./shared/tauri-bridge.js";
import { desktopPlatformLabel } from "./shared/platform.js";

export function applyPreset(mode, defaults) {
  const storageMode = document.getElementById("storage-mode");
  const deploymentMode = document.getElementById("deployment-mode");
  const agentDiscovery = document.getElementById("agent-discovery");
  const agentManifestPath = document.getElementById("agent-manifest-path");

  if (mode === "local") {
    deploymentMode.value = "local";
    agentDiscovery.value = "static";
    storageMode.value = "sqlite";
    if (!document.getElementById("sqlite-path").value.trim()) {
      document.getElementById("sqlite-path").value = defaults.sqliteDatabasePath;
    }
    if (!agentManifestPath.value.trim()) {
      agentManifestPath.value = defaults.agentManifestPath;
    }
  } else if (mode === "cloud") {
    deploymentMode.value = "cloud";
    agentDiscovery.value = "static";
    storageMode.value = "postgres";
    if (!document.getElementById("database-url").value.trim()) {
      document.getElementById("database-url").value = "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev";
    }
  } else {
    deploymentMode.value = "distributed";
    agentDiscovery.value = "registry";
    storageMode.value = "postgres";
    if (!document.getElementById("database-url").value.trim()) {
      document.getElementById("database-url").value = "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev";
    }
    if (!agentManifestPath.value.trim()) {
      agentManifestPath.value = defaults.distributedAgentManifestPath;
    }
  }

  if (!document.getElementById("agent-endpoints").value.trim()) {
    document.getElementById("agent-endpoints").value = "127.0.0.1:5001,127.0.0.1:5002";
  }
}

export function currentRemoteBootstrapPayload() {
  return {
    sshUser: document.getElementById("remote-ssh-user").value.trim(),
    targetHost: document.getElementById("remote-target-host").value.trim(),
    remoteWorkspace: document.getElementById("remote-workspace").value.trim(),
    sshPort: document.getElementById("remote-ssh-port").value
      ? Number(document.getElementById("remote-ssh-port").value)
      : null,
  };
}

export function currentRemoteAgentPayload() {
  return {
    ...currentRemoteBootstrapPayload(),
    orchestratorUrl: document.getElementById("remote-orchestrator-url").value.trim(),
    agentId: document.getElementById("remote-agent-id").value.trim(),
    advertiseHost: document.getElementById("remote-advertise-host").value.trim(),
    agentPort: Number(document.getElementById("remote-agent-port").value || "5001"),
  };
}

export function renderDoctor(report, platformLabel, workspaceLabel, doctorGrid) {
  platformLabel.textContent = desktopPlatformLabel(report.platform);
  workspaceLabel.textContent = report.workspace;
  doctorGrid.innerHTML = "";

  report.checks.forEach((check) => {
    const card = document.createElement("article");
    card.className = "doctor-card desktop-shell-surface-card";
    const stateLabel = check.ok ? "ok" : "missing";
    card.innerHTML = `<strong>${check.label}</strong><span class="doctor-state ${stateLabel}">${stateLabel}</span>`;
    applyDesktopState(card.querySelector(".doctor-state"), stateLabel, { kind: "health" });
    doctorGrid.appendChild(card);
  });
}
