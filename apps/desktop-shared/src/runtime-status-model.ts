import type { DetailField, Filterable, MeshPanel, RuntimeEntry, RuntimeFilter, RuntimeSelection, RuntimeStatusModel, TopologySection, UnknownRecord } from "./runtime-status-types.js";

export type { RuntimeStatusModel } from "./runtime-status-types.js";

function asRecord(value: unknown): UnknownRecord | null {
  return value && typeof value === "object" ? (value as UnknownRecord) : null;
}

function asArray(value: unknown): UnknownRecord[] {
  return Array.isArray(value) ? (value as UnknownRecord[]) : [];
}

function sentenceCase(value: unknown): string {
  const text = String(value ?? "").trim().replaceAll(/[_-]+/gu, " ");
  if (!text) return "Unknown";
  return text.charAt(0).toUpperCase() + text.slice(1);
}

function normalizeMetric(value: unknown, fallback: unknown = "--"): string {
  if (value === undefined || value === null || value === "") return String(fallback);
  if (typeof value === "number") return Number.isFinite(value) ? String(value) : String(fallback);
  if (typeof value === "boolean") return value ? "Yes" : "No";
  return String(value);
}

function toSlug(value: unknown, fallback = "unknown"): string {
  const normalized = String(value ?? "")
    .trim()
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/gu, "-")
    .replaceAll(/^-+|-+$/gu, "");
  return normalized || fallback;
}

function uniqueValues(values: string[]): string[] {
  return Array.from(new Set(values.filter(Boolean)));
}

function summarizeAgentHealth(agent: UnknownRecord): string {
  const parts: string[] = [];
  if (typeof agent.health_score === "number") parts.push(`health ${agent.health_score}`);
  if (typeof agent.cooldown_remaining_ms === "number" && agent.cooldown_remaining_ms > 0) {
    parts.push(`cooldown ${agent.cooldown_remaining_ms} ms`);
  }
  if (typeof agent.consecutive_failures === "number" && agent.consecutive_failures > 0) {
    parts.push(`failures ${agent.consecutive_failures}`);
  }
  return parts.join(" · ");
}

function makeDetailEntry(label: string, value: unknown): DetailField | null {
  const normalized = normalizeMetric(value, "");
  return normalized ? { label, value: normalized } : null;
}

function compactDetails(entries: Array<DetailField | null>): DetailField[] {
  return entries.filter(Boolean) as DetailField[];
}

function buildLocalRuntimeEntries(summary: UnknownRecord | null): RuntimeEntry[] {
  if (!summary) return [];
  const entries: RuntimeEntry[] = [
    {
      id: "frontend",
      label: "Frontend",
      status: summary.frontend_status || "unknown",
      deployment: summary.deployment_mode || "local",
      type: "frontend",
      authority: summary.authority_mode || summary.control_mode || "",
      note: `GUI mode · ${normalizeMetric(summary.control_mode, "single shell")}`,
      badge: summary.deployment_mode || "local",
      filterTags: ["all", "local", "frontend"],
      detail: {
        title: "Frontend runtime",
        eyebrow: "Local shell",
        copy: "Desktop frontend shell state stays visible here so GUI/runtime coupling is explicit.",
        fields: compactDetails([
          makeDetailEntry("Status", summary.frontend_status || "unknown"),
          makeDetailEntry("Deployment", summary.deployment_mode || "local"),
          makeDetailEntry("Control", summary.control_mode || "single shell"),
          makeDetailEntry("Authority", summary.authority_mode || "unknown"),
        ]),
      },
    },
    {
      id: "orchestrator",
      label: "Orchestrator",
      status: summary.orchestrator_status || "unknown",
      deployment: summary.deployment_mode || "local",
      type: "orchestrator",
      authority: summary.authority_mode || summary.control_mode || "",
      note: `Authority · ${normalizeMetric(summary.authority_mode, "unknown")}`,
      badge: summary.control_mode || "orchestrated",
      filterTags: uniqueValues([
        "all",
        "local",
        "orchestrator",
        toSlug(summary.control_mode, ""),
        toSlug(summary.authority_mode, ""),
      ]),
      detail: {
        title: "Orchestrator runtime",
        eyebrow: "Authority",
        copy: "Control ownership and orchestration posture stay visible here before you leave the desktop shell.",
        fields: compactDetails([
          makeDetailEntry("Status", summary.orchestrator_status || "unknown"),
          makeDetailEntry("Control", summary.control_mode || "unknown"),
          makeDetailEntry("Authority", summary.authority_mode || "unknown"),
          makeDetailEntry("Active agents", summary.active_agent_count),
          makeDetailEntry("Agents", summary.agent_count),
        ]),
      },
    },
  ];
  const agents = asArray(summary.agents);
  for (const [index, agent] of agents.entries()) {
    entries.push({
      id: `local-agent-${index + 1}`,
      label: agent.label || `Agent ${index + 1}`,
      status: agent.status || "unknown",
      deployment: summary.deployment_mode || "local",
      type: "agent",
      authority: summary.authority_mode || summary.control_mode || "",
      note: "Local runtime summary",
      badge: "local",
      filterTags: uniqueValues([
        "all",
        "local",
        "agent",
        "local-agent",
        toSlug(summary.control_mode, ""),
        toSlug(summary.authority_mode, ""),
        toSlug(agent.status, ""),
      ]),
      detail: {
        title: agent.label || `Agent ${index + 1}`,
        eyebrow: "Local agent",
        copy: "This entry comes from the local desktop runtime summary.",
        fields: compactDetails([
          makeDetailEntry("Status", agent.status || "unknown"),
          makeDetailEntry("Deployment", summary.deployment_mode || "local"),
          makeDetailEntry("Control", summary.control_mode || "unknown"),
          makeDetailEntry("Authority", summary.authority_mode || "unknown"),
        ]),
      },
    });
  }
  return entries;
}

function buildMeshRuntimeEntries(healthPayload: UnknownRecord | null): RuntimeEntry[] {
  if (!healthPayload) return [];
  const solverAgents = asArray(healthPayload.solver_agents);
  return solverAgents.map((agent, index) => {
    const sessionState = agent.session_state || agent.control_mode || "unknown";
    const healthSummary = summarizeAgentHealth(agent);
    return {
      id: agent.id || `mesh-agent-${index + 1}`,
      label: agent.id || `${agent.host || "agent"}:${agent.port || "?"}`,
      status:
        typeof agent.cooldown_remaining_ms === "number" && agent.cooldown_remaining_ms > 0
          ? "degraded"
          : sessionState,
      deployment: healthPayload.deployment?.mode || "distributed",
      type: "agent",
      authority: agent.control_mode || "offline_mesh",
      note: [
        [agent.host, agent.port].filter(Boolean).join(":"),
        agent.orch_id ? `orch ${agent.orch_id}` : null,
        healthSummary || null,
      ]
        .filter(Boolean)
        .join(" · "),
      badge: agent.control_mode || "mesh",
      filterTags: uniqueValues([
        "all",
        "mesh",
        "agent",
        toSlug(agent.control_mode, ""),
        toSlug(sessionState, ""),
        typeof agent.cooldown_remaining_ms === "number" && agent.cooldown_remaining_ms > 0
          ? "degraded"
          : "",
      ]),
      detail: {
        title: agent.id || `Mesh agent ${index + 1}`,
        eyebrow: sentenceCase(agent.control_mode || "mesh"),
        copy: "Direct mesh runtime state, endpoint identity, and cooldown health all land here together.",
        fields: compactDetails([
          makeDetailEntry("Endpoint", [agent.host, agent.port].filter(Boolean).join(":")),
          makeDetailEntry("Session state", sessionState),
          makeDetailEntry("Control mode", agent.control_mode || "offline_mesh"),
          makeDetailEntry("Orchestrator", agent.orch_id),
          makeDetailEntry("Session", agent.orch_session_id),
          makeDetailEntry("Region", agent.region),
          makeDetailEntry("Zone", agent.zone),
          makeDetailEntry("Health", agent.health_score),
          makeDetailEntry("Cooldown ms", agent.cooldown_remaining_ms),
          makeDetailEntry("Failures", agent.consecutive_failures),
          makeDetailEntry("Last failure", agent.last_failure_reason),
        ]),
      },
    };
  });
}

function buildMeshPanels(
  summary: UnknownRecord | null,
  healthPayload: UnknownRecord | null,
): MeshPanel[] {
  if (!healthPayload) return [];
  const deployment = asRecord(healthPayload.deployment) || {};
  const registry = asRecord(healthPayload.remote_solver_registry) || {};
  const transport = asRecord(healthPayload.transport) || {};
  const solverAgents = asArray(healthPayload.solver_agents);
  const managedGroups = asArray(registry.mesh_topology?.managed_orchestrators);
  const offlineMesh = asRecord(registry.mesh_topology?.offline_mesh) || {};
  const authorityGroups = asArray(registry.authority_groups);
  return [
    {
      label: "Deployment",
      title: sentenceCase(deployment.mode || summary?.deployment_mode || "unknown"),
      pills: [
        { label: "Discovery", value: normalizeMetric(deployment.discovery, "n/a") },
        { label: "Endpoints", value: normalizeMetric(deployment.endpoint_count, 0) },
        { label: "Ready", value: normalizeMetric(deployment.ready_endpoint_count, 0) },
      ],
      items: [
        {
          title: "Cooling endpoints",
          meta: `${normalizeMetric(deployment.cooling_down_count, 0)} cooling`,
          copy: "Endpoints in cooldown stay visible so mesh routing remains auditable.",
          filterTags: ["all", "mesh", "degraded"],
        },
      ],
      filterTags: ["all", "mesh"],
      detail: {
        title: "Mesh deployment",
        eyebrow: "Deployment",
        copy: "Current discovery and readiness posture for the visible mesh route.",
        fields: compactDetails([
          makeDetailEntry("Mode", deployment.mode),
          makeDetailEntry("Discovery", deployment.discovery),
          makeDetailEntry("Endpoints", deployment.endpoint_count),
          makeDetailEntry("Ready", deployment.ready_endpoint_count),
          makeDetailEntry("Cooling", deployment.cooling_down_count),
          makeDetailEntry("Manifest", deployment.manifest_path),
        ]),
      },
    },
    {
      label: "Registry",
      title: `${normalizeMetric(registry.active_agents, 0)} active / ${normalizeMetric(
        registry.total_agents,
        0,
      )} total`,
      pills: [
        { label: "Managed", value: normalizeMetric(registry.control_modes?.orch_managed, 0) },
        { label: "Offline", value: normalizeMetric(registry.control_modes?.offline_mesh, 0) },
        { label: "Stale", value: normalizeMetric(registry.stale_agents, 0) },
      ],
      items: authorityGroups.slice(0, 4).map((group) => ({
        title: group.orch_id || group.control_mode || "authority group",
        meta: `${normalizeMetric(group.agent_count, 0)} agents`,
        copy: [
          `mode ${normalizeMetric(group.control_mode, "unknown")}`,
          group.orch_session_id ? `session ${group.orch_session_id}` : null,
          Array.isArray(group.agent_ids) && group.agent_ids.length ? group.agent_ids.join(", ") : null,
        ]
          .filter(Boolean)
          .join(" · "),
        filterTags: uniqueValues([
          "all",
          "mesh",
          toSlug(group.control_mode, ""),
          group.orch_id ? `orch:${group.orch_id}` : "",
        ]),
        detail: {
          title: group.orch_id || sentenceCase(group.control_mode || "authority"),
          eyebrow: "Authority group",
          copy: "A visible authority cluster grouped by control mode and bound orchestrator/session.",
          fields: compactDetails([
            makeDetailEntry("Mode", group.control_mode),
            makeDetailEntry("Orchestrator", group.orch_id),
            makeDetailEntry("Session", group.orch_session_id),
            makeDetailEntry("Agents", group.agent_count),
            makeDetailEntry("Agent ids", Array.isArray(group.agent_ids) ? group.agent_ids.join(", ") : ""),
          ]),
        },
      })),
      wide: authorityGroups.length > 2,
      filterTags: ["all", "mesh", "authority"],
      detail: {
        title: "Registry authority",
        eyebrow: "Registry",
        copy: "Registry grouping tells you who currently owns visible mesh execution paths.",
        fields: compactDetails([
          makeDetailEntry("Active agents", registry.active_agents),
          makeDetailEntry("Total agents", registry.total_agents),
          makeDetailEntry("Managed groups", registry.control_modes?.orch_managed),
          makeDetailEntry("Offline mesh", registry.control_modes?.offline_mesh),
          makeDetailEntry("Stale agents", registry.stale_agents),
        ]),
      },
    },
    {
      label: "Transport",
      title: `${normalizeMetric(transport.http, 4000)} / ${normalizeMetric(
        transport.solver_agent_tcp,
        5001,
      )}`,
      pills: [
        { label: "Managed orch", value: String(managedGroups.length) },
        { label: "Offline mesh", value: normalizeMetric(offlineMesh.agent_count, 0) },
        { label: "Agents", value: String(solverAgents.length) },
      ],
      items: solverAgents.slice(0, 4).map((agent) => ({
        title: agent.id || [agent.host, agent.port].filter(Boolean).join(":"),
        meta: agent.control_mode || "mesh",
        copy: [
          [agent.host, agent.port].filter(Boolean).join(":"),
          agent.region || agent.zone || null,
          summarizeAgentHealth(agent) || null,
        ]
          .filter(Boolean)
          .join(" · "),
        filterTags: uniqueValues([
          "all",
          "mesh",
          toSlug(agent.control_mode, ""),
          typeof agent.cooldown_remaining_ms === "number" && agent.cooldown_remaining_ms > 0
            ? "degraded"
            : "",
        ]),
        detail: {
          title: agent.id || "Transport agent",
          eyebrow: "Transport path",
          copy: "Live transport identity as seen by the orchestrator health endpoint.",
          fields: compactDetails([
            makeDetailEntry("Endpoint", [agent.host, agent.port].filter(Boolean).join(":")),
            makeDetailEntry("Mode", agent.control_mode),
            makeDetailEntry("Region", agent.region),
            makeDetailEntry("Zone", agent.zone),
            makeDetailEntry("Health", agent.health_score),
            makeDetailEntry("Cooldown ms", agent.cooldown_remaining_ms),
            makeDetailEntry("Failures", agent.consecutive_failures),
          ]),
        },
      })),
      wide: solverAgents.length > 2,
      filterTags: ["all", "mesh", "transport"],
      detail: {
        title: "Mesh transport",
        eyebrow: "Transport",
        copy: "HTTP control port and solver agent TCP path are shown together so route mismatches stay obvious.",
        fields: compactDetails([
          makeDetailEntry("HTTP", transport.http),
          makeDetailEntry("Solver TCP", transport.solver_agent_tcp),
          makeDetailEntry("Managed orch", managedGroups.length),
          makeDetailEntry("Offline mesh", offlineMesh.agent_count),
          makeDetailEntry("Visible agents", solverAgents.length),
        ]),
      },
    },
  ].filter((panel) => panel.items.length > 0 || panel.pills.length > 0);
}

function buildTopologySections(healthPayload: UnknownRecord | null): TopologySection[] {
  if (!healthPayload) return [];
  const registry = asRecord(healthPayload.remote_solver_registry) || {};
  const meshTopology = asRecord(registry.mesh_topology) || {};
  const managedGroups = asArray(meshTopology.managed_orchestrators);
  const authorityGroups = asArray(registry.authority_groups);
  const transitions = asArray(registry.recent_session_transitions);
  const offlineMesh = asRecord(meshTopology.offline_mesh) || {};
  return [
    {
      eyebrow: "Mesh topology",
      title: "Managed orchestrators",
      copy: "Each orchestrator keeps one visible group so agent authority stays explicit.",
      stats: [
        { label: "Groups", value: String(managedGroups.length) },
        { label: "Offline", value: normalizeMetric(offlineMesh.agent_count, 0) },
      ],
      items: managedGroups.map((group) => ({
        title: group.orch_id || "orchestrator",
        copy: Array.isArray(group.agent_ids) ? group.agent_ids.join(", ") : "",
        meta: [
          { label: "Agents", value: normalizeMetric(group.agent_count, 0) },
          {
            label: "Sessions",
            value:
              Array.isArray(group.session_ids) && group.session_ids.length
                ? group.session_ids.join(", ")
                : "none",
          },
        ],
        filterTags: uniqueValues([
          "all",
          "mesh",
          "orch-managed",
          group.orch_id ? `orch:${group.orch_id}` : "",
        ]),
        detail: {
          title: group.orch_id || "Managed orchestrator",
          eyebrow: "Mesh topology",
          copy: "Solver agents currently grouped under one orchestrator authority.",
          fields: compactDetails([
            makeDetailEntry("Agent count", group.agent_count),
            makeDetailEntry("Agents", Array.isArray(group.agent_ids) ? group.agent_ids.join(", ") : ""),
            makeDetailEntry("Sessions", Array.isArray(group.session_ids) ? group.session_ids.join(", ") : ""),
          ]),
        },
      })),
      filterTags: ["all", "mesh", "orch-managed"],
    },
    {
      eyebrow: "Authority",
      title: "Authority groups",
      copy: "Mesh control stays legible when grouped by control mode and active orchestrator binding.",
      stats: [
        { label: "Groups", value: String(authorityGroups.length) },
        { label: "Transitions", value: String(transitions.length) },
      ],
      items: authorityGroups.slice(0, 6).map((group) => ({
        title: group.orch_id || sentenceCase(group.control_mode || "group"),
        copy: Array.isArray(group.agent_ids) ? group.agent_ids.join(", ") : "",
        meta: [
          { label: "Mode", value: normalizeMetric(group.control_mode, "unknown") },
          { label: "Session", value: normalizeMetric(group.orch_session_id, "none") },
        ],
        filterTags: uniqueValues([
          "all",
          "mesh",
          toSlug(group.control_mode, ""),
          group.orch_id ? `orch:${group.orch_id}` : "",
        ]),
        detail: {
          title: group.orch_id || sentenceCase(group.control_mode || "authority"),
          eyebrow: "Authority",
          copy: "This group shows which agents share the same visible control binding.",
          fields: compactDetails([
            makeDetailEntry("Mode", group.control_mode),
            makeDetailEntry("Orchestrator", group.orch_id),
            makeDetailEntry("Session", group.orch_session_id),
            makeDetailEntry("Agents", Array.isArray(group.agent_ids) ? group.agent_ids.join(", ") : ""),
          ]),
        },
      })),
      filterTags: ["all", "mesh", "authority"],
    },
    {
      eyebrow: "Recent state",
      title: "Session transitions",
      copy: "Recent control transitions are surfaced here so mesh detaches and rebinds do not disappear into logs.",
      stats: [
        { label: "Recent", value: String(transitions.length) },
        { label: "Stale after", value: `${normalizeMetric(registry.stale_after_ms, 0)} ms` },
      ],
      items: transitions.slice(0, 6).map((event) => ({
        title: event.agent_id || "agent",
        copy: [event.from ? `${event.from} -> ${event.to}` : event.to, event.reason, event.source]
          .filter(Boolean)
          .join(" · "),
        meta: [{ label: "At", value: normalizeMetric(event.at, "n/a") }],
        filterTags: uniqueValues([
          "all",
          "mesh",
          toSlug(event.to, ""),
          toSlug(event.reason, ""),
        ]),
        detail: {
          title: event.agent_id || "Transition",
          eyebrow: "Recent state",
          copy: "Recent control transitions help explain why a mesh route changed ownership or detached.",
          fields: compactDetails([
            makeDetailEntry("From", event.from),
            makeDetailEntry("To", event.to),
            makeDetailEntry("Reason", event.reason),
            makeDetailEntry("Source", event.source),
            makeDetailEntry("At", event.at),
          ]),
        },
      })),
      filterTags: ["all", "mesh", "transitions"],
    },
  ].filter((section) => section.items.length > 0 || section.stats.length > 0);
}

function buildRuntimeFilters(meshHealth: UnknownRecord | null): RuntimeFilter[] {
  if (!meshHealth) return [];
  const registry = asRecord(meshHealth.remote_solver_registry) || {};
  const offlineCount =
    registry.control_modes?.offline_mesh || registry.mesh_topology?.offline_mesh?.agent_count || 0;
  const managedCount = registry.control_modes?.orch_managed || 0;
  const degradedCount = asArray(meshHealth.solver_agents).filter(
    (agent) => typeof agent.cooldown_remaining_ms === "number" && agent.cooldown_remaining_ms > 0,
  ).length;
  const filters: RuntimeFilter[] = [{ value: "all", label: "All" }];
  if (managedCount > 0) filters.push({ value: "orch-managed", label: "Managed" });
  if (offlineCount > 0) filters.push({ value: "offline-mesh", label: "Offline mesh" });
  if (degradedCount > 0) filters.push({ value: "degraded", label: "Degraded" });
  return filters;
}

function hasFilterTag(entity: Filterable | null | undefined, filterValue: string): boolean {
  if (!filterValue || filterValue === "all") return true;
  return Array.isArray(entity?.filterTags) && entity.filterTags.includes(filterValue);
}

function filterItems<T extends Filterable>(items: T[] | undefined, filterValue: string): T[] {
  return (Array.isArray(items) ? items : []).filter((item) => hasFilterTag(item, filterValue));
}

function applyModelFilter(model: RuntimeStatusModel, filterValue: string): RuntimeStatusModel {
  if (!filterValue || filterValue === "all") return { ...model, selectedFilter: "all" };
  return {
    ...model,
    summary: `${model.summary} · filter ${sentenceCase(filterValue)}`,
    selectedFilter: filterValue,
    runtimes: filterItems(model.runtimes, filterValue),
    meshPanels: model.meshPanels
      .filter((panel) => hasFilterTag(panel, filterValue) || filterItems(panel.items, filterValue).length > 0)
      .map((panel) => ({ ...panel, items: filterItems(panel.items, filterValue) })),
    topology: model.topology
      .filter((section) => hasFilterTag(section, filterValue) || filterItems(section.items, filterValue).length > 0)
      .map((section) => ({ ...section, items: filterItems(section.items, filterValue) })),
  };
}

export function buildRuntimeStatusModel(
  summaryPayload: unknown,
  healthPayload: unknown,
): RuntimeStatusModel {
  const runtimeSummary = asRecord(summaryPayload);
  const meshHealth = asRecord(healthPayload);
  const meshRuntimes = buildMeshRuntimeEntries(meshHealth);
  const deploymentMode = runtimeSummary?.deployment_mode || meshHealth?.deployment?.mode || "unknown";
  const totalMeshAgents =
    meshHealth?.remote_solver_registry?.total_agents ??
    meshHealth?.deployment?.endpoint_count ??
    meshRuntimes.length;
  return {
    summary: meshHealth
      ? `Runtime ${deploymentMode} · mesh agents ${normalizeMetric(totalMeshAgents, 0)}`
      : runtimeSummary
        ? `Runtime ${deploymentMode} · local agents ${normalizeMetric(runtimeSummary.agent_count, 0)}`
        : "No runtime status available.",
    runtimes: [...buildLocalRuntimeEntries(runtimeSummary), ...meshRuntimes],
    meshPanels: buildMeshPanels(runtimeSummary, meshHealth),
    topology: buildTopologySections(meshHealth),
    filters: buildRuntimeFilters(meshHealth),
    selectedFilter: "all",
    detailSelection: null,
  };
}

export function selectRuntimeStatusModelFilter(
  model: RuntimeStatusModel,
  filterValue: string,
): RuntimeStatusModel {
  return applyModelFilter(model, filterValue);
}

function collectSelectableEntries(model: RuntimeStatusModel): RuntimeSelection[] {
  return [
    ...model.runtimes.map((entry) => ({ key: `runtime:${entry.id}`, detail: entry.detail })),
    ...model.meshPanels.flatMap((panel, index) => [
      ...(panel.detail ? [{ key: `panel:${index}`, detail: panel.detail }] : []),
      ...panel.items.map((item, itemIndex) => ({
        key: `panel-item:${index}:${itemIndex}:${item.title || item.id || itemIndex}`,
        detail: item.detail,
      })),
    ]),
    ...model.topology.flatMap((section, index) =>
      section.items.map((item, itemIndex) => ({
        key: `topology-item:${index}:${itemIndex}:${item.title || item.id || itemIndex}`,
        detail: item.detail,
      })),
    ),
  ].filter((entry): entry is RuntimeSelection => Boolean(entry.detail));
}

export function selectRuntimeStatusModelDetail(
  model: RuntimeStatusModel,
  selectionKey: string,
): RuntimeStatusModel {
  const selectable = collectSelectableEntries(model);
  const selected = selectable.find((entry) => entry.key === selectionKey) || selectable[0] || null;
  return {
    ...model,
    detailSelection: selected ? { key: selected.key, detail: selected.detail } : null,
  };
}
