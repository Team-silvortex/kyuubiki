function formatTime(unixMs) {
  return typeof unixMs === "number" && unixMs > 0 ? new Date(unixMs).toLocaleString() : "n/a";
}

function timelineEntriesFor(node) {
  return Array.isArray(node?.workflow_snapshots) ? [...node.workflow_snapshots].slice().reverse() : [];
}

function nodeIdentity(node) {
  return [node?.agent_id || "", node?.target_host || "", node?.cluster_id || ""].join("::");
}

function titleCase(value) {
  return value
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function stringifyDetailValue(value) {
  return typeof value === "string" ? value : JSON.stringify(value);
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;")
    .replaceAll("'", "&#39;");
}

function escapeAttribute(value) {
  return escapeHtml(value);
}

function workflowKindMeta(kind) {
  if (kind === "mesh_rollout_stage") {
    return { label: "Mesh rollout", tone: "mesh" };
  }
  return { label: "Remote action", tone: "remote" };
}

function stageMeta(stage) {
  const normalized = stage || "unknown";
  if (normalized.includes("preflight")) return { label: "Preflight", tone: "preflight" };
  if (normalized.includes("bootstrap")) return { label: "Bootstrap", tone: "bootstrap" };
  if (normalized.includes("start")) return { label: "Start agent", tone: "start" };
  if (normalized.includes("probe")) return { label: "Probe", tone: "probe" };
  return { label: titleCase(normalized), tone: "neutral" };
}

function statusMeta(status) {
  if (status === "succeeded") return { label: "Healthy", tone: "healthy" };
  if (status === "failed") return { label: "Failed", tone: "failed" };
  return { label: titleCase(status || "unknown"), tone: "neutral" };
}

function diagnosisMeta(entry) {
  if (entry.status === "failed") {
    return { label: "Recovery needed", tone: "failed" };
  }
  if ((entry.stage || "").includes("preflight")) {
    return { label: "Ready for rollout", tone: "healthy" };
  }
  if ((entry.stage || "").includes("start")) {
    return { label: "Runtime transition", tone: "active" };
  }
  return { label: "Observed transition", tone: "neutral" };
}

function renderSemanticBadges(entry) {
  const kind = workflowKindMeta(entry.workflow_kind);
  const stage = stageMeta(entry.stage);
  const status = statusMeta(entry.status);
  const diagnosis = diagnosisMeta(entry);
  return `
    <div class="remote-node-timeline__badges">
      <span class="remote-node-timeline__badge" data-tone="${escapeAttribute(kind.tone)}">${escapeHtml(kind.label)}</span>
      <span class="remote-node-timeline__badge" data-tone="${escapeAttribute(stage.tone)}">${escapeHtml(stage.label)}</span>
      <span class="remote-node-timeline__badge" data-tone="${escapeAttribute(status.tone)}">${escapeHtml(status.label)}</span>
      <span class="remote-node-timeline__badge" data-tone="${escapeAttribute(diagnosis.tone)}">${escapeHtml(diagnosis.label)}</span>
    </div>
  `;
}

function detailValue(details, key, fallback = "n/a") {
  const value = details[key];
  return escapeHtml(value === undefined || value === null || value === "" ? fallback : stringifyDetailValue(value));
}

function renderSummarySlots(details) {
  return `
    <div class="remote-node-timeline__summary-grid">
      <div class="remote-node-timeline__summary-slot">
        <span>Target host</span>
        <code>${detailValue(details, "target_host")}</code>
      </div>
      <div class="remote-node-timeline__summary-slot">
        <span>Agent id</span>
        <code>${detailValue(details, "agent_id")}</code>
      </div>
      <div class="remote-node-timeline__summary-slot">
        <span>Control mode</span>
        <code>${detailValue(details, "control_mode", "orchestrated")}</code>
      </div>
      <div class="remote-node-timeline__summary-slot">
        <span>Cluster id</span>
        <code>${detailValue(details, "cluster_id")}</code>
      </div>
    </div>
  `;
}

function renderAdditionalDetails(details) {
  const reservedKeys = new Set(["target_host", "agent_id", "control_mode", "cluster_id"]);
  const entries = Object.entries(details).filter(([key]) => !reservedKeys.has(key));
  if (entries.length === 0) {
    return `<span class="remote-node-timeline__detail-empty">No additional detail fields recorded.</span>`;
  }
  return entries.map(([key, value]) => `
      <div class="remote-node-timeline__detail-row">
        <span>${escapeHtml(key)}</span>
        <code>${escapeHtml(stringifyDetailValue(value))}</code>
      </div>
    `).join("");
}

function recommendedActions(entry, details) {
  const actions = [];
  const summary = (entry.summary || "").toLowerCase();
  const stage = entry.stage || "";
  const controlMode = details.control_mode || "";
  const hasCluster = Boolean(details.cluster_id);

  if (entry.status === "failed") {
    actions.push({ kind: "probe", label: "Probe again", detail: "Verify whether the node is still reachable." });
  }
  if (stage.includes("preflight")) {
    actions.push({ kind: "mesh-preflight", label: "Retry mesh preflight", detail: "Review peer endpoints and cluster id before retrying." });
    if (!hasCluster) {
      actions.push({ kind: "focus-cluster", label: "Set cluster id", detail: "Set a mesh cluster id for this node before continuing the rollout." });
    }
  }
  if (stage.includes("bootstrap")) {
    actions.push({ kind: "bootstrap", label: "Re-run bootstrap", detail: "Confirm the remote workspace path and SSH target first." });
  }
  if (stage.includes("start")) {
    actions.push({ kind: "start", label: "Start agent again", detail: "Check agent identity, advertise host, and agent port first." });
  }
  if (summary.includes("certificate") || summary.includes("cert")) {
    actions.push({ kind: "resolve-certificate", label: "Resolve certificate state", detail: "Align or re-issue the node certificate from inventory." });
    actions.push({ kind: "focus-certificate", label: "Open certificate binding", detail: "Jump to the certificate binding and inventory controls for this node." });
  }
  if (summary.includes("peer") || summary.includes("cluster")) {
    actions.push({ kind: "focus-peer-endpoints", label: "Inspect mesh fields", detail: "Check peer endpoints for duplicates, empty values, or unreachable hosts." });
  }
  if (controlMode === "offline_mesh" && entry.status === "succeeded" && stage.includes("preflight")) {
    actions.push({ kind: "start", label: "Start agent", detail: "This node looks ready for mesh rollout and agent start." });
  }
  return actions.filter((action, index, list) => list.findIndex((entryItem) => entryItem.kind === action.kind && entryItem.label === action.label) === index);
}

function renderRecommendations(entry, details, selectedNodeIndex) {
  const actions = recommendedActions(entry, details);
  if (actions.length === 0) {
    return `
      <div class="remote-node-timeline__recommendations remote-node-timeline__recommendations--empty">
        <span class="remote-node-timeline__detail-section-title">Recommended actions</span>
        <span class="remote-node-timeline__detail-empty">No follow-up actions suggested for this snapshot.</span>
      </div>
    `;
  }
  return `
    <div class="remote-node-timeline__recommendations">
      <span class="remote-node-timeline__detail-section-title">Recommended actions</span>
      <div class="remote-node-timeline__recommendation-list">
        ${actions.map((action) => `
          <div class="remote-node-timeline__recommendation-item">
            <span>Next</span>
            <strong>${escapeHtml(action.detail)}</strong>
            <button
              class="remote-node-timeline__recommendation-action"
              data-recommended-action="${escapeAttribute(action.kind)}"
              data-recommended-node-index="${selectedNodeIndex}"
            >${escapeHtml(action.label)}</button>
          </div>
        `).join("")}
      </div>
    </div>
  `;
}

function renderDetails(entry, selectedNodeIndex) {
  if (!entry) {
    return `<article class="remote-node-timeline__details remote-node-timeline__details--empty"><strong>Snapshot details</strong><span>Select a workflow snapshot to inspect its structured contract fields.</span></article>`;
  }
  const details = entry.details && typeof entry.details === "object" ? entry.details : {};
  return `
    <article class="remote-node-timeline__details">
      <strong>${escapeHtml(workflowKindMeta(entry.workflow_kind).label)} · ${escapeHtml(stageMeta(entry.stage).label)}</strong>
      <span>${escapeHtml(entry.status || "unknown")} · ${formatTime(entry.recorded_at_unix_ms)}</span>
      <span>${escapeHtml(entry.summary || "no summary")}</span>
      ${renderSemanticBadges(entry)}
      ${renderSummarySlots(details)}
      ${renderRecommendations(entry, details, selectedNodeIndex)}
      <div class="remote-node-timeline__detail-section">
        <span class="remote-node-timeline__detail-section-title">Additional fields</span>
        <div class="remote-node-timeline__detail-grid">${renderAdditionalDetails(details)}</div>
      </div>
    </article>
  `;
}

export function createRemoteNodeTimelineController({ host, getLastNodes, runRecommendedAction }) {
  let selectedNodeIndex = null;
  let selectedEntryIndex = 0;
  let selectedNodeIdentity = "";
  let lastSeenLatestSnapshotKey = "";

  const latestSnapshotKeyFor = (node) => {
    const latest = timelineEntriesFor(node)[0];
    return latest ? `${latest.stage || "unknown"}::${latest.recorded_at_unix_ms || 0}::${latest.status || "unknown"}` : "";
  };

  const renderTimeline = () => {
    if (!host) return;
    const nodes = getLastNodes();
    const node = typeof selectedNodeIndex === "number" ? nodes[selectedNodeIndex] : null;
    if (!node) {
      host.innerHTML = `<article class="remote-node-timeline__empty"><strong>Workflow timeline</strong><span>Select a remote node card to inspect recent workflow snapshots.</span></article>`;
      return;
    }
    const entries = timelineEntriesFor(node);
    const latestSnapshotKey = latestSnapshotKeyFor(node);
    const shouldFollowLatest = latestSnapshotKey && latestSnapshotKey !== lastSeenLatestSnapshotKey;
    if (shouldFollowLatest) {
      selectedEntryIndex = 0;
      lastSeenLatestSnapshotKey = latestSnapshotKey;
    }
    const activeEntry = entries[selectedEntryIndex] || entries[0] || null;
    host.innerHTML = entries.length === 0
      ? `<article class="remote-node-timeline__empty"><strong>${escapeHtml(node.label || node.target_host)}</strong><span>No workflow snapshots recorded yet.</span></article>`
      : `
        <article class="remote-node-timeline__header">
          <strong>${escapeHtml(node.label || node.target_host)}</strong>
          <span>${escapeHtml(node.agent_id || "agent unset")} · ${escapeHtml(node.control_mode || "orchestrated")}</span>
          <span>${entries.length} snapshot(s) · latest ${formatTime(entries[0]?.recorded_at_unix_ms)}</span>
          <span class="remote-node-timeline__follow-state" data-fresh="${shouldFollowLatest ? "true" : "false"}">
            ${shouldFollowLatest ? "Auto-focused latest snapshot" : "Following selected node context"}
          </span>
        </article>
        <div class="remote-node-timeline__body">
          <div class="remote-node-timeline__list">
            ${entries.map((entry, index) => `
              <article
                class="remote-node-timeline__item"
                data-status="${escapeAttribute(entry.status || "unknown")}"
                data-stage-tone="${escapeAttribute(stageMeta(entry.stage).tone)}"
                data-timeline-entry-index="${index}"
                data-selected="${index === selectedEntryIndex ? "true" : "false"}"
                data-latest="${index === 0 ? "true" : "false"}"
              >
                <strong>${escapeHtml(workflowKindMeta(entry.workflow_kind).label)} · ${escapeHtml(stageMeta(entry.stage).label)}</strong>
                <span>${escapeHtml(statusMeta(entry.status).label)} · ${formatTime(entry.recorded_at_unix_ms)}</span>
                ${renderSemanticBadges(entry)}
                <span>${escapeHtml(entry.summary || "no summary")}</span>
              </article>
            `).join("")}
          </div>
          ${renderDetails(activeEntry, selectedNodeIndex)}
        </div>
      `;
  };

  const selectNode = (nodeIndex) => {
    selectedNodeIndex = typeof nodeIndex === "number" ? nodeIndex : null;
    selectedEntryIndex = 0;
    const nodes = getLastNodes();
    const node = typeof selectedNodeIndex === "number" ? nodes[selectedNodeIndex] : null;
    selectedNodeIdentity = nodeIdentity(node);
    lastSeenLatestSnapshotKey = node ? latestSnapshotKeyFor(node) : "";
    renderTimeline();
  };

  const selectEntry = (entryIndex) => {
    selectedEntryIndex = typeof entryIndex === "number" && entryIndex >= 0 ? entryIndex : 0;
    renderTimeline();
  };

  const keepNodeContext = (nodeIndex) => {
    const nodes = getLastNodes();
    const candidate = typeof nodeIndex === "number" ? nodes[nodeIndex] : null;
    if (!candidate) return;
    const candidateIdentity = nodeIdentity(candidate);
    if (candidateIdentity && candidateIdentity === selectedNodeIdentity) {
      selectedNodeIndex = nodeIndex;
      return;
    }
    selectNode(nodeIndex);
  };

  host?.addEventListener("click", (event) => {
    const action = event.target.closest("[data-recommended-action]");
    if (action?.dataset.recommendedAction && action.dataset.recommendedNodeIndex) {
      keepNodeContext(Number(action.dataset.recommendedNodeIndex));
      runRecommendedAction?.(action.dataset.recommendedAction, Number(action.dataset.recommendedNodeIndex));
      return;
    }
    const entry = event.target.closest("[data-timeline-entry-index]");
    if (!entry?.dataset.timelineEntryIndex) return;
    selectEntry(Number(entry.dataset.timelineEntryIndex));
  });

  return {
    keepNodeContext,
    renderTimeline,
    selectEntry,
    selectNode,
  };
}
