"use client";

import { useEffect, useMemo, useState } from "react";
import type { WorkflowGraphNode, WorkflowOperatorDescriptor } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import {
  buildOperatorOptionLabel,
  sortWorkflowOperatorOptionPresets,
} from "@/components/workbench/workflow/workbench-workflow-operator-descriptor-summary";
import {
  buildWorkflowOperatorPresetRecommendations,
} from "@/components/workbench/workflow/workbench-workflow-operator-compatibility";
import {
  scoreWorkflowNodeTemplatePresetSearch,
  suggestWorkflowNodeTemplatePresets,
} from "@/components/workbench/workflow/workbench-workflow-operator-search-match";
import type { WorkflowNodeTemplatePreset } from "@/components/workbench/workflow/workbench-workflow-node-templates";

const RECENT_OPERATOR_STORAGE_KEY = "kyuubiki.workflow.recentOperators";
const FAVORITE_OPERATOR_STORAGE_KEY = "kyuubiki.workflow.favoriteOperators";

function rankWorkflowOperatorValidationStatus(
  status?: WorkflowOperatorDescriptor["validation"]["baseline_status"],
) {
  if (status === "verified") return 0;
  if (status === "partial") return 1;
  if (status === "unverified") return 2;
  return 3;
}

function groupWorkflowOperatorOptionPresets(
  presets: WorkflowNodeTemplatePreset[],
  operatorDescriptorMap: Map<string, WorkflowOperatorDescriptor>,
) {
  const groups = new Map<string, WorkflowNodeTemplatePreset[]>();
  for (const preset of presets) {
    const descriptor = preset.operatorId
      ? operatorDescriptorMap.get(preset.operatorId)
      : undefined;
    const domain = descriptor?.domain ?? "other";
    const current = groups.get(domain);
    if (current) current.push(preset);
    else groups.set(domain, [preset]);
  }
  return [...groups.entries()].sort(([left], [right]) => left.localeCompare(right));
}

export function filterWorkflowOperatorOptionPresets(
  presets: WorkflowNodeTemplatePreset[],
  operatorDescriptorMap: Map<string, WorkflowOperatorDescriptor>,
  query: string,
  filters?: {
    domain: string;
    validation: string;
    capability: string;
  },
) {
  const normalizedQuery = query.trim();
  const ranked = presets.flatMap((preset) => {
    const descriptor = preset.operatorId
      ? operatorDescriptorMap.get(preset.operatorId)
      : undefined;
    if (filters?.domain && descriptor?.domain !== filters.domain) return [];
    if (
      filters?.validation &&
      descriptor?.validation?.baseline_status !== filters.validation
    ) {
      return [];
    }
    if (
      filters?.capability &&
      !descriptor?.capability_tags.includes(filters.capability)
    ) {
      return [];
    }
    const score = scoreWorkflowNodeTemplatePresetSearch(
      preset,
      descriptor,
      normalizedQuery,
    );
    if (normalizedQuery && score == null) return [];
    return [{ preset, score: score ?? 0 }];
  });
  if (!normalizedQuery) {
    return sortWorkflowOperatorOptionPresets(
      ranked.map((entry) => entry.preset),
      operatorDescriptorMap,
    );
  }
  return ranked
    .sort((left, right) => {
      const scoreDiff = right.score - left.score;
      if (scoreDiff !== 0) return scoreDiff;
      const leftDescriptor = left.preset.operatorId
        ? operatorDescriptorMap.get(left.preset.operatorId)
        : undefined;
      const rightDescriptor = right.preset.operatorId
        ? operatorDescriptorMap.get(right.preset.operatorId)
        : undefined;
      const validationDiff =
        rankWorkflowOperatorValidationStatus(leftDescriptor?.validation?.baseline_status) -
        rankWorkflowOperatorValidationStatus(rightDescriptor?.validation?.baseline_status);
      if (validationDiff !== 0) return validationDiff;
      return left.preset.label.localeCompare(right.preset.label);
    })
    .map((entry) => entry.preset);
}

export function WorkbenchWorkflowOperatorSearch(props: {
  labels: WorkflowSidebarLabels;
  operatorId: string;
  query: string;
  domainFilter: string;
  validationFilter: string;
  capabilityFilter: string;
  filteredPresets: WorkflowNodeTemplatePreset[];
  availableDomains: string[];
  availableCapabilities: string[];
  selectedSourceNode?: WorkflowGraphNode | null;
  operatorDescriptorMap: Map<string, WorkflowOperatorDescriptor>;
  onOperatorIdChange: (value: string) => void;
  onQueryChange: (value: string) => void;
  onDomainFilterChange: (value: string) => void;
  onValidationFilterChange: (value: string) => void;
  onCapabilityFilterChange: (value: string) => void;
  onQuickInsert: (operatorId: string) => void;
}) {
  const {
    labels,
    operatorId,
    query,
    domainFilter,
    validationFilter,
    capabilityFilter,
    filteredPresets,
    availableDomains,
    availableCapabilities,
    selectedSourceNode,
    operatorDescriptorMap,
    onOperatorIdChange,
    onQueryChange,
    onDomainFilterChange,
    onValidationFilterChange,
    onCapabilityFilterChange,
    onQuickInsert,
  } = props;
  const [recentOperatorIds, setRecentOperatorIds] = useState<string[]>([]);
  const [favoriteOperatorIds, setFavoriteOperatorIds] = useState<string[]>([]);
  const groupedPresets = groupWorkflowOperatorOptionPresets(
    filteredPresets,
    operatorDescriptorMap,
  );
  const filteredPresetByOperatorId = useMemo(
    () =>
      new Map(
        filteredPresets.flatMap((preset) =>
          preset.operatorId ? [[preset.operatorId, preset] as const] : [],
        ),
      ),
    [filteredPresets],
  );
  useEffect(() => {
    try {
      const raw = window.localStorage.getItem(RECENT_OPERATOR_STORAGE_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        setRecentOperatorIds(parsed.filter((value): value is string => typeof value === "string"));
      }
    } catch {}
  }, []);
  useEffect(() => {
    try {
      const raw = window.localStorage.getItem(FAVORITE_OPERATOR_STORAGE_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        setFavoriteOperatorIds(
          parsed.filter((value): value is string => typeof value === "string"),
        );
      }
    } catch {}
  }, []);
  const favoritePresets = useMemo(
    () =>
      favoriteOperatorIds
        .map((operatorId) => filteredPresetByOperatorId.get(operatorId))
        .filter(Boolean)
        .slice(0, 8) as WorkflowNodeTemplatePreset[],
    [favoriteOperatorIds, filteredPresetByOperatorId],
  );
  const recentPresets = useMemo(
    () =>
      recentOperatorIds
        .map((operatorId) => filteredPresetByOperatorId.get(operatorId))
        .filter(Boolean)
        .slice(0, 6) as WorkflowNodeTemplatePreset[],
    [filteredPresetByOperatorId, recentOperatorIds],
  );
  const compatiblePresets = useMemo(
    () =>
      buildWorkflowOperatorPresetRecommendations({
        operatorDescriptorMap,
        presets: filteredPresets,
        sourceNode: selectedSourceNode ?? null,
      }),
    [filteredPresets, operatorDescriptorMap, selectedSourceNode],
  );
  const searchSuggestions = useMemo(
    () => suggestWorkflowNodeTemplatePresets(filteredPresets, operatorDescriptorMap, query.trim()),
    [filteredPresets, operatorDescriptorMap, query],
  );
  const activeFilters = [
    query.trim() ? `${labels.operatorSearchLabel}: ${query.trim()}` : null,
    domainFilter ? `${labels.operatorDomainFilterLabel}: ${domainFilter}` : null,
    validationFilter
      ? `${labels.operatorValidationFilterLabel}: ${validationFilter}`
      : null,
    capabilityFilter
      ? `${labels.operatorCapabilityFilterLabel}: ${capabilityFilter}`
      : null,
  ].filter(Boolean) as string[];
  const hasActiveFilters = activeFilters.length > 0;
  function rememberOperator(operatorId: string) {
    const next = [operatorId, ...recentOperatorIds.filter((value) => value !== operatorId)].slice(
      0,
      12,
    );
    setRecentOperatorIds(next);
    try {
      window.localStorage.setItem(RECENT_OPERATOR_STORAGE_KEY, JSON.stringify(next));
    } catch {}
  }
  function toggleFavoriteOperator(operatorId: string) {
    const next = favoriteOperatorIds.includes(operatorId)
      ? favoriteOperatorIds.filter((value) => value !== operatorId)
      : [operatorId, ...favoriteOperatorIds].slice(0, 16);
    setFavoriteOperatorIds(next);
    try {
      window.localStorage.setItem(FAVORITE_OPERATOR_STORAGE_KEY, JSON.stringify(next));
    } catch {}
  }
  function favoriteLabel(operatorId: string) {
    return favoriteOperatorIds.includes(operatorId)
      ? labels.operatorFavoriteRemoveLabel
      : labels.operatorFavoriteAddLabel;
  }

  return (
    <>
      <label>
        <span>{labels.operatorSearchLabel}</span>
        <input
          onChange={(event) => onQueryChange(event.target.value)}
          placeholder={labels.operatorSearchPlaceholder}
          value={query}
        />
      </label>
      <label>
        <span>{labels.operatorDomainFilterLabel}</span>
        <select onChange={(event) => onDomainFilterChange(event.target.value)} value={domainFilter}>
          <option value="">{labels.operatorFilterAllLabel}</option>
          {availableDomains.map((domain) => (
            <option key={domain} value={domain}>
              {domain}
            </option>
          ))}
        </select>
      </label>
      <label>
        <span>{labels.operatorValidationFilterLabel}</span>
        <select onChange={(event) => onValidationFilterChange(event.target.value)} value={validationFilter}>
          <option value="">{labels.operatorFilterAllLabel}</option>
          <option value="verified">verified</option>
          <option value="partial">partial</option>
          <option value="unverified">unverified</option>
        </select>
      </label>
      <label>
        <span>{labels.operatorCapabilityFilterLabel}</span>
        <select onChange={(event) => onCapabilityFilterChange(event.target.value)} value={capabilityFilter}>
          <option value="">{labels.operatorFilterAllLabel}</option>
          {availableCapabilities.map((capability) => (
            <option key={capability} value={capability}>
              {capability}
            </option>
          ))}
        </select>
      </label>
      <label>
        <span>{labels.operatorLabel}</span>
        <select
          onChange={(event) => {
            onOperatorIdChange(event.target.value);
            if (event.target.value) rememberOperator(event.target.value);
          }}
          value={operatorId}
        >
          <option value="">{labels.datasetNoneLabel}</option>
          {groupedPresets.map(([domain, presets]) => (
            <optgroup key={domain} label={domain}>
              {presets.map((preset) => (
                <option key={preset.id} value={preset.operatorId}>
                  {buildOperatorOptionLabel(
                    labels,
                    preset.label,
                    preset.operatorId
                      ? operatorDescriptorMap.get(preset.operatorId)
                      : undefined,
                  )}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
      </label>
      <div className="sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.operatorFilterMatchesLabel}</span>
          <strong>{filteredPresets.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.operatorFilterActiveLabel}</span>
          <strong>{hasActiveFilters ? activeFilters.join(" · ") : labels.operatorFilterAllLabel}</strong>
        </div>
        {hasActiveFilters ? (
          <div className="button-row">
            <button
              onClick={() => {
                onQueryChange("");
                onDomainFilterChange("");
                onValidationFilterChange("");
                onCapabilityFilterChange("");
              }}
              type="button"
            >
              {labels.operatorFilterClearLabel}
            </button>
          </div>
        ) : null}
      </div>
      {recentPresets.length > 0 ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{labels.operatorRecentLabel}</span>
            <strong>{recentPresets.length}</strong>
          </div>
          <div className="button-row button-row--adaptive">
            {recentPresets.map((preset) =>
              preset.operatorId ? (
                <button
                  key={`recent:${preset.id}`}
                  onClick={() => {
                    rememberOperator(preset.operatorId!);
                    onQuickInsert(preset.operatorId!);
                  }}
                  type="button"
                >
                  {preset.label}
                </button>
              ) : null,
            )}
          </div>
        </div>
      ) : null}
      {favoritePresets.length > 0 ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{labels.operatorFavoritesLabel}</span>
            <strong>{favoritePresets.length}</strong>
          </div>
          <div className="button-row button-row--adaptive">
            {favoritePresets.map((preset) =>
              preset.operatorId ? (
                <button
                  key={`favorite:${preset.id}`}
                  onClick={() => {
                    rememberOperator(preset.operatorId!);
                    onQuickInsert(preset.operatorId!);
                  }}
                  type="button"
                >
                  {preset.label}
                </button>
              ) : null,
            )}
          </div>
        </div>
      ) : null}
      {compatiblePresets.length > 0 ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>Recommended next operators</span>
            <strong>{compatiblePresets.length}</strong>
          </div>
          <div style={{ display: "grid", gap: "0.45rem" }}>
            {compatiblePresets.map(({ preset, reason }) =>
              preset.operatorId ? (
                <div key={`compatible:${preset.id}`} style={{ display: "grid", gap: "0.2rem" }}>
                  <div className="button-row button-row--adaptive">
                    <button
                      onClick={() => {
                        rememberOperator(preset.operatorId!);
                        onQuickInsert(preset.operatorId!);
                      }}
                      type="button"
                    >
                      {preset.label}
                    </button>
                    <button
                      onClick={() => toggleFavoriteOperator(preset.operatorId!)}
                      type="button"
                    >
                      {favoriteLabel(preset.operatorId!)}
                    </button>
                  </div>
                  <span className="card-copy">{reason}</span>
                </div>
              ) : null,
            )}
          </div>
        </div>
      ) : null}
      {filteredPresets.length > 0 ? (
        <div className="button-row button-row--adaptive">
          {(query.trim() ? searchSuggestions.slice(0, 6) : filteredPresets.slice(0, 6).map((preset) => ({
            preset,
            score: 0,
            matchSummary: [],
          }))).map((entry) =>
            entry.preset.operatorId ? (
              <div key={`quick:${entry.preset.id}`} style={{ display: "grid", gap: "0.2rem" }}>
                <div style={{ display: "flex", gap: "0.35rem" }}>
                <button
                  onClick={() => {
                    rememberOperator(entry.preset.operatorId!);
                    onQuickInsert(entry.preset.operatorId!);
                  }}
                  type="button"
                >
                  {entry.preset.label}
                </button>
                <button
                  onClick={() => toggleFavoriteOperator(entry.preset.operatorId!)}
                  type="button"
                >
                  {favoriteLabel(entry.preset.operatorId!)}
                </button>
                </div>
                {query.trim() ? (
                  <span className="card-copy">
                    {labels.operatorSearchScoreLabel}: {entry.score}
                    {entry.matchSummary.length > 0
                      ? ` · ${labels.operatorSearchReasonLabel}: ${entry.matchSummary.join(" · ")}`
                      : ""}
                  </span>
                ) : null}
              </div>
            ) : null,
          )}
        </div>
      ) : null}
      {query.trim() && filteredPresets.length === 0 ? (
        <p className="card-copy">{labels.operatorSearchEmptyLabel}</p>
      ) : null}
    </>
  );
}
