"use client";

import { useEffect, useRef, useState } from "react";
import { MATERIAL_PRESETS } from "@/lib/materials";
import type { ModelMaterial } from "@/lib/api";
import { nextMaterialId } from "@/lib/workbench/material-commands";
import { getWorkbenchMaterialLibraryCopy } from "@/components/workbench/workbench-extended-language-copy";
import { getWorkbenchCompactPanelCopy } from "../workbench-compact-panel-copy";
import { WorkbenchListPager, WorkbenchSubpanelNav } from "../workbench-compact-panels";
import type { WorkbenchCopy } from "../workbench-copy";

type WorkbenchMaterialLibraryCardProps = {
  language: string;
  ui: Pick<WorkbenchCopy, "adminBrowsePage" | "adminEditPage" | "selectRecord" | "previousPage" | "nextPage">;
  materialLabel: string;
  modulusLabel: string;
  poissonRatioLabel: string;
  activeMaterial: string;
  currentMaterials: ModelMaterial[];
  hiddenMaterialIds: string[];
  isPlane: boolean;
  selectedElement: number | null;
  localMaterialLabel: (value: string, language: string) => string;
  getMaterialColor: (materialId: string) => string;
  onActiveMaterialChange: (materialId: string) => void;
  onAddMaterial: () => void;
  onAddCustomMaterial: () => void;
  onImportMaterials: (file: File | undefined) => void;
  onUpdateMaterial: (
    materialId: string,
    field: "name" | "youngs_modulus" | "poisson_ratio",
    value: string | number,
  ) => void;
  onToggleMaterialVisibility: (materialId: string) => void;
  onApplyMaterial: (materialId: string, mode: "selected" | "all") => void;
  onDeleteMaterial: (materialId: string) => void;
  round: (value: number) => number;
};

export function WorkbenchMaterialLibraryCard({
  language,
  ui,
  materialLabel,
  modulusLabel,
  poissonRatioLabel,
  activeMaterial,
  currentMaterials,
  hiddenMaterialIds,
  isPlane,
  selectedElement,
  localMaterialLabel,
  getMaterialColor,
  onActiveMaterialChange,
  onAddMaterial,
  onAddCustomMaterial,
  onImportMaterials,
  onUpdateMaterial,
  onToggleMaterialVisibility,
  onApplyMaterial,
  onDeleteMaterial,
  round,
}: WorkbenchMaterialLibraryCardProps) {
  const copy = getWorkbenchMaterialLibraryCopy(language);
  const compact = getWorkbenchCompactPanelCopy(language);
  const [page, setPage] = useState("browse");
  const [query, setQuery] = useState("");
  const [listPage, setListPage] = useState(0);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const root = useRef<HTMLElement>(null);
  const material = currentMaterials.find((entry) => entry.id === selectedId);
  useEffect(() => {
    // Restore focus only when the row or form that opened this page was unmounted.
    if (document.activeElement !== document.body) return;
    root.current?.querySelector<HTMLInputElement>(
      page === "edit" ? '[data-workbench-material-control="name"]' : '[data-workbench-material-control="search"]',
    )?.focus();
  }, [page, material?.id]);
  const needle = query.trim().toLocaleLowerCase();
  const matches = currentMaterials.filter((entry) => `${entry.id}\n${entry.name}`.toLocaleLowerCase().includes(needle));
  const pages = Math.max(1, Math.ceil(matches.length / 8));
  const visiblePage = Math.min(listPage, pages - 1);
  const addMaterial = (callback: () => void) => {
    // Both existing add commands use this same ID allocator; selection never applies a material.
    const id = nextMaterialId(currentMaterials);
    callback();
    setSelectedId(id);
    setPage("edit");
  };
  return (
    <section ref={root} className="sidebar-card" data-workbench-materials="panel">
      <div className="card-head">
        <h2>{copy.title}</h2>
        <span>{currentMaterials.length}</span>
      </div>
      <WorkbenchSubpanelNav label={copy.title} value={page} onChange={setPage} attribute="data-workbench-material-page"
        pages={[{ id: "browse", label: ui.adminBrowsePage }, { id: "edit", label: ui.adminEditPage, disabled: !material },
          { id: "add", label: copy.addMaterial }]} />
      {page === "browse" ? <div data-workbench-material-content="browse">
        <label className="material-browser-search"><span>{compact.search}</span>
          <input type="search" value={query} data-workbench-material-control="search"
            onChange={(event) => { setQuery(event.target.value); setListPage(0); }} />
        </label>
        <p className="card-copy" aria-live="polite">{matches.length} / {currentMaterials.length}</p>
        <WorkbenchListPager page={visiblePage} pages={pages} previousLabel={ui.previousPage} nextLabel={ui.nextPage} onChange={setListPage} />
        <div className="material-browser-list">
          {matches.slice(visiblePage * 8, (visiblePage + 1) * 8).map((entry) => <button key={entry.id} type="button"
            className="ghost-button material-browser-item" data-workbench-material-id={entry.id}
            onClick={() => { setSelectedId(entry.id); setPage("edit"); }}>
            <span className="material-chip-card__swatch" style={{ background: getMaterialColor(entry.id) }} aria-hidden="true" />
            <span className="material-browser-item__text"><strong>{entry.name || entry.id}</strong>
              <small>{entry.id} · {round(entry.youngs_modulus / 1.0e9)} GPa{hiddenMaterialIds.includes(entry.id) ? ` · ${copy.hide}` : ""}</small>
            </span>
          </button>)}
        </div>
        {!matches.length ? <p className="card-copy">{compact.noMatches}</p> : null}
      </div> : null}
      {page === "add" ? <div data-workbench-material-content="add">
      <div className="button-row">
        <select aria-label={materialLabel} value={activeMaterial} onChange={(event) => onActiveMaterialChange(event.target.value)}>
          {MATERIAL_PRESETS.map((preset) => (
            <option key={preset.value} value={preset.value}>
              {localMaterialLabel(preset.value, language)}
            </option>
          ))}
        </select>
        <button className="ghost-button" data-workbench-material-control="add-preset" onClick={() => addMaterial(onAddMaterial)} type="button">
          {copy.addMaterial}
        </button>
        <button className="ghost-button" data-workbench-material-control="add-custom" onClick={() => addMaterial(onAddCustomMaterial)} type="button">
          {copy.newCustom}
        </button>
      </div>
      <label className="import-box">
        <span>{copy.importMaterials}</span>
        <small>{copy.importHint}</small>
        <input
          type="file"
          accept=".json,.csv,text/csv,application/json"
          data-workbench-material-control="import"
          onChange={(event) => {
            const file = event.target.files?.[0];
            event.currentTarget.value = "";
            if (!file) return;
            onImportMaterials(file);
            setQuery(""); setListPage(0); setPage("browse");
          }}
        />
      </label>
      </div> : null}
      {page === "edit" && !material ? <p className="card-copy">{ui.selectRecord}</p> : null}
      {page === "edit" && material ? (
          <div key={material.id} className="material-chip-card" data-workbench-material-content="edit" data-workbench-material-editor={material.id}>
            <div className="material-chip-card__head">
              <span
                className="material-chip-card__swatch"
                style={{ background: getMaterialColor(material.id) }}
              />
              <strong>{material.id}</strong>
            </div>
            <div className="form-grid compact">
              <label>
                <span>{materialLabel}</span>
                <input data-workbench-material-control="name" value={material.name} onChange={(event) => onUpdateMaterial(material.id, "name", event.target.value)} />
              </label>
              <label>
                <span>{modulusLabel}</span>
                <input
                  type="number"
                  min={0.1}
                  step={0.1}
                  value={round(material.youngs_modulus / 1.0e9)}
                  data-workbench-material-control="modulus"
                  onChange={(event) => onUpdateMaterial(material.id, "youngs_modulus", Number(event.target.value) * 1.0e9)}
                />
              </label>
              {isPlane ? (
                <label>
                  <span>{poissonRatioLabel}</span>
                  <input
                    type="number"
                    min={0.01}
                    max={0.49}
                    step={0.01}
                    value={material.poisson_ratio ?? 0.33}
                    onChange={(event) => onUpdateMaterial(material.id, "poisson_ratio", Number(event.target.value))}
                  />
                </label>
              ) : null}
            </div>
            <div className="material-editor-actions">
              <button
                className={`ghost-button ghost-button--compact${hiddenMaterialIds.includes(material.id) ? "" : " ghost-button--active"}`}
                onClick={() => onToggleMaterialVisibility(material.id)}
                data-workbench-material-control="visibility"
                type="button"
              >
                {hiddenMaterialIds.includes(material.id)
                  ? copy.show
                  : copy.hide}
              </button>
              <button
                className="ghost-button ghost-button--compact"
                disabled={selectedElement === null}
                onClick={() => onApplyMaterial(material.id, "selected")}
                data-workbench-material-control="apply-selected"
                type="button"
              >
                {copy.applySelected}
              </button>
              <button className="ghost-button ghost-button--compact" data-workbench-material-control="apply-all" onClick={() => onApplyMaterial(material.id, "all")} type="button">
                {copy.applyAll}
              </button>
              <button
                className="ghost-button ghost-button--compact"
                disabled={currentMaterials.length <= 1}
                data-workbench-material-control="delete"
                onClick={() => { onDeleteMaterial(material.id); setSelectedId(null); setListPage(0); setPage("browse"); }}
                type="button"
              >
                {copy.deleteMaterial}
              </button>
            </div>
          </div>
      ) : null}
    </section>
  );
}
