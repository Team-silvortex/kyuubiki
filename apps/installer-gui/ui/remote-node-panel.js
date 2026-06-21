export function mountRemoteNodePanel({
  invoke,
  runAction,
  invokeGuardedMutation,
  applyRemoteNodeToForm,
  showCompletion,
  currentRemoteBootstrapPayload,
  currentRemoteAgentPayload,
  currentRemoteNodeRegistryPayload,
  hydrateRemoteNodeRegistry,
  withRemoteNodeStatus,
}) {
  const host = document.getElementById("remote-node-cards");
  const meshHealthHost = document.getElementById("remote-mesh-health");
  const meshIssuesHost = document.getElementById("remote-mesh-issues");
  const meshClustersHost = document.getElementById("remote-mesh-clusters");
  const searchInput = document.getElementById("remote-node-search");
  const filterSelect = document.getElementById("remote-node-filter");
  const modeFilterSelect = document.getElementById("remote-node-mode-filter");
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
      return matchesQuery && matchesFilter && matchesMode;
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
  const meshDiagnostics = (nodes) => {
    const meshNodes = nodes.filter((node) => (node.control_mode || "orchestrated") === "offline_mesh");
    const issues = [];
    const clusterMembers = new Map();
    const clusterState = new Map();
    meshNodes.forEach((node) => {
      const clusterId = node.cluster_id?.trim();
      if (!clusterId) {
        issues.push(`${node.label || node.target_host}: missing cluster_id`);
      } else {
        clusterMembers.set(clusterId, (clusterMembers.get(clusterId) || 0) + 1);
        if (!clusterState.has(clusterId)) {
          clusterState.set(clusterId, { nodes: [], peerCount: 0, readyCount: 0 });
        }
        const state = clusterState.get(clusterId);
        state.nodes.push(node);
        state.peerCount += Array.isArray(node.peer_endpoints) ? node.peer_endpoints.length : 0;
        if (node.last_probe_summary?.includes("mesh preflight ok")) state.readyCount += 1;
      }
      if (!Array.isArray(node.peer_endpoints) || node.peer_endpoints.length === 0) {
        issues.push(`${node.label || node.target_host}: missing peer_endpoints`);
      }
      if (node.last_action === "mesh_preflight_failed" || node.last_probe_summary?.includes("mesh preflight")) {
        if (node.last_probe_status === "failed") {
          issues.push(`${node.label || node.target_host}: ${node.last_probe_summary}`);
        }
      }
    });
    clusterMembers.forEach((count, clusterId) => {
      if (count < 2) issues.push(`cluster ${clusterId}: only ${count} mesh node configured`);
    });
    return {
      meshNodeCount: meshNodes.length,
      clusterCount: clusterMembers.size,
      healthyPreflightCount: meshNodes.filter((node) => node.last_probe_summary?.includes("mesh preflight ok")).length,
      issueCount: issues.length,
      issues,
      clusters: Array.from(clusterState.entries())
        .map(([clusterId, state]) => ({
          clusterId,
          nodeCount: state.nodes.length,
          peerCount: state.peerCount,
          readyCount: state.readyCount,
          missingPreflightCount: state.nodes.length - state.readyCount,
          missingStartCount: state.nodes.filter((node) => node.last_action !== "start_agent").length,
          readiness: state.nodes.length >= 2 && state.peerCount >= state.nodes.length && state.readyCount === state.nodes.length ? "ready" : "warning",
          finalReadiness:
            state.nodes.length >= 2 &&
            state.peerCount >= state.nodes.length &&
            state.readyCount === state.nodes.length &&
            state.nodes.every((node) => node.last_action === "start_agent")
              ? "go"
              : "hold",
        }))
        .sort((left, right) => left.clusterId.localeCompare(right.clusterId)),
    };
  };
  const renderMeshDiagnostics = (nodes) => {
    if (!meshHealthHost || !meshIssuesHost || !meshClustersHost) return;
    const diagnostics = meshDiagnostics(nodes);
    meshHealthHost.innerHTML = diagnostics.meshNodeCount === 0
      ? ""
      : `
        <article class="remote-mesh-health__metric"><strong>Mesh nodes</strong><span>${diagnostics.meshNodeCount}</span></article>
        <article class="remote-mesh-health__metric"><strong>Clusters</strong><span>${diagnostics.clusterCount}</span></article>
        <article class="remote-mesh-health__metric"><strong>Preflight ok</strong><span>${diagnostics.healthyPreflightCount}</span></article>
        <article class="remote-mesh-health__metric"><strong>Issues</strong><span>${diagnostics.issueCount}</span></article>
      `;
    meshIssuesHost.innerHTML = diagnostics.issues.length === 0
      ? (diagnostics.meshNodeCount === 0 ? "" : `<article class="remote-mesh-issues__item"><strong>Mesh ready</strong><span>No contract gaps detected in the current node registry.</span></article>`)
      : diagnostics.issues.map((issue) => `<article class="remote-mesh-issues__item"><strong>Mesh issue</strong><span>${issue}</span></article>`).join("");
    meshClustersHost.innerHTML = diagnostics.clusters.length === 0
      ? ""
      : diagnostics.clusters.map((cluster) => `
        <article class="remote-mesh-cluster" data-readiness="${cluster.readiness}">
          <strong>${cluster.clusterId}</strong>
          <span><strong>${cluster.finalReadiness === "go" ? "Ready for run" : "Hold"}</strong></span>
          <span>nodes ${cluster.nodeCount} · peers ${cluster.peerCount}</span>
          <span>preflight ok ${cluster.readyCount}/${cluster.nodeCount}</span>
          <span>agents started ${cluster.nodeCount - cluster.missingStartCount}/${cluster.nodeCount}</span>
          <span>${cluster.finalReadiness === "go" ? "preflight and agent startup aligned" : cluster.readiness === "ready" ? "start remaining agents before workload tests" : `needs ${cluster.missingPreflightCount} more verified node(s)`}</span>
          <div class="action-row">
            <button data-remote-cluster-action="focus-cluster" data-remote-cluster-id="${cluster.clusterId}">Focus cluster</button>
            ${cluster.missingStartCount === 0 ? "" : `<button data-remote-cluster-action="start-missing" data-remote-cluster-id="${cluster.clusterId}">Start missing agents</button>`}
            ${cluster.readiness === "ready" ? "" : `<button data-remote-cluster-action="preflight-missing" data-remote-cluster-id="${cluster.clusterId}">Preflight missing</button>`}
          </div>
        </article>
      `).join("");
  };
  const pendingClusterMeshNodes = (clusterId) =>
    lastNodes.filter((node) =>
      (node.control_mode || "orchestrated") === "offline_mesh" &&
      node.cluster_id === clusterId &&
      !node.last_probe_summary?.includes("mesh preflight ok"),
    );
  const pendingClusterStartNodes = (clusterId) =>
    lastNodes.filter((node) =>
      (node.control_mode || "orchestrated") === "offline_mesh" &&
      node.cluster_id === clusterId &&
      node.last_action !== "start_agent",
    );
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
    renderMeshDiagnostics(lastNodes);
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
      card.innerHTML = `
        <div class="remote-node-card__header">
          <strong>${node.label || node.target_host}</strong>
          <span>${node.ssh_user}@${node.target_host}:${node.ssh_port ?? 22}</span>
        </div>
        <div class="remote-node-card__meta">
          <span>${node.remote_workspace}</span>
          <span>${node.control_mode || "orchestrated"} · ${node.agent_id} · ${node.advertise_host}:${node.agent_port}</span>
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
        () => ({}),
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
  meshClustersHost?.addEventListener("click", async (event) => {
    const target = event.target.closest("[data-remote-cluster-action]");
    if (!target) return;
    const clusterId = target.dataset.remoteClusterId;
    if (target.dataset.remoteClusterAction === "focus-cluster") {
      if (modeFilterSelect) modeFilterSelect.value = "offline_mesh";
      if (groupSelect) groupSelect.value = "orchestrator";
      if (searchInput) searchInput.value = clusterId;
      renderRemoteNodeCards(lastNodes);
      showCompletion(`Focused cluster ${clusterId}.`);
      return;
    }
    if (target.dataset.remoteClusterAction === "start-missing") {
      const pendingNodes = pendingClusterStartNodes(clusterId);
      if (pendingNodes.length === 0) {
        showCompletion(`Cluster ${clusterId} has no pending agent starts.`);
        return;
      }
      for (const node of pendingNodes) {
        const nodeIndex = lastNodes.indexOf(node);
        await executeNodeAction(nodeIndex, "start");
      }
      showCompletion(`Started ${pendingNodes.length} pending agent(s) in ${clusterId}.`);
      return;
    }
    if (target.dataset.remoteClusterAction !== "preflight-missing") return;
    const pendingNodes = pendingClusterMeshNodes(clusterId);
    if (pendingNodes.length === 0) {
      showCompletion(`Cluster ${clusterId} is already fully preflighted.`);
      return;
    }
    for (const node of pendingNodes) {
      const nodeIndex = lastNodes.indexOf(node);
      await executeNodeAction(nodeIndex, "mesh-preflight");
    }
    showCompletion(`Completed mesh preflight for ${pendingNodes.length} pending node(s) in ${clusterId}.`);
  });
  searchInput?.addEventListener("input", () => renderRemoteNodeCards(lastNodes));
  filterSelect?.addEventListener("change", () => renderRemoteNodeCards(lastNodes));
  modeFilterSelect?.addEventListener("change", () => renderRemoteNodeCards(lastNodes));
  groupSelect?.addEventListener("change", () => renderRemoteNodeCards(lastNodes));

  return { renderRemoteNodeCards };
}
