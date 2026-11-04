"use client";

import type { AssetStoreEntry } from "@/lib/api";
import { storeWorkspaceCopy } from "./workbench-store-workspace";

export function StoreEntryDetails({ entry, language, onBack, onStage, disabled, actionLabel }: {
  entry: AssetStoreEntry; language: string; onBack: () => void; onStage: () => void;
  disabled: boolean; actionLabel: string;
}) {
  const copy = storeWorkspaceCopy(language);
  const rows = [
    ["ID", entry.id], [copy.source, entry.source_id], [copy.version, entry.version],
    [copy.domain, entry.domain], [copy.reference, entry.package_ref], [copy.target, entry.install?.target],
  ];
  return <section className="sidebar-card store-detail" data-workbench-store-detail={entry.id}>
    <div className="button-row">
      <button type="button" className="ghost-button ghost-button--compact" data-workbench-store-detail-action="back" onClick={onBack}>{copy.back}</button>
    </div>
    <p className="eyebrow">{entry.kind}</p>
    <h2>{entry.title}</h2>
    <p className="card-copy">{entry.summary || entry.id}</p>
    <dl className="store-detail-facts">{rows.map(([label, value]) =>
      <div key={label}><dt>{label}</dt><dd>{value || copy.unknown}</dd></div>)}</dl>
    <div className="store-tags">{(entry.tags || []).map((tag, index) => <span className="status-pill" key={`${index}:${tag}`}>{tag}</span>)}</div>
    <p className="card-copy">{entry.install?.requires_download === false && entry.source_kind === "builtin" ? copy.builtin : copy.downloadPending}</p>
    <p className="warning-copy">{copy.trust}</p>
    <p className="card-copy">{copy.stageOnly}</p>
    <button type="button" className="ghost-button" data-workbench-store-detail-action="stage" disabled={disabled} onClick={onStage}>{actionLabel}</button>
  </section>;
}
