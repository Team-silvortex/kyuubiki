export function createRemoteNodeMeshController({
  meshHealthHost,
  meshIssuesHost,
  meshClustersHost,
  getLastNodes,
  modeFilterSelect,
  groupSelect,
  searchInput,
  rerender,
  showCompletion,
  executeNodeAction,
}) {
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
      if ((node.last_action === "mesh_preflight_failed" || node.last_probe_summary?.includes("mesh preflight")) && node.last_probe_status === "failed") {
        issues.push(`${node.label || node.target_host}: ${node.last_probe_summary}`);
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
    getLastNodes().filter((node) =>
      (node.control_mode || "orchestrated") === "offline_mesh" &&
      node.cluster_id === clusterId &&
      !node.last_probe_summary?.includes("mesh preflight ok"),
    );

  const pendingClusterStartNodes = (clusterId) =>
    getLastNodes().filter((node) =>
      (node.control_mode || "orchestrated") === "offline_mesh" &&
      node.cluster_id === clusterId &&
      node.last_action !== "start_agent",
    );

  const bindEvents = () => {
    meshClustersHost?.addEventListener("click", async (event) => {
      const target = event.target.closest("[data-remote-cluster-action]");
      if (!target) return;
      const clusterId = target.dataset.remoteClusterId;
      if (target.dataset.remoteClusterAction === "focus-cluster") {
        if (modeFilterSelect) modeFilterSelect.value = "offline_mesh";
        if (groupSelect) groupSelect.value = "orchestrator";
        if (searchInput) searchInput.value = clusterId;
        rerender();
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
          const nodeIndex = getLastNodes().indexOf(node);
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
        const nodeIndex = getLastNodes().indexOf(node);
        await executeNodeAction(nodeIndex, "mesh-preflight");
      }
      showCompletion(`Completed mesh preflight for ${pendingNodes.length} pending node(s) in ${clusterId}.`);
    });
  };

  return {
    bindEvents,
    renderMeshDiagnostics,
  };
}
