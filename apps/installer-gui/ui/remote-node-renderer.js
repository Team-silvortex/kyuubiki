export function groupRemoteNodes(nodes, groupBy, statusOf) {
  const groups = new Map();
  nodes.forEach((node) => {
    let key = "__all__";
    switch (groupBy || "none") {
      case "status":
        key = statusOf(node);
        break;
      case "workspace":
        key = node.remote_workspace || "(workspace unset)";
        break;
      case "control_mode":
        key = node.control_mode || "orchestrated";
        break;
      case "orchestrator":
        key = node.control_mode === "offline_mesh"
          ? node.cluster_id || "(mesh cluster unset)"
          : node.orchestrator_url || "(orchestrator unset)";
        break;
      default:
        break;
    }
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(node);
  });
  return groups;
}

export function renderRemoteNodeGroups({
  host,
  groups,
  lastNodes,
  statusOf,
  summary,
  timeLabel,
  certificateStatusFor,
}) {
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
      const certificateStatus = certificateStatusFor(node);
      card.className = "remote-node-card";
      card.dataset.nodeStatus = statusOf(node);
      card.dataset.certificateStatus = certificateStatus.tone;
      card.dataset.remoteNodeCardIndex = String(index);
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
