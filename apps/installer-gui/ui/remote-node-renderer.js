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
  const appendText = (parent, tagName, text, className = "") => {
    const element = document.createElement(tagName);
    if (className) element.className = className;
    element.textContent = String(text ?? "");
    parent.appendChild(element);
    return element;
  };

  const appendActionButton = (parent, action, label, index, className = "") => {
    const button = document.createElement("button");
    if (className) button.className = className;
    button.dataset.remoteNodeAction = action;
    button.dataset.remoteNodeIndex = String(index);
    button.textContent = label;
    parent.appendChild(button);
    return button;
  };

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

      const header = document.createElement("div");
      header.className = "remote-node-card__header";
      appendText(header, "strong", node.label || node.target_host);
      appendText(header, "span", `${node.ssh_user}@${node.target_host}:${node.ssh_port ?? 22}`);

      const meta = document.createElement("div");
      meta.className = "remote-node-card__meta";
      appendText(meta, "span", node.remote_workspace);
      appendText(meta, "span", `${node.control_mode || "orchestrated"} · ${node.agent_id} · ${node.advertise_host}:${node.agent_port}`);
      const certificate = document.createElement("span");
      certificate.className = "remote-node-card__certificate";
      const certificatePill = appendText(certificate, "span", certificateStatus.tone, "remote-node-card__certificate-pill");
      certificatePill.dataset.certificateTone = certificateStatus.tone;
      appendText(certificate, "span", certificateStatus.summary);
      meta.appendChild(certificate);
      appendText(meta, "span", `certificate ${certificateStatus.tone} · ${certificateStatus.detail}`);

      const status = document.createElement("div");
      status.className = "remote-node-card__status";
      appendText(status, "strong", statusOf(node).toUpperCase());
      appendText(status, "span", node.control_mode === "offline_mesh"
        ? `cluster ${node.cluster_id || "(unset)"} · peers ${(node.peer_endpoints || []).length}`
        : node.orchestrator_url || "orchestrator unset");
      appendText(status, "span", summary(node));
      appendText(status, "span", `Probe: ${timeLabel(node.last_probe_unix_ms)} · Action: ${timeLabel(node.last_action_unix_ms)}`);

      const actions = document.createElement("div");
      actions.className = "action-row";
      appendActionButton(actions, "use", "Use", index);
      appendActionButton(actions, "probe", "Probe", index);
      appendActionButton(actions, "bootstrap", "Bootstrap", index);
      appendActionButton(actions, "start", "Start agent", index, "primary");

      card.append(header, meta, status, actions);
      groupShell.appendChild(card);
    });
    host.appendChild(groupShell);
  });
}
