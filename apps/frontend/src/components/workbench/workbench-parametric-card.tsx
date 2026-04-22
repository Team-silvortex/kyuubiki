"use client";

import type { ParametricPanelConfig, ParametricTrussConfig } from "@/lib/models";

type WorkbenchParametricCardProps = {
  isPlane: boolean;
  title: string;
  subtitle: string;
  lengthLabel: string;
  heightLabel: string;
  divisionsXLabel: string;
  divisionsYLabel: string;
  thicknessLabel: string;
  modulusLabel: string;
  poissonRatioLabel: string;
  loadCaseLabel: string;
  baysLabel: string;
  areaLabel: string;
  generateLabel: string;
  panelParametric: ParametricPanelConfig;
  parametric: ParametricTrussConfig;
  onPanelParametricChange: (key: keyof ParametricPanelConfig, value: number) => void;
  onParametricChange: (key: keyof ParametricTrussConfig, value: number) => void;
  onGenerate: () => void;
};

export function WorkbenchParametricCard({
  isPlane,
  title,
  subtitle,
  lengthLabel,
  heightLabel,
  divisionsXLabel,
  divisionsYLabel,
  thicknessLabel,
  modulusLabel,
  poissonRatioLabel,
  loadCaseLabel,
  baysLabel,
  areaLabel,
  generateLabel,
  panelParametric,
  parametric,
  onPanelParametricChange,
  onParametricChange,
  onGenerate,
}: WorkbenchParametricCardProps) {
  return (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{subtitle}</span>
      </div>
      <div className="form-grid compact">
        {isPlane ? (
          <>
            <label>
              <span>{lengthLabel}</span>
              <input type="number" min={0.2} step={0.1} value={panelParametric.width} onChange={(event) => onPanelParametricChange("width", Number(event.target.value))} />
            </label>
            <label>
              <span>{heightLabel}</span>
              <input type="number" min={0.2} step={0.1} value={panelParametric.height} onChange={(event) => onPanelParametricChange("height", Number(event.target.value))} />
            </label>
            <label>
              <span>{divisionsXLabel}</span>
              <input type="number" min={1} max={12} step={1} value={panelParametric.divisionsX} onChange={(event) => onPanelParametricChange("divisionsX", Number(event.target.value))} />
            </label>
            <label>
              <span>{divisionsYLabel}</span>
              <input type="number" min={1} max={12} step={1} value={panelParametric.divisionsY} onChange={(event) => onPanelParametricChange("divisionsY", Number(event.target.value))} />
            </label>
            <label>
              <span>{thicknessLabel}</span>
              <input type="number" min={0.001} step={0.001} value={panelParametric.thickness} onChange={(event) => onPanelParametricChange("thickness", Number(event.target.value))} />
            </label>
            <label>
              <span>{modulusLabel}</span>
              <input type="number" min={0.1} step={0.1} value={panelParametric.youngsModulusGpa} onChange={(event) => onPanelParametricChange("youngsModulusGpa", Number(event.target.value))} />
            </label>
            <label>
              <span>{poissonRatioLabel}</span>
              <input type="number" min={0.01} max={0.49} step={0.01} value={panelParametric.poissonRatio} onChange={(event) => onPanelParametricChange("poissonRatio", Number(event.target.value))} />
            </label>
            <label>
              <span>{loadCaseLabel}</span>
              <input type="number" step={100} value={panelParametric.loadY} onChange={(event) => onPanelParametricChange("loadY", Number(event.target.value))} />
            </label>
          </>
        ) : (
          <>
            <label>
              <span>{baysLabel}</span>
              <input type="number" min={2} max={10} step={1} value={parametric.bays} onChange={(event) => onParametricChange("bays", Number(event.target.value))} />
            </label>
            <label>
              <span>{lengthLabel}</span>
              <input type="number" min={1} step={0.5} value={parametric.span} onChange={(event) => onParametricChange("span", Number(event.target.value))} />
            </label>
            <label>
              <span>{heightLabel}</span>
              <input type="number" min={0.2} step={0.1} value={parametric.height} onChange={(event) => onParametricChange("height", Number(event.target.value))} />
            </label>
            <label>
              <span>{areaLabel}</span>
              <input type="number" min={0.0001} step={0.0001} value={parametric.area} onChange={(event) => onParametricChange("area", Number(event.target.value))} />
            </label>
            <label>
              <span>{modulusLabel}</span>
              <input type="number" min={0.1} step={0.1} value={parametric.youngsModulusGpa} onChange={(event) => onParametricChange("youngsModulusGpa", Number(event.target.value))} />
            </label>
            <label>
              <span>{loadCaseLabel}</span>
              <input type="number" step={100} value={parametric.loadY} onChange={(event) => onParametricChange("loadY", Number(event.target.value))} />
            </label>
          </>
        )}
      </div>
      <button className="solve-button" onClick={onGenerate} type="button">
        {generateLabel}
      </button>
    </section>
  );
}
