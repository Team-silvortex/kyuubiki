import { createRemoteNodeCertificateController } from "./remote-node-certificates.js";
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
  const focusField = (fieldId) => {
    const field = document.getElementById(fieldId);
    if (!field) return false;
    field.scrollIntoView({ block: "center", behavior: "smooth" });
    field.focus?.();
    if (typeof field.select === "function" && ("value" in field)) field.select();
    return true;
  };
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
  const runNodeAction = async (name, node, index, actionKind, runner, onSuccess) => {
    try {
      const result = await runAction(name, runner);
      const successPatch = onSuccess(result);
      await persistNodeState(index, {
        ...successPatch,
        last_action: actionKind,
        last_action_unix_ms: stamp(),
        workflowSnapshot: workflowSnapshotFor(
          node,
          actionKind,
          "succeeded",
          successPatch.last_probe_summary || (typeof result === "string" ? result : `${actionKind} succeeded`),
        ),
      });
      return result;
    } catch (error) {
      await persistNodeState(index, {
        last_probe_status: actionKind === "probe" ? "failed" : "failed",
        last_probe_summary: error.message || String(error),
        last_probe_unix_ms: stamp(),
        last_action: `${actionKind}_failed`,
        last_action_unix_ms: stamp(),
        workflowSnapshot: workflowSnapshotFor(node, actionKind, "failed", error.message || String(error)),
      });
      throw error;
    }
  };

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

  async function executeNodeAction(nodeIndex, actionKind) {
    const registry = await invoke("remote_node_registry");
    const node = registry.nodes?.[nodeIndex];
    if (!node) throw new Error("remote node no longer exists");
    applyRemoteNodeToForm(node);
    if (actionKind === "probe") {
      return runNodeAction("probe-remote-node-card", node, nodeIndex, "probe", () =>
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
      return runNodeAction("remote-bootstrap-card", node, nodeIndex, "bootstrap", () =>
        invokeGuardedMutation("remote_bootstrap", {
          remoteBootstrap: currentRemoteBootstrapPayload(),
        }),
        () => ({}),
      );
    }
    if (actionKind === "start") {
      return runNodeAction("remote-start-agent-card", node, nodeIndex, "start_agent", () =>
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
      return runNodeAction("remote-mesh-preflight-card", node, nodeIndex, "mesh_preflight", async () => {
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
    meshRolloutFailuresHost,
    meshClustersHost,
    getLastNodes: () => lastNodes,
    modeFilterSelect,
    groupSelect,
    searchInput,
    rerender: () => renderRemoteNodeCards(lastNodes),
    showCompletion,
    executeNodeAction,
  });
  const timelineController = createRemoteNodeTimelineController({
    host: timelineHost,
    getLastNodes: () => lastNodes,
    runRecommendedAction: async (actionKind, nodeIndex) => {
      const node = lastNodes[nodeIndex];
      if (!node) throw new Error("remote node no longer exists");
      applyRemoteNodeToForm(node);
      if (actionKind === "focus-cluster") {
        focusField("remote-cluster-id");
        showCompletion(`Focused mesh cluster id for ${node.label || node.target_host}.`);
        return;
      }
      if (actionKind === "focus-peer-endpoints") {
        focusField("remote-peer-endpoints");
        showCompletion(`Focused peer endpoints for ${node.label || node.target_host}.`);
        return;
      }
      if (actionKind === "focus-certificate") {
        const focused = focusField("remote-certificate-id") || focusField("certificate-revoke-id");
        if (focused) {
          showCompletion(`Focused certificate controls for ${node.label || node.target_host}.`);
        } else {
          showCompletion(`Loaded certificate controls for ${node.label || node.target_host}.`);
        }
        return;
      }
      if (actionKind === "resolve-certificate") {
        const result = await certificateController.resolveCertificateForNodeIndex(nodeIndex);
        if (result.outcome === "assigned" || result.outcome === "unchanged") {
          showCompletion(`Resolved certificate state for ${node.label || node.target_host} with ${result.certificateId}.`);
        } else if (result.outcome === "cleared") {
          showCompletion(`Cleared stale certificate binding for ${node.label || node.target_host}.`);
        } else if (result.outcome === "aligned") {
          showCompletion(`Certificate state already aligned for ${node.label || node.target_host}.`);
        } else if (result.outcome === "ambiguous") {
          showCompletion(`Multiple active certificates match ${node.label || node.target_host}; pick one from inventory.`);
        } else {
          showCompletion(`No active certificate match available for ${node.label || node.target_host}.`);
        }
        return;
      }
      if (actionKind === "inspect") {
        showCompletion(`Loaded ${node.label || node.target_host} into the form for manual review.`);
        return;
      }
      await executeNodeAction(nodeIndex, actionKind);
      const actionLabel = actionKind === "mesh-preflight"
        ? "mesh preflight"
        : actionKind === "start"
          ? "start agent"
          : actionKind;
      showCompletion(`Completed ${actionLabel} for ${node.label || node.target_host}.`);
    },
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
      if (actionKind === "mesh-rollout") {
        const result = await meshController.runVisibleMeshRollout(visibleNodes(lastNodes));
        const failureSummary = result.failures.length === 0
          ? "no failures"
          : `failures ${result.failures.length}: ${result.failures.map((entry) => `${entry.stage} ${entry.label}`).join(", ")}`;
        showCompletion(
          `Completed mesh rollout for ${result.total} visible node(s): bootstrap ${result.bootstrapped}, preflight ${result.preflighted}, start ${result.started}; ${failureSummary}.`,
        );
        return;
      }
      const visible = visibleNodes(lastNodes).filter((node) =>
        actionKind === "mesh-preflight" ? (node.control_mode || "orchestrated") === "offline_mesh" : true,
      );
      if (visible.length === 0) throw new Error("no visible remote nodes");
      if (actionKind === "mesh-preflight") meshController.renderRolloutFailures([]);
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
