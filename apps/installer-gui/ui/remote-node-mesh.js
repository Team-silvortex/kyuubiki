export function createRemoteNodeMeshController({
  meshHealthHost,
  meshIssuesHost,
  meshRolloutFailuresHost,
  meshClustersHost,
  getLastNodes,
  modeFilterSelect,
  groupSelect,
  searchInput,
  rerender,
  showCompletion,
  executeNodeAction,
}) {
  let lastRolloutFailures = [];
  const escapeHtml = (value) =>
    String(value ?? "")
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll("\"", "&quot;")
      .replaceAll("'", "&#39;");
  const meshNodesFrom = (nodes) =>
    nodes.filter((node) => (node.control_mode || "orchestrated") === "offline_mesh");

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
      : diagnostics.issues.map((issue) => `<article class="remote-mesh-issues__item"><strong>Mesh issue</strong><span>${escapeHtml(issue)}</span></article>`).join("");
    meshClustersHost.innerHTML = diagnostics.clusters.length === 0
      ? ""
      : diagnostics.clusters.map((cluster) => `
        <article class="remote-mesh-cluster" data-readiness="${cluster.readiness}">
          <strong>${escapeHtml(cluster.clusterId)}</strong>
          <span><strong>${cluster.finalReadiness === "go" ? "Ready for run" : "Hold"}</strong></span>
          <span>nodes ${cluster.nodeCount} · peers ${cluster.peerCount}</span>
          <span>preflight ok ${cluster.readyCount}/${cluster.nodeCount}</span>
          <span>agents started ${cluster.nodeCount - cluster.missingStartCount}/${cluster.nodeCount}</span>
          <span>${cluster.finalReadiness === "go" ? "preflight and agent startup aligned" : cluster.readiness === "ready" ? "start remaining agents before workload tests" : `needs ${cluster.missingPreflightCount} more verified node(s)`}</span>
          <div class="action-row">
            <button data-remote-cluster-action="focus-cluster" data-remote-cluster-id="${escapeHtml(cluster.clusterId)}">Focus cluster</button>
            ${cluster.missingStartCount === 0 ? "" : `<button data-remote-cluster-action="start-missing" data-remote-cluster-id="${escapeHtml(cluster.clusterId)}">Start missing agents</button>`}
            ${cluster.readiness === "ready" ? "" : `<button data-remote-cluster-action="preflight-missing" data-remote-cluster-id="${escapeHtml(cluster.clusterId)}">Preflight missing</button>`}
          </div>
        </article>
      `).join("");
  };

  const renderRolloutFailures = (failures) => {
    if (!meshRolloutFailuresHost) return;
    lastRolloutFailures = Array.isArray(failures) ? failures : [];
    meshRolloutFailuresHost.innerHTML = lastRolloutFailures.length === 0
      ? ""
      : `
        <div class="action-row">
          <button data-remote-mesh-failure-action="retry-failures">Retry failed nodes</button>
        </div>
        ${lastRolloutFailures.map((entry) => `
          <article class="remote-mesh-rollout-failures__item">
            <strong>${escapeHtml(entry.stage)} · ${escapeHtml(entry.label)}</strong>
            <span>${escapeHtml(entry.message)}</span>
          </article>
        `).join("")}
      `;
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

  const recordStageFailure = (failures, stage, node, error) => {
    failures.push({
      stage,
      label: node.label || node.target_host || "unknown node",
      message: error?.message || String(error),
      ref: {
        agentId: node.agent_id || "",
        clusterId: node.cluster_id || "",
        targetHost: node.target_host || "",
      },
    });
  };

  const findFailureNodeIndex = (failure) =>
    getLastNodes().findIndex((node) =>
      (node.agent_id || "") === (failure.ref?.agentId || "") &&
      (node.cluster_id || "") === (failure.ref?.clusterId || "") &&
      (node.target_host || "") === (failure.ref?.targetHost || ""),
    );

  const retryFailedNodes = async () => {
    if (lastRolloutFailures.length === 0) {
      showCompletion("No failed mesh rollout nodes to retry.");
      return { retried: 0, remaining: 0 };
    }
    const retryFailures = [];
    let retried = 0;
    for (const failure of lastRolloutFailures) {
      const nodeIndex = findFailureNodeIndex(failure);
      if (nodeIndex < 0) {
        retryFailures.push({ ...failure, message: "node no longer exists in registry" });
        continue;
      }
      try {
        await executeNodeAction(nodeIndex, failure.stage);
        retried += 1;
      } catch (error) {
        retryFailures.push({ ...failure, message: error?.message || String(error) });
      }
    }
    renderRolloutFailures(retryFailures);
    return { retried, remaining: retryFailures.length };
  };

  const runVisibleMeshRollout = async (nodes) => {
    const meshNodes = meshNodesFrom(nodes);
    if (meshNodes.length === 0) throw new Error("no visible offline_mesh nodes");
    let bootstrapped = 0;
    let preflighted = 0;
    let started = 0;
    const failures = [];
    for (const node of meshNodes) {
      const nodeIndex = getLastNodes().indexOf(node);
      try {
        await executeNodeAction(nodeIndex, "bootstrap");
        bootstrapped += 1;
      } catch (error) {
        recordStageFailure(failures, "bootstrap", node, error);
      }
    }
    for (const node of meshNodes) {
      const nodeIndex = getLastNodes().indexOf(node);
      try {
        await executeNodeAction(nodeIndex, "mesh-preflight");
        preflighted += 1;
      } catch (error) {
        recordStageFailure(failures, "mesh-preflight", node, error);
      }
    }
    for (const node of meshNodes) {
      const nodeIndex = getLastNodes().indexOf(node);
      try {
        await executeNodeAction(nodeIndex, "start");
        started += 1;
      } catch (error) {
        recordStageFailure(failures, "start", node, error);
      }
    }
    renderRolloutFailures(failures);
    return { bootstrapped, failures, preflighted, started, total: meshNodes.length };
  };

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
        renderRolloutFailures([]);
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
      renderRolloutFailures([]);
    });
    meshRolloutFailuresHost?.addEventListener("click", async (event) => {
      const target = event.target.closest("[data-remote-mesh-failure-action]");
      if (!target || target.dataset.remoteMeshFailureAction !== "retry-failures") return;
      const result = await retryFailedNodes();
      showCompletion(`Retried ${result.retried} failed node action(s); remaining failures ${result.remaining}.`);
    });
  };

  return {
    bindEvents,
    renderMeshDiagnostics,
    renderRolloutFailures,
    retryFailedNodes,
    runVisibleMeshRollout,
  };
}
