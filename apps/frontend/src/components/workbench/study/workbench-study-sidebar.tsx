"use client";

import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import type { StudyDomainOption, StudyKindOptionGroup } from "@/lib/workbench/view-models";
import type { StudyKind } from "@/components/workbench/workbench-types";

type StudyPanelTab = "summary" | "controls";
type ControlsPage = "setup" | "review";

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
  studyDomainLabel: string;
  studyDomainOptions: StudyDomainOption[];
  noDomainStudiesLabel: string;
  studyKindOptionGroups: StudyKindOptionGroup[];
  onStudyKindChange: (kind: StudyKind) => void;
  summaryRows: StudySidebarRow[];
  controlsRows: StudySidebarRow[];
  controlsContent?: ReactNode;
  controlsTitle: string;
  controlsSetupPageLabel: string;
  controlsReviewPageLabel: string;
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
  studyDomainLabel,
  studyDomainOptions,
  noDomainStudiesLabel,
  studyKindOptionGroups,
  onStudyKindChange,
  summaryRows,
  controlsRows,
  controlsContent,
  controlsTitle,
  controlsSetupPageLabel,
  controlsReviewPageLabel,
  readyLabel,
  busyLabel,
  isPending,
  runLabel,
  runningLabel,
  onRun,
}: WorkbenchStudySidebarProps) {
  const [selectedDomain, setSelectedDomain] = useState<string>("mechanical");
  const [controlsPage, setControlsPage] = useState<ControlsPage>("setup");
  useEffect(() => {
    const matchingGroup = studyKindOptionGroups.find((group) => group.options.some((option) => option.value === studyKind));
    if (matchingGroup && matchingGroup.domainKey !== selectedDomain) {
      setSelectedDomain(matchingGroup.domainKey);
    }
  }, [selectedDomain, studyKind, studyKindOptionGroups]);
  const visibleStudyKindOptionGroups = useMemo(
    () => studyKindOptionGroups.filter((group) => group.domainKey === selectedDomain && group.options.length > 0),
    [selectedDomain, studyKindOptionGroups],
  );

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
              <span>{studyDomainLabel}</span>
              <div className="button-row">
                {studyDomainOptions.map((option) => (
                  <button
                    key={option.key}
                    className={`ghost-button ghost-button--compact${selectedDomain === option.key ? " ghost-button--active" : ""}`}
                    onClick={() => setSelectedDomain(option.key)}
                    type="button"
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </label>
            <label>
              <span>{studyTypeLabel}</span>
              <select value={studyKind} onChange={(event) => onStudyKindChange(event.target.value as StudyKind)}>
                {visibleStudyKindOptionGroups.map((group) => (
                  <optgroup key={group.label} label={group.label}>
                    {group.options.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </select>
              {visibleStudyKindOptionGroups.length === 0 ? <small>{noDomainStudiesLabel}</small> : null}
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
          <div className="panel-tabs panel-tabs--wide">
            <button
              className={`panel-tab${controlsPage === "setup" ? " panel-tab--active" : ""}`}
              onClick={() => setControlsPage("setup")}
              type="button"
            >
              {controlsSetupPageLabel}
            </button>
            <button
              className={`panel-tab${controlsPage === "review" ? " panel-tab--active" : ""}`}
              onClick={() => setControlsPage("review")}
              type="button"
            >
              {controlsReviewPageLabel}
            </button>
          </div>
          {controlsPage === "setup" ? (controlsContent ? controlsContent : null) : null}
          {controlsPage === "review" && controlsRows.length > 0 ? (
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
