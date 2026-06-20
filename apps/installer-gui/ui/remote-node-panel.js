export function mountRemoteNodePanel({
  invoke,
  runAction,
  invokeGuardedMutation,
  applyRemoteNodeToForm,
  showCompletion,
  currentRemoteBootstrapPayload,
  currentRemoteAgentPayload,
}) {
  const host = document.getElementById("remote-node-cards");

  function renderRemoteNodeCards(nodes) {
    host.innerHTML = "";
    if (!Array.isArray(nodes) || nodes.length === 0) {
      host.innerHTML = `<article class="remote-node-card"><strong>No registered nodes</strong><span>Save JSON nodes above to get one-click remote actions here.</span></article>`;
      return;
    }

    nodes.forEach((node, index) => {
      const card = document.createElement("article");
      card.className = "remote-node-card";
      card.innerHTML = `
        <div class="remote-node-card__header">
          <strong>${node.label || node.target_host}</strong>
          <span>${node.ssh_user}@${node.target_host}:${node.ssh_port ?? 22}</span>
        </div>
        <div class="remote-node-card__meta">
          <span>${node.remote_workspace}</span>
          <span>${node.agent_id} · ${node.advertise_host}:${node.agent_port}</span>
        </div>
        <div class="action-row">
          <button data-remote-node-action="use" data-remote-node-index="${index}">Use</button>
          <button data-remote-node-action="probe" data-remote-node-index="${index}">Probe</button>
          <button data-remote-node-action="bootstrap" data-remote-node-index="${index}">Bootstrap</button>
          <button class="primary" data-remote-node-action="start" data-remote-node-index="${index}">Start agent</button>
        </div>
      `;
      host.appendChild(card);
    });
  }

  host.addEventListener("click", async (event) => {
    const target = event.target.closest("[data-remote-node-action]");
    if (!target) return;

    const registry = await invoke("remote_node_registry");
    const node = registry.nodes?.[Number(target.dataset.remoteNodeIndex)];
    if (!node) throw new Error("remote node no longer exists");
    applyRemoteNodeToForm(node);

    switch (target.dataset.remoteNodeAction) {
      case "use":
        showCompletion(`Loaded remote node ${node.label || node.target_host}.`);
        break;
      case "probe":
        await runAction("probe-remote-node-card", () =>
          invokeGuardedMutation("probe_remote_node", {
            remoteBootstrap: currentRemoteBootstrapPayload(),
          }),
        );
        showCompletion(`Probed remote node ${node.label || node.target_host}.`);
        break;
      case "bootstrap":
        await runAction("remote-bootstrap-card", () =>
          invokeGuardedMutation("remote_bootstrap", {
            remoteBootstrap: currentRemoteBootstrapPayload(),
          }),
        );
        showCompletion(`Bootstrapped remote node ${node.label || node.target_host}.`);
        break;
      case "start":
        await runAction("remote-start-agent-card", () =>
          invokeGuardedMutation("remote_start_agent", {
            remoteAgent: currentRemoteAgentPayload(),
          }),
        );
        showCompletion(`Started agent on ${node.label || node.target_host}.`);
        break;
      default:
        break;
    }
  });

  return { renderRemoteNodeCards };
}
