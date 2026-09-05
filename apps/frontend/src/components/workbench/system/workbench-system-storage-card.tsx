"use client";

import { useEffect, useState } from "react";
import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import { WorkbenchSystemOverviewCard } from "@/components/workbench/system/workbench-system-overview-card";

import {
  clearWorkbenchSafeStorage,
  clearWorkbenchStorageBucket,
  inspectWorkbenchStorage,
  listWorkbenchStorageRules,
  type WorkbenchStorageSnapshot,
} from "@/components/workbench/system/workbench-system-storage";

function formatBytes(bytes: number | null) {
  if (bytes === null || !Number.isFinite(bytes)) return "--";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function formatPercent(usageBytes: number | null, quotaBytes: number | null) {
  if (usageBytes === null || quotaBytes === null || quotaBytes <= 0) return "--";
  return `${((usageBytes / quotaBytes) * 100).toFixed(1)}%`;
}

type WorkbenchSystemStorageCardProps = {
  copy: WorkbenchCopy;
};

export function WorkbenchSystemStorageCard({ copy }: WorkbenchSystemStorageCardProps) {
  const [snapshot, setSnapshot] = useState<WorkbenchStorageSnapshot | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [page, setPage] = useState<"overview" | "details">("overview");

  async function refresh() {
    setSnapshot(await inspectWorkbenchStorage());
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function runAction(actionId: string, callback: () => void) {
    setBusyAction(actionId);
    try {
      callback();
      await refresh();
    } finally {
      setBusyAction(null);
    }
  }

  const largestBuckets = snapshot?.buckets.filter((bucket) => bucket.bytes > 0).slice(0, 5) ?? [];
  const storageRules = listWorkbenchStorageRules();

  return (
    <WorkbenchSystemOverviewCard
      className="runtime-overview-card"
      status={formatBytes(snapshot?.totalBytes ?? null)}
      title={copy.workflowPackageInstallRulesStorageLabel}
    >
      <div className="panel-tabs">
        <button
          className={`panel-tab${page === "overview" ? " panel-tab--active" : ""}`}
          onClick={() => setPage("overview")}
          type="button"
        >
          {copy.overview}
        </button>
        <button
          className={`panel-tab${page === "details" ? " panel-tab--active" : ""}`}
          onClick={() => setPage("details")}
          type="button"
        >
          {copy.details}
        </button>
      </div>
      {page === "overview" ? (
        <>
      <div className="sidebar-list sidebar-list--metrics">
        <div className="sidebar-list__row">
          <span>{copy.workflowPackageInstallRulesStorageScopeLabel}</span>
          <strong>{formatBytes(snapshot?.usageBytes ?? null)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>quota</span>
          <strong>{formatBytes(snapshot?.quotaBytes ?? null)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>quota %</span>
          <strong>{formatPercent(snapshot?.usageBytes ?? null, snapshot?.quotaBytes ?? null)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>localStorage keys</span>
          <strong>{snapshot?.localStorageKeys ?? "--"}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{copy.workflowPackageInstallRulesResidualsLabel}</span>
          <strong>{snapshot ? `${snapshot.unknownKeys} / ${formatBytes(snapshot.unknownBytes)}` : "--"}</strong>
        </div>
      </div>

      {largestBuckets.length > 0 ? (
        <div style={{ display: "grid", gap: "0.45rem", marginTop: "0.75rem" }}>
          {largestBuckets.map((bucket) => (
            <div key={bucket.id} style={{ display: "grid", gap: "0.2rem" }}>
              <div className="sidebar-list__row">
                <span>{storageBucketLabel(bucket.id, copy)}</span>
                <strong>{formatBytes(bucket.bytes)}</strong>
              </div>
                  <div className="sidebar-list__row">
                    <span>{copy.databaseRecordCount}</span>
                    <strong>{bucket.entries}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>{copy.data}</span>
                    <strong>{bucket.dataClass}</strong>
                  </div>
              {bucket.mode === "safe" ? (
                <div className="button-row">
                  <button
                    disabled={busyAction !== null}
                    onClick={() => void runAction(bucket.id, () => clearWorkbenchStorageBucket(bucket.id))}
                    type="button"
                  >
                    {busyAction === bucket.id ? copy.running : copy.workflowPackageInstallRulesRepairItemLabel}
                  </button>
                </div>
              ) : null}
            </div>
          ))}
        </div>
      ) : (
        <p className="card-copy" style={{ marginTop: "0.75rem" }}>
          {copy.workflowPackageInstallRulesResidualsCleanLabel}
        </p>
      )}

      <div className="button-row" style={{ marginTop: "0.75rem" }}>
        <button disabled={busyAction !== null} onClick={() => void refresh()} type="button">
          {copy.refresh}
        </button>
        <button
          disabled={busyAction !== null}
          onClick={() => void runAction("safe_cleanup", clearWorkbenchSafeStorage)}
          type="button"
        >
          {busyAction === "safe_cleanup" ? copy.running : copy.workflowPackageInstallRulesRepairLabel}
        </button>
      </div>
      <p className="card-copy" style={{ marginTop: "0.75rem" }}>
        {copy.settingsInstallPolicyUpdateValue}
      </p>
        </>
      ) : null}
      {page === "details" ? (
        <div style={{ display: "grid", gap: "0.75rem" }}>
          <div className="sidebar-list sidebar-list--metrics">
            <div className="sidebar-list__row">
              <span>{copy.workflowPackageInstallRulesStorageScopeLabel}</span>
              <strong>browser.localStorage</strong>
            </div>
            <div className="sidebar-list__row">
              <span>{copy.workflowPackageInstallRulesCleanupLabel}</span>
              <strong>{copy.workflowPackageInstallRulesReadonlyLabel}</strong>
            </div>
            <div className="sidebar-list__row">
              <span>{copy.workflowPackageInstallRulesReadonlyLabel}</span>
              <strong>{copy.settingsInstallPolicyIntegrityValue}</strong>
            </div>
            <div className="sidebar-list__row">
              <span>{copy.workflowPackageInstallRulesResidualsLabel}</span>
              <strong>{snapshot ? `${snapshot.unknownKeys} / ${formatBytes(snapshot.unknownBytes)}` : "--"}</strong>
            </div>
          </div>
          <div style={{ display: "grid", gap: "0.65rem" }}>
            {storageRules.map((rule) => {
              const usage = snapshot?.buckets.find((bucket) => bucket.id === rule.id);
              return (
                <div key={rule.id} style={{ display: "grid", gap: "0.25rem" }}>
                  <div className="sidebar-list__row">
                    <span>{storageBucketLabel(rule.id, copy)}</span>
                    <strong>{formatBytes(usage?.bytes ?? 0)}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>{copy.settingsInstallPolicyTitle}</span>
                    <strong>{rule.mode === "safe" ? copy.workflowPackageInstallRulesAutoLabel : copy.workflowPackageInstallRulesManualLabel}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>{copy.access}</span>
                    <strong>{rule.authority}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>{copy.data}</span>
                    <strong>{rule.dataClass}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>{copy.workflowPackageInstallRulesPortabilityLabel}</span>
                    <strong>{rule.portable ? copy.yes : copy.no}</strong>
                  </div>
                  {rule.keyPrefixes.map((prefix) => (
                    <span className="card-copy" key={`${rule.id}-${prefix}`}>{prefix}</span>
                  ))}
                  {rule.mode === "safe" ? (
                    <div className="button-row">
                      <button
                        disabled={busyAction !== null}
                        onClick={() => void runAction(rule.id, () => clearWorkbenchStorageBucket(rule.id))}
                        type="button"
                      >
                        {busyAction === rule.id ? copy.running : copy.workflowPackageInstallRulesRepairItemLabel}
                      </button>
                    </div>
                  ) : null}
                </div>
              );
            })}
          </div>
        </div>
      ) : null}
    </WorkbenchSystemOverviewCard>
  );
}

function storageBucketLabel(bucketId: string, copy: WorkbenchCopy) {
  const labels: Record<string, string> = {
    workflow_snapshots: copy.workflowPackageInstallRulesSnapshotLabel,
    workflow_drafts: copy.workflowSavedDraftsTitle,
    runtime_temp: copy.runtime,
    local_workflows: copy.workflowLocalWorkflowBadgeLabel,
    workflow_template_library: copy.workflowTemplateChainLibraryLabel,
    workspace_store_manifests: copy.rail.store,
    script_presets: copy.scripts,
    workflow_favorites: copy.workflowTemplateChainPinnedLabel,
    settings: copy.settings,
  };
  return labels[bucketId] ?? bucketId;
}
