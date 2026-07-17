"use client";

import { MATERIAL_PRESETS } from "@/lib/materials";
import type { ModelMaterial } from "@/lib/api";
import { getWorkbenchMaterialLibraryCopy } from "@/components/workbench/workbench-extended-language-copy";

type WorkbenchMaterialLibraryCardProps = {
  language: string;
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
  return (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{copy.title}</h2>
        <span>{currentMaterials.length}</span>
      </div>
      <div className="button-row">
        <select value={activeMaterial} onChange={(event) => onActiveMaterialChange(event.target.value)}>
          {MATERIAL_PRESETS.map((preset) => (
            <option key={preset.value} value={preset.value}>
              {localMaterialLabel(preset.value, language)}
            </option>
          ))}
        </select>
        <button className="ghost-button" onClick={onAddMaterial} type="button">
          {copy.addMaterial}
        </button>
        <button className="ghost-button" onClick={onAddCustomMaterial} type="button">
          {copy.newCustom}
        </button>
      </div>
      <label className="import-box">
        <span>{copy.importMaterials}</span>
        <small>{copy.importHint}</small>
        <input
          type="file"
          accept=".json,.csv,text/csv,application/json"
          onChange={(event) => onImportMaterials(event.target.files?.[0])}
        />
      </label>
      <div className="material-library">
        {currentMaterials.map((material) => (
          <div key={material.id} className="material-chip-card">
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
                <input value={material.name} onChange={(event) => onUpdateMaterial(material.id, "name", event.target.value)} />
              </label>
              <label>
                <span>{modulusLabel}</span>
                <input
                  type="number"
                  min={0.1}
                  step={0.1}
                  value={round(material.youngs_modulus / 1.0e9)}
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
            <div className="button-row">
              <button
                className={`ghost-button ghost-button--compact${hiddenMaterialIds.includes(material.id) ? "" : " ghost-button--active"}`}
                onClick={() => onToggleMaterialVisibility(material.id)}
                type="button"
              >
                {hiddenMaterialIds.includes(material.id)
                  ? copy.show
                  : copy.hide}
              </button>
            </div>
            <div className="button-row">
              <button
                className="ghost-button ghost-button--compact"
                disabled={selectedElement === null}
                onClick={() => onApplyMaterial(material.id, "selected")}
                type="button"
              >
                {copy.applySelected}
              </button>
              <button className="ghost-button ghost-button--compact" onClick={() => onApplyMaterial(material.id, "all")} type="button">
                {copy.applyAll}
              </button>
              <button
                className="ghost-button ghost-button--compact"
                disabled={currentMaterials.length <= 1}
                onClick={() => onDeleteMaterial(material.id)}
                type="button"
              >
                {copy.deleteMaterial}
              </button>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
