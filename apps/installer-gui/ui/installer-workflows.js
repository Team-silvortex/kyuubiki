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
    controlMode: document.getElementById("remote-control-mode").value || "orchestrated",
    orchestratorUrl: document.getElementById("remote-orchestrator-url").value.trim(),
    agentId: document.getElementById("remote-agent-id").value.trim(),
    advertiseHost: document.getElementById("remote-advertise-host").value.trim(),
    agentPort: Number(document.getElementById("remote-agent-port").value || "5001"),
    clusterId: document.getElementById("remote-cluster-id").value.trim(),
    certificateId: document.getElementById("remote-certificate-id").value.trim(),
    peerEndpoints: document.getElementById("remote-peer-endpoints").value
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean),
  };
}

export function currentRemotePolicyPayload() {
  return {
    allowedHosts: document.getElementById("remote-policy-allowed-hosts").value.trim(),
    allowedWorkspaceRoots: document.getElementById("remote-policy-allowed-workspaces").value.trim(),
  };
}

export function currentRemoteNodeRegistryPayload() {
  const raw = document.getElementById("remote-node-registry").value.trim();
  if (!raw) return { nodes: [] };
  return { nodes: JSON.parse(raw) };
}

export function hydrateRemotePolicy(policy) {
  if (!policy) return;
  document.getElementById("remote-policy-allowed-hosts").value = policy.allowed_hosts || "";
  document.getElementById("remote-policy-allowed-workspaces").value =
    policy.allowed_workspace_roots || "";
  document.getElementById("remote-policy-effective-hosts").textContent =
    policy.effective_allowed_hosts || "(unbounded)";
  document.getElementById("remote-policy-effective-workspaces").textContent =
    policy.effective_allowed_workspace_roots || "(unbounded)";
  document.getElementById("remote-policy-config-path").textContent = policy.config_path || "";
}

export function hydrateRemoteNodeRegistry(registry) {
  if (!registry) return;
  document.getElementById("remote-node-registry").value = JSON.stringify(registry.nodes || [], null, 2);
  document.getElementById("remote-node-summary").textContent = registry.rendered || "installer remote nodes";
}

export function applyRemoteNodeToForm(node) {
  if (!node) return;
  document.getElementById("remote-ssh-user").value = node.ssh_user || "";
  document.getElementById("remote-target-host").value = node.target_host || "";
  document.getElementById("remote-ssh-port").value = node.ssh_port ?? "";
  document.getElementById("remote-workspace").value = node.remote_workspace || "";
  document.getElementById("remote-control-mode").value = node.control_mode || "orchestrated";
  document.getElementById("remote-agent-id").value = node.agent_id || "";
  document.getElementById("remote-advertise-host").value = node.advertise_host || "";
  document.getElementById("remote-agent-port").value = node.agent_port ?? 5001;
  document.getElementById("remote-orchestrator-url").value = node.orchestrator_url || "";
  document.getElementById("remote-cluster-id").value = node.cluster_id || "";
  document.getElementById("remote-certificate-id").value = node.certificate_id || "";
  document.getElementById("remote-peer-endpoints").value = Array.isArray(node.peer_endpoints) ? node.peer_endpoints.join(",") : "";
}

export function appendRemoteNodeWorkflowSnapshot(node, snapshot, limit = 16) {
  const existing = Array.isArray(node.workflow_snapshots) ? node.workflow_snapshots : [];
  return [
    ...existing,
    {
      ...snapshot,
      recorded_at_unix_ms: snapshot.recorded_at_unix_ms ?? Date.now(),
    },
  ].slice(-limit);
}

export function withRemoteNodeStatus(node, patch) {
  const workflowSnapshot = patch.workflowSnapshot;
  const next = {
    ...node,
    ...patch,
  };
  delete next.workflowSnapshot;
  if (workflowSnapshot) {
    next.workflow_snapshots = appendRemoteNodeWorkflowSnapshot(node, workflowSnapshot);
  }
  return {
    ...next,
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
