"use client";

import type { ReactNode } from "react";

type StudyKind = "axial_bar_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d";
type StudyPanelTab = "summary" | "controls";

type StudySidebarRow = {
  label: string;
  value: string | number;
};

type WorkbenchStudySidebarProps = {
  studyTab: StudyPanelTab;
  onStudyTabChange: (tab: StudyPanelTab) => void;
  sectionTitle: string;
  summaryTabLabel: string;
  controlsTabLabel: string;
  loadedModelName: string;
  studyTypeLabel: string;
  studyKind: StudyKind;
  studyKindOptions: Array<{ value: StudyKind; label: string }>;
  onStudyKindChange: (kind: StudyKind) => void;
  summaryRows: StudySidebarRow[];
  controlsRows: StudySidebarRow[];
  controlsContent?: ReactNode;
  controlsTitle: string;
  readyLabel: string;
  busyLabel: string;
  isPending: boolean;
  runLabel: string;
  runningLabel: string;
  onRun: () => void;
};

export function WorkbenchStudySidebar({
  studyTab,
  onStudyTabChange,
  sectionTitle,
  summaryTabLabel,
  controlsTabLabel,
  loadedModelName,
  studyTypeLabel,
  studyKind,
  studyKindOptions,
  onStudyKindChange,
  summaryRows,
  controlsRows,
  controlsContent,
  controlsTitle,
  readyLabel,
  busyLabel,
  isPending,
  runLabel,
  runningLabel,
  onRun,
}: WorkbenchStudySidebarProps) {
  return (
    <div className="sidebar-stack panel-scroll-window">
      <div className="panel-tabs">
        <button
          className={`panel-tab${studyTab === "summary" ? " panel-tab--active" : ""}`}
          onClick={() => onStudyTabChange("summary")}
          type="button"
        >
          {summaryTabLabel}
        </button>
        <button
          className={`panel-tab${studyTab === "controls" ? " panel-tab--active" : ""}`}
          onClick={() => onStudyTabChange("controls")}
          type="button"
        >
          {controlsTabLabel}
        </button>
      </div>

      {studyTab === "summary" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{sectionTitle}</h2>
            <span>{loadedModelName}</span>
          </div>
          <div className="form-grid compact">
            <label>
              <span>{studyTypeLabel}</span>
              <select value={studyKind} onChange={(event) => onStudyKindChange(event.target.value as StudyKind)}>
                {studyKindOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="sidebar-list">
            {summaryRows.map((row) => (
              <div key={row.label}>
                <span>{row.label}</span>
                <strong>{row.value}</strong>
              </div>
            ))}
          </div>
        </section>
      ) : null}

      {studyTab === "controls" ? (
        <section className="sidebar-card">
          <div className="card-head">
            <h2>{controlsTitle}</h2>
            <span>{isPending ? busyLabel : readyLabel}</span>
          </div>
          {controlsContent ? controlsContent : null}
          {controlsRows.length > 0 ? (
            <div className="sidebar-list">
              {controlsRows.map((row) => (
                <div key={row.label}>
                  <span>{row.label}</span>
                  <strong>{row.value}</strong>
                </div>
              ))}
            </div>
          ) : null}
          <button className="solve-button" disabled={isPending} onClick={onRun} type="button">
            {isPending ? runningLabel : runLabel}
          </button>
        </section>
      ) : null}
    </div>
  );
}
