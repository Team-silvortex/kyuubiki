"use client";

import { useEffect, useMemo, useState } from "react";
import type { WorkflowOperatorDescriptor } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import {
  buildOperatorOptionLabel,
  sortWorkflowOperatorOptionPresets,
} from "@/components/workbench/workflow/workbench-workflow-operator-descriptor-summary";

type WorkflowOperatorOptionPreset = {
  id: string;
  label: string;
  operatorId?: string;
};

const RECENT_OPERATOR_STORAGE_KEY = "kyuubiki.workflow.recentOperators";
const FAVORITE_OPERATOR_STORAGE_KEY = "kyuubiki.workflow.favoriteOperators";

function groupWorkflowOperatorOptionPresets(
  presets: WorkflowOperatorOptionPreset[],
  operatorDescriptorMap: Map<string, WorkflowOperatorDescriptor>,
) {
  const groups = new Map<string, WorkflowOperatorOptionPreset[]>();
  for (const preset of presets) {
    const descriptor = preset.operatorId
      ? operatorDescriptorMap.get(preset.operatorId)
      : undefined;
    const domain = descriptor?.domain ?? "other";
    groups.set(domain, [...(groups.get(domain) ?? []), preset]);
  }
  return [...groups.entries()].sort(([left], [right]) => left.localeCompare(right));
}

export function filterWorkflowOperatorOptionPresets(
  presets: WorkflowOperatorOptionPreset[],
  operatorDescriptorMap: Map<string, WorkflowOperatorDescriptor>,
  query: string,
  filters?: {
    domain: string;
    validation: string;
    capability: string;
  },
) {
  const sorted = sortWorkflowOperatorOptionPresets(presets, operatorDescriptorMap);
  const normalizedQuery = query.trim().toLowerCase();
  return sorted.filter((preset) => {
    const descriptor = preset.operatorId
      ? operatorDescriptorMap.get(preset.operatorId)
      : undefined;
    if (filters?.domain && descriptor?.domain !== filters.domain) return false;
    if (
      filters?.validation &&
      descriptor?.validation?.baseline_status !== filters.validation
    ) {
      return false;
    }
    if (
      filters?.capability &&
      !descriptor?.capability_tags.includes(filters.capability)
    ) {
      return false;
    }
    if (!normalizedQuery) return true;
    const haystack = [
      preset.label,
      preset.operatorId,
      descriptor?.summary,
      descriptor?.family,
      descriptor?.domain,
      descriptor?.capability_tags.join(" "),
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return haystack.includes(normalizedQuery);
  });
}

export function WorkbenchWorkflowOperatorSearch(props: {
  labels: WorkflowSidebarLabels;
  operatorId: string;
  query: string;
  domainFilter: string;
  validationFilter: string;
  capabilityFilter: string;
  filteredPresets: WorkflowOperatorOptionPreset[];
  availableDomains: string[];
  availableCapabilities: string[];
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
        .map((operatorId) =>
          filteredPresets.find((preset) => preset.operatorId === operatorId),
        )
        .filter(Boolean)
        .slice(0, 8) as WorkflowOperatorOptionPreset[],
    [favoriteOperatorIds, filteredPresets],
  );
  const recentPresets = useMemo(
    () =>
      recentOperatorIds
        .map((operatorId) =>
          filteredPresets.find((preset) => preset.operatorId === operatorId),
        )
        .filter(Boolean)
        .slice(0, 6) as WorkflowOperatorOptionPreset[],
    [filteredPresets, recentOperatorIds],
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
      {filteredPresets.length > 0 ? (
        <div className="button-row button-row--adaptive">
          {filteredPresets.slice(0, 6).map((preset) =>
            preset.operatorId ? (
              <div key={`quick:${preset.id}`} style={{ display: "flex", gap: "0.35rem" }}>
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
