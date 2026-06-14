"use client";

import { useEffect, useState } from "react";

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

export function WorkbenchSystemStorageCard() {
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
    <section className="sidebar-card sidebar-card--compact runtime-overview-card">
      <div className="card-head">
        <h2>Disk usage</h2>
        <span>{formatBytes(snapshot?.totalBytes ?? null)}</span>
      </div>
      <div className="panel-tabs">
        <button
          className={`panel-tab${page === "overview" ? " panel-tab--active" : ""}`}
          onClick={() => setPage("overview")}
          type="button"
        >
          Overview
        </button>
        <button
          className={`panel-tab${page === "details" ? " panel-tab--active" : ""}`}
          onClick={() => setPage("details")}
          type="button"
        >
          Details
        </button>
      </div>
      {page === "overview" ? (
        <>
      <div className="sidebar-list sidebar-list--metrics">
        <div className="sidebar-list__row">
          <span>browser storage</span>
          <strong>{formatBytes(snapshot?.usageBytes ?? null)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>quota</span>
          <strong>{formatBytes(snapshot?.quotaBytes ?? null)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>quota usage</span>
          <strong>{formatPercent(snapshot?.usageBytes ?? null, snapshot?.quotaBytes ?? null)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>local storage keys</span>
          <strong>{snapshot?.localStorageKeys ?? "--"}</strong>
        </div>
      </div>

      {largestBuckets.length > 0 ? (
        <div style={{ display: "grid", gap: "0.45rem", marginTop: "0.75rem" }}>
          {largestBuckets.map((bucket) => (
            <div key={bucket.id} style={{ display: "grid", gap: "0.2rem" }}>
              <div className="sidebar-list__row">
                <span>{bucket.label}</span>
                <strong>{formatBytes(bucket.bytes)}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>entries</span>
                <strong>{bucket.entries}</strong>
              </div>
              {bucket.mode === "safe" ? (
                <div className="button-row">
                  <button
                    disabled={busyAction !== null}
                    onClick={() => void runAction(bucket.id, () => clearWorkbenchStorageBucket(bucket.id))}
                    type="button"
                  >
                    {busyAction === bucket.id ? "Cleaning..." : `Clear ${bucket.label}`}
                  </button>
                </div>
              ) : null}
            </div>
          ))}
        </div>
      ) : (
        <p className="card-copy" style={{ marginTop: "0.75rem" }}>
          No measurable workbench storage buckets are active yet.
        </p>
      )}

      <div className="button-row" style={{ marginTop: "0.75rem" }}>
        <button disabled={busyAction !== null} onClick={() => void refresh()} type="button">
          Refresh
        </button>
        <button
          disabled={busyAction !== null}
          onClick={() => void runAction("safe_cleanup", clearWorkbenchSafeStorage)}
          type="button"
        >
          {busyAction === "safe_cleanup" ? "Cleaning..." : "Clean safe caches"}
        </button>
      </div>
      <p className="card-copy" style={{ marginTop: "0.75rem" }}>
        Safe cleanup only removes snapshots, drafts, and temporary runtime cache. Local workflow assets and presets stay untouched.
      </p>
        </>
      ) : null}
      {page === "details" ? (
        <div style={{ display: "grid", gap: "0.75rem" }}>
          <div className="sidebar-list sidebar-list--metrics">
            <div className="sidebar-list__row">
              <span>storage scope</span>
              <strong>browser localStorage / per-user workspace profile</strong>
            </div>
            <div className="sidebar-list__row">
              <span>cleanup mode</span>
              <strong>visible, selective, policy-bound</strong>
            </div>
            <div className="sidebar-list__row">
              <span>read-only rules</span>
              <strong>careful buckets are inspect-first</strong>
            </div>
          </div>
          <div style={{ display: "grid", gap: "0.65rem" }}>
            {storageRules.map((rule) => {
              const usage = snapshot?.buckets.find((bucket) => bucket.id === rule.id);
              return (
                <div key={rule.id} style={{ display: "grid", gap: "0.25rem" }}>
                  <div className="sidebar-list__row">
                    <span>{rule.label}</span>
                    <strong>{formatBytes(usage?.bytes ?? 0)}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>policy</span>
                    <strong>{rule.cleanupLabel}</strong>
                  </div>
                  <span className="card-copy">{rule.detail}</span>
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
                        {busyAction === rule.id ? "Cleaning..." : `Clear ${rule.label}`}
                      </button>
                    </div>
                  ) : null}
                </div>
              );
            })}
          </div>
        </div>
      ) : null}
    </section>
  );
}
