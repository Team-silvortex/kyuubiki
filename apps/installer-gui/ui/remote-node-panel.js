import { createRemoteNodeCertificateController } from "./remote-node-certificates.js";
import { createRemoteNodeMeshController } from "./remote-node-mesh.js";

export function mountRemoteNodePanel({
  invoke,
  runAction,
  invokeGuardedMutation,
  applyRemoteNodeToForm,
  getActiveCertificates,
  showCompletion,
  currentRemoteBootstrapPayload,
  currentRemoteAgentPayload,
  currentRemoteNodeRegistryPayload,
  hydrateRemoteNodeRegistry,
  withRemoteNodeStatus,
}) {
  const host = document.getElementById("remote-node-cards");
  const meshHealthHost = document.getElementById("remote-mesh-health");
  const certificateHealthHost = document.getElementById("remote-certificate-health");
  const meshIssuesHost = document.getElementById("remote-mesh-issues");
  const meshClustersHost = document.getElementById("remote-mesh-clusters");
  const searchInput = document.getElementById("remote-node-search");
  const filterSelect = document.getElementById("remote-node-filter");
  const modeFilterSelect = document.getElementById("remote-node-mode-filter");
  const certificateFilterSelect = document.getElementById("remote-node-certificate-filter");
  const certificateBulkActionButton = document.getElementById("remote-certificate-bulk-action");
  const groupSelect = document.getElementById("remote-node-group");
  let lastNodes = [];
  const statusOf = (node) => node.last_probe_status === "ok" ? "ok" : node.last_probe_status === "failed" ? "failed" : "unknown";
  const summary = (node) =>
    node.last_probe_summary || (node.last_probe_status ? `Last probe: ${node.last_probe_status}` : "No probe yet");
  const timeLabel = (unixMs) => (typeof unixMs === "number" && unixMs > 0 ? new Date(unixMs).toLocaleString() : "never");
  const syncRegistry = async (updater) => {
    const registry = currentRemoteNodeRegistryPayload();
    const nodes = updater(registry.nodes || []);
    await invokeGuardedMutation("write_remote_nodes", { remoteNodes: { nodes } });
    const fresh = await invoke("remote_node_registry");
    hydrateRemoteNodeRegistry(fresh);
    renderRemoteNodeCards(fresh.nodes || []);
    return fresh;
  };
  const visibleNodes = (nodes) => {
    const query = searchInput?.value.trim().toLowerCase() || "";
    const filter = filterSelect?.value || "all";
    const modeFilter = modeFilterSelect?.value || "all";
    const certificateFilter = certificateFilterSelect?.value || "all";
    return nodes.filter((node) => {
      const haystack = [
        node.label,
        node.target_host,
        node.agent_id,
        node.advertise_host,
        node.control_mode,
        node.cluster_id,
        ...(Array.isArray(node.peer_endpoints) ? node.peer_endpoints : []),
        node.last_probe_summary,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      const matchesQuery = !query || haystack.includes(query);
      const nodeStatus = statusOf(node);
      const matchesFilter = filter === "all" || filter === nodeStatus;
      const nodeMode = node.control_mode || "orchestrated";
      const matchesMode = modeFilter === "all" || modeFilter === nodeMode;
      const matchesCertificate = certificateFilter === "all" || certificateController.certificateStatusFor(node).tone === certificateFilter;
      return matchesQuery && matchesFilter && matchesMode && matchesCertificate;
    });
  };
  const validateMeshNode = (node) => {
    if ((node.control_mode || "orchestrated") !== "offline_mesh") {
      throw new Error("mesh preflight requires offline_mesh nodes");
    }
    if (!node.cluster_id?.trim()) {
      throw new Error("mesh preflight requires cluster_id");
    }
    if (!Array.isArray(node.peer_endpoints) || node.peer_endpoints.length === 0) {
      throw new Error("mesh preflight requires at least one peer endpoint");
    }
    return `cluster ${node.cluster_id} · peers ${node.peer_endpoints.length}`;
  };
  const certificateController = createRemoteNodeCertificateController({
    certificateHealthHost,
    certificateFilterSelect,
    certificateBulkActionButton,
    getActiveCertificates,
    getLastNodes: () => lastNodes,
    getVisibleNodes: visibleNodes,
    rerender: () => renderRemoteNodeCards(lastNodes),
    showCompletion,
    syncRegistry,
  });
  const groupKeyFor = (node) => {
    switch (groupSelect?.value || "none") {
      case "status":
        return statusOf(node);
      case "workspace":
        return node.remote_workspace || "(workspace unset)";
      case "control_mode":
        return node.control_mode || "orchestrated";
      case "orchestrator":
        return node.control_mode === "offline_mesh"
          ? node.cluster_id || "(mesh cluster unset)"
          : node.orchestrator_url || "(orchestrator unset)";
      default:
        return "__all__";
    }
  };
  const groupNodes = (nodes) => {
    const groups = new Map();
    nodes.forEach((node) => {
      const key = groupKeyFor(node);
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(node);
    });
    return groups;
  };
  const stamp = () => Date.now();
  const persistNodeState = (index, patch) =>
    syncRegistry((nodes) =>
      nodes.map((entry, idx) => (idx === index ? withRemoteNodeStatus(entry, patch) : entry)),
    );
  const runNodeAction = async (name, index, actionKind, runner, onSuccess) => {
    try {
      const result = await runAction(name, runner);
      await persistNodeState(index, {
        ...onSuccess(result),
        last_action: actionKind,
        last_action_unix_ms: stamp(),
      });
      return result;
    } catch (error) {
      await persistNodeState(index, {
        last_probe_status: actionKind === "probe" ? "failed" : "failed",
        last_probe_summary: error.message || String(error),
        last_probe_unix_ms: stamp(),
        last_action: `${actionKind}_failed`,
        last_action_unix_ms: stamp(),
      });
      throw error;
    }
  };

  function renderRemoteNodeCards(nodes) {
    lastNodes = Array.isArray(nodes) ? nodes : [];
    host.innerHTML = "";
    meshController.renderMeshDiagnostics(lastNodes);
    certificateController.renderCertificateHealth(lastNodes);
    const filteredNodes = visibleNodes(lastNodes);
    if (lastNodes.length === 0) {
      host.innerHTML = `<article class="remote-node-card"><strong>No registered nodes</strong><span>Save JSON nodes above to get one-click remote actions here.</span></article>`;
      return;
    }
    if (filteredNodes.length === 0) {
      host.innerHTML = `<article class="remote-node-card"><strong>No matching nodes</strong><span>Adjust the search text or status filter.</span></article>`;
      return;
    }

    const groups = groupNodes(filteredNodes);
    groups.forEach((groupNodesForKey, key) => {
      const groupShell = document.createElement("section");
      groupShell.className = "remote-node-group";
      if (key !== "__all__") {
        const title = document.createElement("h3");
        title.className = "remote-node-group__title";
        title.textContent = key;
        groupShell.appendChild(title);
      }
      groupNodesForKey.forEach((node) => {
      const index = lastNodes.indexOf(node);
      const card = document.createElement("article");
      card.className = "remote-node-card";
      card.dataset.nodeStatus = statusOf(node);
      const certificateStatus = certificateController.certificateStatusFor(node);
      card.dataset.certificateStatus = certificateStatus.tone;
      card.innerHTML = `
        <div class="remote-node-card__header">
          <strong>${node.label || node.target_host}</strong>
          <span>${node.ssh_user}@${node.target_host}:${node.ssh_port ?? 22}</span>
        </div>
        <div class="remote-node-card__meta">
          <span>${node.remote_workspace}</span>
          <span>${node.control_mode || "orchestrated"} · ${node.agent_id} · ${node.advertise_host}:${node.agent_port}</span>
          <span class="remote-node-card__certificate">
            <span class="remote-node-card__certificate-pill" data-certificate-tone="${certificateStatus.tone}">
              ${certificateStatus.tone}
            </span>
            <span>${certificateStatus.summary}</span>
          </span>
          <span>certificate ${certificateStatus.tone} · ${certificateStatus.detail}</span>
        </div>
        <div class="remote-node-card__status">
          <strong>${statusOf(node).toUpperCase()}</strong>
          <span>${node.control_mode === "offline_mesh" ? `cluster ${node.cluster_id || "(unset)"} · peers ${(node.peer_endpoints || []).length}` : node.orchestrator_url || "orchestrator unset"}</span>
          <span>${summary(node)}</span>
          <span>Probe: ${timeLabel(node.last_probe_unix_ms)} · Action: ${timeLabel(node.last_action_unix_ms)}</span>
        </div>
        <div class="action-row">
          <button data-remote-node-action="use" data-remote-node-index="${index}">Use</button>
          <button data-remote-node-action="probe" data-remote-node-index="${index}">Probe</button>
          <button data-remote-node-action="bootstrap" data-remote-node-index="${index}">Bootstrap</button>
          <button class="primary" data-remote-node-action="start" data-remote-node-index="${index}">Start agent</button>
        </div>
      `;
        groupShell.appendChild(card);
      });
      host.appendChild(groupShell);
    });
  }

  async function executeNodeAction(nodeIndex, actionKind) {
    const registry = await invoke("remote_node_registry");
    const node = registry.nodes?.[nodeIndex];
    if (!node) throw new Error("remote node no longer exists");
    applyRemoteNodeToForm(node);
    if (actionKind === "probe") {
      return runNodeAction("probe-remote-node-card", nodeIndex, "probe", () =>
        invokeGuardedMutation("probe_remote_node", {
          remoteBootstrap: currentRemoteBootstrapPayload(),
        }),
        (result) => ({
          last_probe_status: "ok",
          last_probe_summary: result,
          last_probe_unix_ms: stamp(),
        }),
      );
    }
    if (actionKind === "bootstrap") {
      return runNodeAction("remote-bootstrap-card", nodeIndex, "bootstrap", () =>
        invokeGuardedMutation("remote_bootstrap", {
          remoteBootstrap: currentRemoteBootstrapPayload(),
        }),
        () => ({}),
      );
    }
    if (actionKind === "start") {
      return runNodeAction("remote-start-agent-card", nodeIndex, "start_agent", () =>
        invokeGuardedMutation("remote_start_agent", {
          remoteAgent: currentRemoteAgentPayload(),
        }),
        (result) => ({
          last_probe_summary: typeof result === "string" ? result : "remote solver agent started",
        }),
      );
    }
    if (actionKind === "mesh-preflight") {
      const meshSummary = validateMeshNode(node);
      return runNodeAction("remote-mesh-preflight-card", nodeIndex, "mesh_preflight", async () => {
        const result = await invokeGuardedMutation("probe_remote_node", {
          remoteBootstrap: currentRemoteBootstrapPayload(),
        });
        return `${meshSummary} · ${result}`;
      }, (result) => ({
        last_probe_status: "ok",
        last_probe_summary: `mesh preflight ok · ${result}`,
        last_probe_unix_ms: stamp(),
      }));
    }
    return null;
  }

  const meshController = createRemoteNodeMeshController({
    meshHealthHost,
    meshIssuesHost,
    meshClustersHost,
    getLastNodes: () => lastNodes,
    modeFilterSelect,
    groupSelect,
    searchInput,
    rerender: () => renderRemoteNodeCards(lastNodes),
    showCompletion,
    executeNodeAction,
  });

  host.addEventListener("click", async (event) => {
    const target = event.target.closest("[data-remote-node-action]");
    if (!target) return;

    const registry = await invoke("remote_node_registry");
    const node = registry.nodes?.[Number(target.dataset.remoteNodeIndex)];
    if (!node) throw new Error("remote node no longer exists");
    applyRemoteNodeToForm(node);
    const nodeIndex = Number(target.dataset.remoteNodeIndex);

    switch (target.dataset.remoteNodeAction) {
      case "use":
        showCompletion(`Loaded remote node ${node.label || node.target_host}.`);
        break;
      case "probe":
        await executeNodeAction(nodeIndex, "probe");
        showCompletion(`Probed remote node ${node.label || node.target_host}.`);
        break;
      case "bootstrap":
        await executeNodeAction(nodeIndex, "bootstrap");
        showCompletion(`Bootstrapped remote node ${node.label || node.target_host}.`);
        break;
      case "start":
        await executeNodeAction(nodeIndex, "start");
        showCompletion(`Started agent on ${node.label || node.target_host}.`);
        break;
      default:
        break;
    }
  });
  document.querySelectorAll("[data-remote-bulk-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      const actionKind = button.dataset.remoteBulkAction;
      if (actionKind === "assign-certificates") {
        const result = await certificateController.assignCertificatesForVisibleNodes();
        showCompletion(
          `Assigned ${result.assigned} certificate binding(s); kept ${result.unchanged}; ${result.missing} missing; ${result.ambiguous} ambiguous across ${result.total} visible nodes.`,
        );
        return;
      }
      if (actionKind === "clear-certificates") {
        const result = await certificateController.clearCertificatesForVisibleNodes();
        showCompletion(`Cleared ${result.cleared} certificate binding(s) across ${result.total} visible nodes.`);
        return;
      }
      if (actionKind === "certificate-focus-action") {
        await certificateController.runFocusedCertificateAction();
        return;
      }
      const visible = visibleNodes(lastNodes).filter((node) =>
        actionKind === "mesh-preflight" ? (node.control_mode || "orchestrated") === "offline_mesh" : true,
      );
      if (visible.length === 0) throw new Error("no visible remote nodes");
      for (const node of visible) {
        const nodeIndex = lastNodes.indexOf(node);
        await executeNodeAction(nodeIndex, actionKind);
      }
      showCompletion(`Completed ${actionKind} for ${visible.length} visible nodes.`);
    });
  });
  meshController.bindEvents();
  certificateController.bindEvents();
  searchInput?.addEventListener("input", () => renderRemoteNodeCards(lastNodes));
  filterSelect?.addEventListener("change", () => renderRemoteNodeCards(lastNodes));
  modeFilterSelect?.addEventListener("change", () => renderRemoteNodeCards(lastNodes));
  certificateFilterSelect?.addEventListener("change", () => renderRemoteNodeCards(lastNodes));
  groupSelect?.addEventListener("change", () => renderRemoteNodeCards(lastNodes));

  return { renderRemoteNodeCards };
}
