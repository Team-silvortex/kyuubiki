import { createRemoteNodeActionCoordinator } from "./remote-node-actions.js";
import { createRemoteNodeBulkActionCoordinator } from "./remote-node-bulk-actions.js";
import { createRemoteNodeCertificateController } from "./remote-node-certificates.js";
import { createRemoteNodeExecutor } from "./remote-node-executor.js";
import { createRemoteNodeMeshController } from "./remote-node-mesh.js";
import { groupRemoteNodes, renderRemoteNodeGroups } from "./remote-node-renderer.js";
import { createRemoteNodeTimelineController } from "./remote-node-timeline.js";

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
  const meshRolloutFailuresHost = document.getElementById("remote-mesh-rollout-failures");
  const meshClustersHost = document.getElementById("remote-mesh-clusters");
  const timelineHost = document.getElementById("remote-node-timeline");
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
  const stamp = () => Date.now();
  const workflowSnapshotFor = (node, actionKind, status, summary) => ({
    workflow_kind: actionKind.startsWith("mesh") ? "mesh_rollout_stage" : "remote_node_action",
    stage: actionKind,
    status,
    summary,
    recorded_at_unix_ms: stamp(),
    details: {
      agent_id: node.agent_id || "",
      target_host: node.target_host || "",
      control_mode: node.control_mode || "orchestrated",
      cluster_id: node.cluster_id || "",
    },
  });
  const persistNodeState = (index, patch) =>
    syncRegistry((nodes) =>
      nodes.map((entry, idx) => (idx === index ? withRemoteNodeStatus(entry, patch) : entry)),
    );

  function renderRemoteNodeCards(nodes) {
    lastNodes = Array.isArray(nodes) ? nodes : [];
    host.innerHTML = "";
    meshController.renderMeshDiagnostics(lastNodes);
    certificateController.renderCertificateHealth(lastNodes);
    timelineController.renderTimeline();
    const filteredNodes = visibleNodes(lastNodes);
    if (lastNodes.length === 0) {
      host.innerHTML = `<article class="remote-node-card"><strong>No registered nodes</strong><span>Save JSON nodes above to get one-click remote actions here.</span></article>`;
      return;
    }
    if (filteredNodes.length === 0) {
      host.innerHTML = `<article class="remote-node-card"><strong>No matching nodes</strong><span>Adjust the search text or status filter.</span></article>`;
      return;
    }

    renderRemoteNodeGroups({
      host,
      groups: groupRemoteNodes(filteredNodes, groupSelect?.value || "none", statusOf),
      lastNodes,
      statusOf,
      summary,
      timeLabel,
      certificateStatusFor: certificateController.certificateStatusFor,
    });
  }

  const executor = createRemoteNodeExecutor({
    applyRemoteNodeToForm,
    currentRemoteAgentPayload,
    currentRemoteBootstrapPayload,
    invoke,
    invokeGuardedMutation,
    persistNodeState,
    runAction,
    stamp,
    workflowSnapshotFor,
  });

  const meshController = createRemoteNodeMeshController({
    meshHealthHost,
    meshIssuesHost,
    meshRolloutFailuresHost,
    meshClustersHost,
    getLastNodes: () => lastNodes,
    modeFilterSelect,
    groupSelect,
    searchInput,
    rerender: () => renderRemoteNodeCards(lastNodes),
    showCompletion,
    executeNodeAction: executor.executeNodeAction,
  });
  const actionCoordinator = createRemoteNodeActionCoordinator({
    applyRemoteNodeToForm,
    executeNodeAction: executor.executeNodeAction,
    getLastNodes: () => lastNodes,
    resolveCertificateForNodeIndex: certificateController.resolveCertificateForNodeIndex,
    showCompletion,
  });
  const bulkActionCoordinator = createRemoteNodeBulkActionCoordinator({
    assignCertificatesForVisibleNodes: certificateController.assignCertificatesForVisibleNodes,
    clearCertificatesForVisibleNodes: certificateController.clearCertificatesForVisibleNodes,
    executeNodeAction: executor.executeNodeAction,
    getLastNodes: () => lastNodes,
    getVisibleNodes: visibleNodes,
    renderMeshRolloutFailures: meshController.renderRolloutFailures,
    runFocusedCertificateAction: certificateController.runFocusedCertificateAction,
    runVisibleMeshRollout: meshController.runVisibleMeshRollout,
    showCompletion,
  });
  const timelineController = createRemoteNodeTimelineController({
    host: timelineHost,
    getLastNodes: () => lastNodes,
    runRecommendedAction: actionCoordinator.runRecommendedAction,
  });

  host.addEventListener("click", async (event) => {
    const card = event.target.closest("[data-remote-node-card-index]");
    if (card?.dataset.remoteNodeCardIndex) {
      timelineController.selectNode(Number(card.dataset.remoteNodeCardIndex));
    }
    const target = event.target.closest("[data-remote-node-action]");
    if (!target) return;

    const registry = await invoke("remote_node_registry");
    const node = registry.nodes?.[Number(target.dataset.remoteNodeIndex)];
    if (!node) throw new Error("remote node no longer exists");
    applyRemoteNodeToForm(node);
    const nodeIndex = Number(target.dataset.remoteNodeIndex);

    switch (target.dataset.remoteNodeAction) {
      case "use":
        timelineController.selectNode(nodeIndex);
        showCompletion(`Loaded remote node ${node.label || node.target_host}.`);
        break;
      case "probe":
        await executor.executeNodeAction(nodeIndex, "probe");
        showCompletion(`Probed remote node ${node.label || node.target_host}.`);
        break;
      case "bootstrap":
        await executor.executeNodeAction(nodeIndex, "bootstrap");
        showCompletion(`Bootstrapped remote node ${node.label || node.target_host}.`);
        break;
      case "start":
        await executor.executeNodeAction(nodeIndex, "start");
        showCompletion(`Started agent on ${node.label || node.target_host}.`);
        break;
      default:
        break;
    }
  });
  document.querySelectorAll("[data-remote-bulk-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      await bulkActionCoordinator.runBulkAction(button.dataset.remoteBulkAction);
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
