function normalizeLabel(value) {
  return String(value || "")
    .replace(/[_-]+/g, " ")
    .trim();
}

function summarizeAgents(summary) {
  const active = Number(summary?.active_agent_count || 0);
  const total = Number(summary?.agent_count || 0);
  return `${active}/${total} running`;
}

function summarizeServices(summary) {
  const orchestrator = summary?.orchestrator_status || "unknown";
  const frontend = summary?.frontend_status || "unknown";
  return `orchestrator ${orchestrator}, frontend ${frontend}`;
}

function stateClassForStatus(value) {
  const normalized = String(value || "").trim().toLowerCase();
  if (normalized === "running") return "desktop-shell-state--healthy";
  if (normalized === "stopped") return "desktop-shell-state--danger";
  return "desktop-shell-state--idle";
}

function deploymentCardClass(value) {
  const normalized = String(value || "").trim().toLowerCase();
  if (normalized === "cloud") return "desktop-runtime-plane__card--deployment-cloud";
  if (normalized === "distributed") return "desktop-runtime-plane__card--deployment-distributed";
  return "desktop-runtime-plane__card--deployment-local";
}

function authorityCardClass(value) {
  const normalized = String(value || "").trim().toLowerCase();
  if (normalized === "single_orchestrator") return "desktop-runtime-plane__card--authority-single";
  if (normalized === "offline_mesh") return "desktop-runtime-plane__card--authority-mesh";
  return "desktop-runtime-plane__card--authority-self";
}

function deploymentBadge(value) {
  const normalized = String(value || "").trim().toLowerCase();
  if (normalized === "distributed") return "DISTR";
  if (normalized === "cloud") return "CLOUD";
  return "LOCAL";
}

function authorityBadge(value) {
  const normalized = String(value || "").trim().toLowerCase();
  if (normalized === "single_orchestrator") return "ORCH";
  if (normalized === "offline_mesh") return "MESH";
  return "SOLO";
}

export function formatRuntimeStatusReport({ title, rendered, summary }) {
  const heading = String(title || "Runtime status").trim();
  const body = String(rendered || "").trim();
  if (!summary) {
    return body ? `${heading}\n\n${body}` : heading;
  }
  return body ? `${heading}\n\n${body}` : heading;
}

export function renderRuntimeStatusPlane(container, summary) {
  if (!container) return;
  if (!summary) {
    container.hidden = true;
    container.innerHTML = "";
    return;
  }
  container.hidden = false;

  const cards = [
    {
      label: "Deployment",
      value: normalizeLabel(summary.deployment_mode) || "local",
      note: normalizeLabel(summary.control_mode) || "standalone",
      badge: deploymentBadge(summary.deployment_mode),
      cardClass: deploymentCardClass(summary.deployment_mode),
    },
    {
      label: "Authority",
      value: normalizeLabel(summary.authority_mode) || "self directed",
      note: "contract mode",
      badge: authorityBadge(summary.authority_mode),
      cardClass: authorityCardClass(summary.authority_mode),
    },
    {
      label: "Agents",
      value: `${Number(summary.active_agent_count || 0)}/${Number(summary.agent_count || 0)}`,
      note: "running endpoints",
    },
    {
      label: "Orchestrator",
      value: summary.orchestrator_status || "unknown",
      note: "control plane",
      stateClass: stateClassForStatus(summary.orchestrator_status),
    },
    {
      label: "Frontend",
      value: summary.frontend_status || "unknown",
      note: "workbench shell",
      stateClass: stateClassForStatus(summary.frontend_status),
    },
  ];

  container.innerHTML = cards
    .map(
      (card) => `
        <section class="desktop-runtime-plane__card">
        <section class="desktop-runtime-plane__card ${card.cardClass || ""}">
          <span class="desktop-runtime-plane__label">${card.label}</span>
          <strong class="desktop-runtime-plane__value ${card.stateClass || ""}">${card.value}</strong>
          <span class="desktop-runtime-plane__note">${card.note}</span>
          ${card.badge ? `<span class="desktop-runtime-plane__badge">${card.badge}</span>` : ""}
        </section>
      `,
    )
    .join("");
}
