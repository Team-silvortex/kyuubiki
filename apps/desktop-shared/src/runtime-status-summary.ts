import {
  buildRuntimeStatusModel,
  selectRuntimeStatusModelDetail,
  selectRuntimeStatusModelFilter,
} from "./runtime-status-model.js";
import type { RuntimeStatusModel, UnknownRecord } from "./runtime-status-types.js";

type RenderOptions = {
  healthPayload?: unknown;
  selectedFilter?: string;
  selectedDetail?: string;
  onFilterChange?: (filter: string) => void;
};

type DecoratedRuntimeStatusModel = RuntimeStatusModel & {
  runtimes: Array<UnknownRecord>;
  meshPanels: Array<UnknownRecord>;
  topology: Array<UnknownRecord>;
};

function isRecord(value: unknown): value is UnknownRecord {
  return Boolean(value && typeof value === "object");
}

function escapeHtml(value: unknown): string {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function normalizeMetric(value: unknown, fallback: unknown = "--", ..._unused: unknown[]): string {
  if (value === undefined || value === null || value === "") return String(fallback);
  if (typeof value === "number") return Number.isFinite(value) ? String(value) : String(fallback);
  if (typeof value === "boolean") return value ? "Yes" : "No";
  return String(value);
}

function sentenceCase(value: unknown): string {
  const text = String(value ?? "").trim().replaceAll(/[_-]+/gu, " ");
  if (!text) return "Unknown";
  return text.charAt(0).toUpperCase() + text.slice(1);
}

function buildRuntimeStateCard(runtime: UnknownRecord): string {
  const id = normalizeMetric(runtime.id, "runtime");
  const label = normalizeMetric(runtime.label || runtime.name || runtime.id, id);
  const state = String(runtime.status || runtime.state || "unknown").toLowerCase();
  const deployment = String(runtime.deployment || runtime.mode || "local").toLowerCase();
  const runtimeType = String(runtime.type || runtime.kind || "runtime").toLowerCase();
  const authority = String(runtime.authority || runtime.controlMode || "").toLowerCase();
  const note = normalizeMetric(runtime.note || runtime.summary || runtime.endpoint, "No details available.");
  const badge = sentenceCase(runtime.badge || state);
  const selectionKey = normalizeMetric(runtime.selectionKey, "");
  return `
    <article class="desktop-runtime-plane__card ${runtime.isSelected ? "desktop-runtime-plane__card--selected" : ""} desktop-runtime-plane__card--deployment-${escapeHtml(
      deployment,
    )} desktop-runtime-plane__card--type-${escapeHtml(runtimeType)} desktop-runtime-plane__card--state-${escapeHtml(
      state,
    )} ${authority ? `desktop-runtime-plane__card--authority-${escapeHtml(authority)}` : ""}" ${
      selectionKey ? `data-runtime-detail="${escapeHtml(selectionKey)}"` : ""
    } tabindex="${selectionKey ? "0" : "-1"}">
      <span class="desktop-runtime-plane__label">${escapeHtml(label)}</span>
      <strong class="desktop-runtime-plane__value desktop-shell-state desktop-shell-state--${escapeHtml(state)}">${escapeHtml(
        sentenceCase(state),
      )}</strong>
      <span class="desktop-runtime-plane__badge">${escapeHtml(badge)}</span>
      <span class="desktop-runtime-plane__note">${escapeHtml(note)}</span>
    </article>
  `;
}

function buildTopologyStat(label: unknown, value: unknown): string {
  return `
    <div class="desktop-mesh-topology__stat">
      <span class="desktop-mesh-topology__stat-label">${escapeHtml(label)}</span>
      <strong class="desktop-mesh-topology__stat-value">${escapeHtml(normalizeMetric(value))}</strong>
    </div>
  `;
}

function buildTopologyItem(item: UnknownRecord): string {
  const title = normalizeMetric(item.title || item.label || item.id, "Topology item");
  const copy = normalizeMetric(item.copy || item.summary || item.description, "");
  const meta = Array.isArray(item.meta) ? item.meta : [];
  const selectionKey = normalizeMetric(item.selectionKey, "");
  return `
    <div class="desktop-mesh-topology__item ${item.isSelected ? "desktop-mesh-topology__item--selected" : ""}" ${
      selectionKey ? `data-runtime-detail="${escapeHtml(selectionKey)}"` : ""
    } tabindex="${selectionKey ? "0" : "-1"}">
      <div class="desktop-mesh-topology__item-head">
        <strong class="desktop-mesh-topology__item-title">${escapeHtml(title)}</strong>
        <div class="desktop-mesh-topology__item-meta">
          ${meta
            .map(
              (entry) =>
                `<span class="desktop-runtime-plane__pill"><strong>${escapeHtml(
                  normalizeMetric(entry.label, ""),
                )}</strong>${escapeHtml(normalizeMetric(entry.value, ""))}</span>`,
            )
            .join("")}
        </div>
      </div>
      ${copy ? `<p class="desktop-mesh-topology__item-copy">${escapeHtml(copy)}</p>` : ""}
    </div>
  `;
}

function buildTopologySection(section: UnknownRecord): string {
  const stats = Array.isArray(section.stats) ? section.stats : [];
  const items = Array.isArray(section.items) ? section.items : [];
  return `
    <section class="desktop-mesh-topology__section">
      <div class="desktop-mesh-topology__header">
        <div>
          <div class="desktop-mesh-topology__eyebrow">${escapeHtml(normalizeMetric(section.eyebrow, "Mesh"))}</div>
          <strong class="desktop-mesh-topology__title">${escapeHtml(normalizeMetric(section.title, "Topology"))}</strong>
        </div>
        ${
          section.copy
            ? `<p class="desktop-mesh-topology__copy">${escapeHtml(normalizeMetric(section.copy))}</p>`
            : ""
        }
      </div>
      ${stats.length ? `<div class="desktop-mesh-topology__stats">${stats.map((entry) => buildTopologyStat(entry.label, entry.value)).join("")}</div>` : ""}
      ${
        items.length
          ? `<div class="desktop-mesh-topology__list">${items.map((item) => buildTopologyItem(item)).join("")}</div>`
          : `<div class="desktop-mesh-topology__empty">No topology items reported.</div>`
      }
    </section>
  `;
}

function buildMeshPanel(panel: UnknownRecord): string {
  const pills = Array.isArray(panel.pills) ? panel.pills : [];
  const items = Array.isArray(panel.items) ? panel.items : [];
  const classes = ["desktop-runtime-plane__mesh-panel"];
  if (panel.wide) classes.push("desktop-runtime-plane__mesh-panel--wide");
  const selectionKey = normalizeMetric(panel.selectionKey, "");
  return `
    <section class="${classes.join(" ")} ${panel.isSelected ? "desktop-runtime-plane__mesh-panel--selected" : ""}" ${
      selectionKey ? `data-runtime-detail="${escapeHtml(selectionKey)}"` : ""
    } tabindex="${selectionKey ? "0" : "-1"}">
      <span class="desktop-runtime-plane__mesh-label">${escapeHtml(normalizeMetric(panel.label, "Mesh panel"))}</span>
      <strong class="desktop-runtime-plane__mesh-title">${escapeHtml(normalizeMetric(panel.title, "Untitled"))}</strong>
      ${
        pills.length
          ? `<div class="desktop-runtime-plane__pill-row">${pills
              .map(
                (pill) =>
                  `<span class="desktop-runtime-plane__pill"><strong>${escapeHtml(normalizeMetric(pill.label, ""))}</strong>${escapeHtml(
                    normalizeMetric(pill.value, ""),
                  )}</span>`,
              )
              .join("")}</div>`
          : ""
      }
      ${
        items.length
          ? `<div class="desktop-runtime-plane__mesh-list">${items
              .map(
                (item) => `
                  <article class="desktop-runtime-plane__mesh-item ${item.isSelected ? "desktop-runtime-plane__mesh-item--selected" : ""}" ${
                    item.selectionKey ? `data-runtime-detail="${escapeHtml(item.selectionKey)}"` : ""
                  } tabindex="${item.selectionKey ? "0" : "-1"}">
                    <div class="desktop-runtime-plane__mesh-head">
                      <strong>${escapeHtml(normalizeMetric(item.title, item.id, "Item"))}</strong>
                      <span>${escapeHtml(normalizeMetric(item.meta, ""))}</span>
                    </div>
                    ${item.copy ? `<p>${escapeHtml(normalizeMetric(item.copy))}</p>` : ""}
                  </article>
                `,
              )
              .join("")}</div>`
          : `<div class="desktop-runtime-plane__empty">No mesh entries reported.</div>`
      }
    </section>
  `;
}

function buildFilterRow(filters: UnknownRecord[], selectedValue: string): string {
  if (!Array.isArray(filters) || filters.length === 0) return "";
  return `
    <div class="desktop-runtime-plane__filters">
      ${filters
        .map((filter) => {
          const value = normalizeMetric(filter.value, "");
          const active = value === selectedValue;
          return `<button class="desktop-runtime-plane__filter${active ? " is-active" : ""}" data-runtime-filter="${escapeHtml(
            value,
          )}" type="button">${escapeHtml(normalizeMetric(filter.label, value || "Filter"))}</button>`;
        })
        .join("")}
    </div>
  `;
}

function normalizePayload(payload: unknown): RuntimeStatusModel {
  if (!isRecord(payload)) {
    return {
      summary: "No runtime status available.",
      runtimes: [],
      meshPanels: [],
      topology: [],
      filters: [],
      selectedFilter: "",
      detailSelection: null,
    };
  }

  return {
    summary: normalizeMetric(payload.summary, "Runtime status loaded."),
    runtimes: Array.isArray(payload.runtimes) ? payload.runtimes : [],
    meshPanels: Array.isArray(payload.meshPanels) ? payload.meshPanels : [],
    topology: Array.isArray(payload.topology) ? payload.topology : [],
    filters: Array.isArray(payload.filters) ? payload.filters : [],
    selectedFilter: normalizeMetric(payload.selectedFilter, ""),
    detailSelection: payload.detailSelection || null,
  };
}

function applySelectionDecorators(model: RuntimeStatusModel): DecoratedRuntimeStatusModel {
  const selectedKey = model.detailSelection?.key || "";
  return {
    ...model,
    runtimes: model.runtimes.map((runtime) => ({
      ...runtime,
      selectionKey: `runtime:${runtime.id}`,
      isSelected: selectedKey === `runtime:${runtime.id}`,
    })),
    meshPanels: model.meshPanels.map((panel, panelIndex) => ({
      ...panel,
      selectionKey: panel.detail ? `panel:${panelIndex}` : "",
      isSelected: selectedKey === `panel:${panelIndex}`,
      items: panel.items.map((item, itemIndex) => {
        const selectionKey = `panel-item:${panelIndex}:${itemIndex}:${item.title || item.id || itemIndex}`;
        return { ...item, selectionKey, isSelected: selectedKey === selectionKey };
      }),
    })),
    topology: model.topology.map((section, sectionIndex) => ({
      ...section,
      items: section.items.map((item, itemIndex) => {
        const selectionKey = `topology-item:${sectionIndex}:${itemIndex}:${item.title || item.id || itemIndex}`;
        return { ...item, selectionKey, isSelected: selectedKey === selectionKey };
      }),
    })),
  };
}

function buildDetailPane(detailSelection: RuntimeStatusModel["detailSelection"]): string {
  const detail = detailSelection?.detail;
  if (!detail) return "";
  const fields = Array.isArray(detail.fields) ? detail.fields : [];
  return `
    <section class="desktop-runtime-detail">
      <div class="desktop-runtime-detail__header">
        <div>
          <div class="desktop-runtime-detail__eyebrow">${escapeHtml(normalizeMetric(detail.eyebrow, "Detail"))}</div>
          <strong class="desktop-runtime-detail__title">${escapeHtml(normalizeMetric(detail.title, "Selection"))}</strong>
        </div>
        ${detail.copy ? `<p class="desktop-runtime-detail__copy">${escapeHtml(normalizeMetric(detail.copy))}</p>` : ""}
      </div>
      ${
        fields.length
          ? `<div class="desktop-runtime-detail__grid">${fields
              .map(
                (field) => `
                  <div class="desktop-runtime-detail__field">
                    <span class="desktop-runtime-detail__label">${escapeHtml(normalizeMetric(field.label, ""))}</span>
                    <strong class="desktop-runtime-detail__value">${escapeHtml(normalizeMetric(field.value, ""))}</strong>
                  </div>
                `,
              )
              .join("")}</div>`
          : `<div class="desktop-runtime-plane__empty">No detail fields reported.</div>`
      }
    </section>
  `;
}

function normalizeRenderOptions(options: unknown): RenderOptions {
  return isRecord(options) ? (options as RenderOptions) : {};
}

function isMeshHealthPayload(value: unknown): boolean {
  return isRecord(value) && ("solver_agents" in value || "remote_solver_registry" in value || "deployment" in value);
}

function normalizeRenderInputs(
  payload: unknown,
  options: unknown = {},
): { model: DecoratedRuntimeStatusModel; options: RenderOptions } {
  const renderOptions = normalizeRenderOptions(options);
  const nestedSummary = isRecord(payload) && isRecord(payload.summary) ? payload.summary : null;
  const healthPayload =
    isRecord(options) && !("onFilterChange" in options) && isMeshHealthPayload(options)
      ? options
      : renderOptions.healthPayload;
  const summaryPayload =
    nestedSummary ||
    (isRecord(payload) &&
    ("deployment_mode" in payload ||
      "frontend_status" in payload ||
      "orchestrator_status" in payload ||
      "agent_count" in payload)
      ? payload
      : null);

  if (summaryPayload || healthPayload) {
    const model = buildRuntimeStatusModel(summaryPayload, healthPayload);
    const selectedFilter = typeof renderOptions.selectedFilter === "string" ? renderOptions.selectedFilter : model.selectedFilter;
    const selectedDetail = typeof renderOptions.selectedDetail === "string" ? renderOptions.selectedDetail : "";
    const filteredModel = selectRuntimeStatusModelFilter(model, selectedFilter);
    return {
      model: applySelectionDecorators(selectRuntimeStatusModelDetail(filteredModel, selectedDetail)),
      options:
        isRecord(options) && ("onFilterChange" in options || "healthPayload" in options)
          ? renderOptions
          : {},
    };
  }

  return {
    model: normalizePayload(payload) as DecoratedRuntimeStatusModel,
    options: renderOptions,
  };
}

export function formatRuntimeStatusReport(payload: unknown, options: unknown = {}): string {
  const { model } = normalizeRenderInputs(payload, options);
  return [
    model.summary,
    model.runtimes.length ? `runtimes: ${model.runtimes.length}` : "",
    model.meshPanels.length ? `mesh panels: ${model.meshPanels.length}` : "",
    model.topology.length ? `topology sections: ${model.topology.length}` : "",
  ]
    .filter(Boolean)
    .join("\n");
}

export function renderRuntimeStatusPlane(
  target: HTMLElement | null,
  payload: unknown,
  options: unknown = {},
): void {
  if (!target) return;
  const { model, options: resolvedOptions } = normalizeRenderInputs(payload, options);
  target.innerHTML = `
    ${model.runtimes.map((runtime) => buildRuntimeStateCard(runtime)).join("")}
    ${model.meshPanels.length ? `<div class="desktop-runtime-plane__mesh">${model.meshPanels.map((panel) => buildMeshPanel(panel)).join("")}</div>` : ""}
    ${buildFilterRow(model.filters, model.selectedFilter)}
    ${model.topology.length ? `<div class="desktop-mesh-topology">${model.topology.map((section) => buildTopologySection(section)).join("")}</div>` : ""}
    ${buildDetailPane(model.detailSelection)}
  `;

  target.querySelectorAll<HTMLElement>("[data-runtime-filter]").forEach((button) => {
    button.addEventListener("click", () => {
      const nextFilter = button.dataset.runtimeFilter || "all";
      if (typeof resolvedOptions.onFilterChange === "function") {
        resolvedOptions.onFilterChange(nextFilter);
        return;
      }

      renderRuntimeStatusPlane(target, payload, { ...resolvedOptions, selectedFilter: nextFilter });
    });
  });

  target.querySelectorAll<HTMLElement>("[data-runtime-detail]").forEach((node) => {
    const applySelection = () => {
      renderRuntimeStatusPlane(target, payload, {
        ...resolvedOptions,
        selectedFilter: model.selectedFilter,
        selectedDetail: node.getAttribute("data-runtime-detail") || "",
      });
    };
    node.addEventListener("click", applySelection);
    node.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        applySelection();
      }
    });
  });
}
